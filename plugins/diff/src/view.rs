use crate::git::Snapshot;
use saddle_plugin_sdk::ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};
use similar::{ChangeTag, TextDiff};
use std::{ops::Range, path::Path, sync::OnceLock, time::Duration};
use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Debug)]
pub struct TextLine {
    pub number: usize,
    pub text: String,
    pub spans: Vec<(String, Style)>,
}
#[derive(Clone, Debug)]
pub struct Row {
    pub file: usize,
    pub note: String,
    pub left: Option<TextLine>,
    pub right: Option<TextLine>,
    pub changed: bool,
    pub header: bool,
    pub hunk: bool,
}
impl Row {
    fn note(file: usize, note: String, header: bool, hunk: bool) -> Self {
        Self {
            file,
            note,
            left: None,
            right: None,
            changed: false,
            header,
            hunk,
        }
    }
    fn key(&self) -> (&str, Option<&str>, Option<&str>) {
        (
            &self.note,
            self.left.as_ref().map(|l| l.text.as_str()),
            self.right.as_ref().map(|l| l.text.as_str()),
        )
    }
}
#[derive(Default)]
pub struct Document {
    pub unified: Vec<Row>,
    pub split: Vec<Row>,
}

fn syntax() -> &'static (SyntaxSet, ThemeSet) {
    static SETS: OnceLock<(SyntaxSet, ThemeSet)> = OnceLock::new();
    SETS.get_or_init(|| {
        (
            SyntaxSet::load_defaults_newlines(),
            ThemeSet::load_defaults(),
        )
    })
}
/// Syntax colors always come from this dark theme, whatever the host theme is.
const THEME: &str = "base16-ocean.dark";
fn linear(c: u8) -> f64 {
    let c = f64::from(c) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}
