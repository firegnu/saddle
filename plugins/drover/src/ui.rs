use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
pub(crate) fn pad(text: &str, width: usize) -> String {
    format!("{text}{}", " ".repeat(width.saturating_sub(text.width())))
}
pub(crate) fn clip(text: &str, width: usize) -> String {
    let clean: String = text.chars().filter(|c| !c.is_control()).collect();
    if clean.width() <= width {
        return clean;
    }
    if width == 0 {
        return String::new();
    }
    let (mut used, mut result) = (0, String::new());
    for c in clean.chars() {
        let w = c.width().unwrap_or(0);
        if used + w >= width {
            break;
        }
        result.push(c);
        used += w;
    }
    result.push('…');
    result
}

pub fn inner(area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    area.inner(ratatui::layout::Margin::new(1, 1))
}

/// A bounded settings surface inside the host-owned plugin frame.
pub(crate) fn dialog(
    t: &crate::theme::Theme,
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    title: &str,
    width: u16,
    height: u16,
) -> ratatui::layout::Rect {
    let area = crate::theme::centered(area, width, height);
    let block = t.block(format!(" {title} "), true);
    let body = block.inner(area);
    frame.render_widget(block, area);
    body.inner(ratatui::layout::Margin::new(1, 0))
}