/// WCAG relative luminance; `None` when the terminal decides the color (Reset, ANSI, indexed).
fn luminance(color: Color) -> Option<f64> {
    let Color::Rgb(r, g, b) = color else {
        return None;
    };
    Some(0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b))
}
/// On a known light background (dark text reads better above about 0.18 luminance), the syntax
/// theme's plain foreground follows the host text color and other colors are darkened with their
/// hue kept: classes the syntax theme itself keeps under 4.5:1 (comments, variables) to 4.5:1,
/// the rest to 7:1, so dim classes stay lighter than the others. Dark or unknown backgrounds
/// keep the highlighted colors.
fn readable(style: Style, bg: Color, text: Color) -> Style {
    let (Some(fg), Some(light)) = (style.fg, luminance(bg)) else {
        return style;
    };
    let (Color::Rgb(r, g, b), Some(l)) = (fg, luminance(fg)) else {
        return style;
    };
    if light < 0.18 {
        return style;
    }
    let settings = &syntax().1.themes[THEME].settings;
    if settings
        .foreground
        .is_some_and(|c| (c.r, c.g, c.b) == (r, g, b))
    {
        return style.fg(text);
    }
    let dim = settings
        .background
        .and_then(|c| luminance(Color::Rgb(c.r, c.g, c.b)))
        .is_some_and(|source| (l + 0.05) / (source + 0.05) < 4.5);
    let target = (light + 0.05) / if dim { 4.5 } else { 7.0 } - 0.05;
    if l <= target {
        return style;
    }
    // Scaling the linear channels keeps the hue; rounding down only adds contrast.
    let darken = |c: u8| {
        let v = linear(c) * target / l;
        let v = if v <= 0.0031308 {
            v * 12.92
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };
        (v * 255.0).floor() as u8
    };
    style.fg(Color::Rgb(darken(r), darken(g), darken(b)))
}
/// Escape terminal controls and pathological graphemes before putting them into a panel frame.
pub fn safe(text: &str) -> String {
    let mut out = String::new();
    for g in text.graphemes(true) {
        if g == "\t" {
            out.push_str("    ");
        } else if g.len() > 128 || !(1..=2).contains(&g.width()) || g.chars().any(char::is_control)
        {
            for c in g.chars() {
                out.extend(c.escape_default());
            }
        } else {
            out.push_str(g);
        }
    }
    out
}
fn line(
    number: usize,
    text: &str,
    hl: &mut HighlightLines<'_>,
    ranges: &[Range<usize>],
) -> TextLine {
    let (ss, _) = syntax();
    let source = format!("{text}\n");
    let highlighted = hl.highlight_line(&source, ss).unwrap_or_else(|_| vec![]);
    let mut spans: Vec<(String, Style)> = Vec::new();
    let mut offset = 0;
    for (color, part) in highlighted {
        let style = Style::default().fg(Color::Rgb(
            color.foreground.r,
            color.foreground.g,
            color.foreground.b,
        ));
        for g in part.trim_end_matches('\n').graphemes(true) {
            let mut style = style;
            if ranges
                .iter()
                .any(|r| r.start < offset + g.len() && r.end > offset)
            {
                style = style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
            }
            let value = safe(g);
            if let Some((last, _)) = spans.last_mut().filter(|(_, s)| *s == style) {
                last.push_str(&value);
            } else {
                spans.push((value, style));
            }
            offset += g.len();
        }
    }
    if spans.is_empty() {
        spans.push((safe(text), Style::default()));
    }
    TextLine {
        number,
        text: text.into(),
        spans,
    }
}
fn emphasis(old: &str, new: &str) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    if old.len() + new.len() > 2048 {
        return (vec![], vec![]);
    }
    let diff = TextDiff::configure()
        .timeout(Duration::from_millis(3))
        .diff_unicode_words(old, new);
    let (mut a, mut b, mut x, mut y) = (vec![], vec![], 0, 0);
    for c in diff.iter_all_changes() {
        let n = c.value().len();
        match c.tag() {
            ChangeTag::Delete => {
                a.push(x..x + n);
                x += n;
            }
            ChangeTag::Insert => {
                b.push(y..y + n);
                y += n;
            }
            ChangeTag::Equal => {
                x += n;
                y += n;
            }
        }
    }
    (a, b)
}
impl Document {
    pub fn build(snapshot: &Snapshot) -> Self {
        let (ss, ts) = syntax();
        let theme = &ts.themes[THEME];
        let mut doc = Self::default();
        for (file, diff) in snapshot.files.iter().enumerate() {
            let header = Row::note(
                file,
                format!("{}  {}", diff.status, safe(&diff.path.to_string_lossy())),
                true,
                false,
            );
            doc.unified.push(header.clone());
            doc.split.push(header);
            let syntax = diff
                .path
                .extension()
                .and_then(|e| e.to_str())
                .and_then(|e| ss.find_syntax_by_extension(e))
                .unwrap_or_else(|| ss.find_syntax_plain_text());
            let (mut old_hl, mut new_hl) = (
                HighlightLines::new(syntax, theme),
                HighlightLines::new(syntax, theme),
            );
            let lines: Vec<_> = diff.patch.lines().collect();
            let (mut i, mut old, mut new, mut in_hunk) = (0, 0, 0, false);
            while i < lines.len() {
                let text = lines[i];
                if text.starts_with("@@ ") {
                    let mut fields = text.split_whitespace();
                    fields.next();
                    old = fields
                        .next()
                        .and_then(|s| s.trim_start_matches('-').split(',').next())
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    new = fields
                        .next()
                        .and_then(|s| s.trim_start_matches('+').split(',').next())
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    old_hl = HighlightLines::new(syntax, theme);
                    new_hl = HighlightLines::new(syntax, theme);
                    in_hunk = true;
                    let row = Row::note(file, safe(text), false, true);
                    doc.unified.push(row.clone());
                    doc.split.push(row);
                    i += 1;
                } else if in_hunk && (text.starts_with('-') || text.starts_with('+')) {
                    let (mut minus, mut plus) = (vec![], vec![]);
                    while i < lines.len() && lines[i].starts_with('-') {
                        minus.push(&lines[i][1..]);
                        i += 1;
                    }
                    while i < lines.len() && lines[i].starts_with('+') {
                        plus.push(&lines[i][1..]);
                        i += 1;
                    }
                    let (mut deletes, mut adds) = (vec![], vec![]);
                    for j in 0..minus.len().max(plus.len()) {
                        let (a, b) = match (minus.get(j), plus.get(j)) {
                            (Some(a), Some(b)) => emphasis(a, b),
                            _ => (vec![], vec![]),
                        };
                        let left = minus.get(j).map(|text| {
                            let l = line(old, text, &mut old_hl, &a);
                            old += 1;
                            l
                        });
                        let right = plus.get(j).map(|text| {
                            let l = line(new, text, &mut new_hl, &b);
                            new += 1;
                            l
                        });
                        let row = Row {
                            file,
                            note: String::new(),
                            left: left.clone(),
                            right: right.clone(),
                            changed: true,
                            header: false,
                            hunk: false,
                        };
                        if left.is_some() {
                            let mut r = row.clone();
                            r.right = None;
                            deletes.push(r);
                        }
                        if right.is_some() {
                            let mut r = row.clone();
                            r.left = None;
                            adds.push(r);
                        }
                        doc.split.push(row);
                    }
                    doc.unified.extend(deletes);
                    doc.unified.extend(adds);
                } else if in_hunk && text.starts_with(' ') {
                    let left = line(old, &text[1..], &mut old_hl, &[]);
                    let right = line(new, &text[1..], &mut new_hl, &[]);
                    old += 1;
                    new += 1;
                    let row = Row {
                        file,
                        note: String::new(),
                        left: Some(left),
                        right: Some(right),
                        changed: false,
                        header: false,
                        hunk: false,
                    };
                    doc.unified.push(row.clone());
                    doc.split.push(row);
                    i += 1;
                } else {
                    // Keep metadata (modes, rename, binary, missing newline, limits, errors) visible.
                    if !text.starts_with("diff --git ")
                        && !text.starts_with("index ")
                        && !text.starts_with("--- ")
                        && !text.starts_with("+++ ")
                    {
                        let row = Row::note(file, safe(text), false, false);
                        doc.unified.push(row.clone());
                        doc.split.push(row);
                    }
                    i += 1;
                }
            }
            doc.unified
                .push(Row::note(file, String::new(), false, false));
            doc.split.push(Row::note(file, String::new(), false, false));
        }
        doc
    }
    pub fn rows(&self, split: bool) -> &[Row] {
        if split { &self.split } else { &self.unified }
    }
}
/// Match the current content within the same file before falling back to its previous row offset.
pub fn restore(
    old_rows: &[Row],
    old_snapshot: &Snapshot,
    top: usize,
    rows: &[Row],
    snapshot: &Snapshot,
) -> usize {
    let Some(anchor) = old_rows.get(top) else {
        return 0;
    };
    let path = &old_snapshot.files[anchor.file].path;
    let Some(file) = snapshot.files.iter().position(|f| &f.path == path) else {
        return top.min(rows.len().saturating_sub(1));
    };
    if let Some(index) = rows
        .iter()
        .position(|r| r.file == file && r.key() == anchor.key())
    {
        return index;
    }
    let start = old_rows
        .iter()
        .position(|r| r.file == anchor.file)
        .unwrap_or(top);
    let indices: Vec<_> = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.file == file)
        .map(|(i, _)| i)
        .collect();
    indices
        .get((top - start).min(indices.len().saturating_sub(1)))
        .copied()
        .unwrap_or(0)
}

pub fn put(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    if width > 0 {
        buf.set_stringn(x, y, safe(text), usize::from(width), style);
    }
}
fn code(
    buf: &mut Buffer,
    area: Rect,
    line: Option<&TextLine>,
    mark: &str,
    bg: Color,
    (muted, text): (Color, Color),
    offset: usize,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    buf.set_style(area, Style::default().bg(bg));
    let Some(line) = line else {
        return;
    };
    let prefix = format!("{:>5}{} ", line.number, mark);
    put(
        buf,
        area.x,
        area.y,
        area.width.min(8),
        &prefix,
        Style::default().fg(muted).bg(bg),
    );
    let mut x = 0;
    let mut col = 0;
    for (part, style) in &line.spans {
        let style = readable(*style, bg, text).bg(bg);
        for g in part.graphemes(true) {
            let w = g.width();
            if col >= offset && x + w <= usize::from(area.width.saturating_sub(8)) {
                buf.set_string(area.x + 8 + x as u16, area.y, g, style);
                x += w;
            } else if col >= offset {
                return;
            }
            col += w;
        }
    }
}
/// `background`, `muted` and `text` are the theme roles received from the host; changed lines
/// keep their own diff colors.
pub fn draw_row(
    buf: &mut Buffer,
    area: Rect,
    row: &Row,
    split: bool,
    offset: usize,
    (background, muted, text): (Color, Color, Color),
) {
    if row.left.is_none() && row.right.is_none() {
        let style = if row.header {
            Style::default().add_modifier(Modifier::BOLD)
        } else if row.hunk {
            Style::default().fg(muted)
        } else {
            Style::default()
        };
        put(buf, area.x, area.y, area.width, &row.note, style);
        return;
    }
    let red = Color::Rgb(52, 28, 30);
    let green = Color::Rgb(23, 48, 35);
    if split {
        let half = area.width / 2;
        code(
            buf,
            Rect::new(area.x, area.y, half, 1),
            row.left.as_ref(),
            if row.changed { "−" } else { " " },
            if row.changed { red } else { background },
            (muted, text),
            offset,
        );
        code(
            buf,
            Rect::new(
                area.x + half + 1,
                area.y,
                area.width.saturating_sub(half + 1),
                1,
            ),
            row.right.as_ref(),
            if row.changed { "+" } else { " " },
            if row.changed { green } else { background },
            (muted, text),
            offset,
        );
        put(
            buf,
            area.x + half,
            area.y,
            1,
            "│",
            Style::default().fg(muted),
        );
    } else {
        let added = row.changed && row.right.is_some();
        let mark = if added {
            "+"
        } else if row.changed {
            "−"
        } else {
            " "
        };
        code(
            buf,
            area,
            row.right.as_ref().or(row.left.as_ref()),
            mark,
            if added {
                green
            } else if row.changed {
                red
            } else {
                background
            },
            (muted, text),
            offset,
        );
    }
}

pub fn display_path(path: &Path) -> String {
    safe(&path.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::FileDiff;
    fn snapshot() -> Snapshot {
        Snapshot {
            root: "/tmp".into(),
            files: vec![
                FileDiff {
                    path: "a.rs".into(),
                    status: "M".into(),
                    patch: "@@ -1,2 +1,2 @@\n-fn old() {}\n+fn 新() {}\n common\n".into(),
                },
                FileDiff {
                    path: "b.txt".into(),
                    status: "?".into(),
                    patch: "@@ -0,0 +1 @@\n+second\n".into(),
                },
            ],
        }
    }
    #[test]
    fn all_files_are_visible_and_split_pairs_changes() {
        let s = snapshot();
        let d = Document::build(&s);
        assert_eq!(d.unified.iter().filter(|r| r.header).count(), 2);
        assert_eq!(d.split.iter().filter(|r| r.header).count(), 2);
        let row = d.split.iter().find(|r| r.changed).unwrap();
        assert!(row.left.is_some() && row.right.is_some());
        assert!(
            row.right
                .as_ref()
                .unwrap()
                .spans
                .iter()
                .any(|(_, s)| s.add_modifier.contains(Modifier::UNDERLINED))
        );
    }
    #[test]
    fn frames_remain_valid_at_tiny_sizes_and_with_controls_and_wide_glyphs() {
        let mut s = snapshot();
        s.files[0].patch.push_str("+\t👩‍💻界\u{1b}[31m\r\n");
        let d = Document::build(&s);
        for width in 1..130 {
            for split in [false, true] {
                let rows = d.rows(split);
                let area = Rect::new(0, 0, width, rows.len() as u16);
                let mut b = Buffer::empty(area);
                for (i, row) in rows.iter().enumerate() {
                    draw_row(
                        &mut b,
                        Rect::new(0, i as u16, width, 1),
                        row,
                        split,
                        1,
                        (Color::Reset, Color::DarkGray, Color::Reset),
                    );
                }
                saddle_plugin_sdk::frame(&b, 1, 1).unwrap();
            }
        }
    }
    #[test]
    fn refresh_keeps_same_content_when_files_are_inserted_above() {
        let s = snapshot();
        let d = Document::build(&s);
        let top = d
            .unified
            .iter()
            .position(|r| r.right.as_ref().is_some_and(|l| l.text == "second"))
            .unwrap();
        let mut next = s.clone();
        next.files.insert(
            0,
            FileDiff {
                path: "0.txt".into(),
                status: "?".into(),
                patch: "@@ -0,0 +1 @@\n+above\n".into(),
            },
        );
        let nd = Document::build(&next);
        let restored = restore(&d.unified, &s, top, &nd.unified, &next);
        assert_eq!(nd.unified[restored].right.as_ref().unwrap().text, "second");
        assert!(restored > top);
    }
}
