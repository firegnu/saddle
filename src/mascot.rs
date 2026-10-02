//! A decorative pet patrolling spare tab-strip space, drawn from a text pet pack. No agent
//! state or input.
use anyhow::{Context, Result, ensure};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier},
};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    sync::{Arc, LazyLock},
};

pub const HEIGHT: u16 = 3;
const WIDTH: u16 = 16;
pub const MIN_WIDTH: u16 = WIDTH + 2;
const CELLS: usize = WIDTH as usize * HEIGHT as usize;
const FPS: f64 = 12.0;
/// Indexed by which quarters of a cell are filled: top-left 1, top-right 2, bottom-left 4,
/// bottom-right 8.
const QUADRANTS: [char; 16] = [
    ' ', '▘', '▝', '▀', '▖', '▌', '▞', '▛', '▗', '▚', '▐', '▜', '▄', '▙', '▟', '█',
];

/// The built-in pets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pet {
    #[default]
    Clawd,
    Cat,
}
impl Pet {
    pub const ALL: [Pet; 2] = [Pet::Clawd, Pet::Cat];
    /// How the pet is written in the config file.
    pub fn name(self) -> &'static str {
        match self {
            Pet::Clawd => "clawd",
            Pet::Cat => "cat",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Pet::Clawd => "Clawd",
            Pet::Cat => "Cat",
        }
    }
    pub fn parse(value: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|p| p.name() == value)
            .ok_or_else(|| format!("unknown mascot {value:?}: expected clawd or cat"))
    }
    fn pack(self) -> Arc<Pack> {
        // Built-in packs are checked in and covered by tests; nothing is read from disk.
        static PACKS: LazyLock<[Arc<Pack>; 2]> = LazyLock::new(|| {
            [
                include_str!("../assets/pets/clawd.toml"),
                include_str!("../assets/pets/cat.toml"),
            ]
            .map(|text| Arc::new(Pack::parse(text).expect("built-in pet pack")))
        });
        PACKS[self as usize].clone()
    }
}
impl<'de> Deserialize<'de> for Pet {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Pet::parse(&value).map_err(serde::de::Error::custom)
    }
}

/// One terminal cell of a pose; colors are palette indexes and 0 is transparent.
#[derive(Clone, Copy)]
struct Cell {
    glyph: char,
    fg: u8,
    bg: u8,
}
struct Clip {
    name: String,
    /// The pose shown at each 12 fps tick.
    ticks: Vec<usize>,
    /// The clip drawn for heading left, when the pack has `<name>-left`.
    left: Option<usize>,
}
struct Pack {
    /// Index 0 is unused: it stands for transparent.
    palette: Vec<Color>,
    poses: Vec<[Cell; CELLS]>,
    clips: Vec<Clip>,
    walking: usize,
    turning: usize,
    /// Clips played between walks.
    actions: Vec<usize>,
    /// Poses without a `-left` clip are flipped when heading left.
    mirror: bool,
    /// Ticks per cell of travel; the walking clip changes pose on the same ticks.
    step_ticks: usize,
}

#[derive(Deserialize)]
struct Source {
    #[serde(default = "default_step")]
    step_ticks: usize,
    #[serde(default)]
    mirror: bool,
    palette: BTreeMap<char, String>,
    poses: BTreeMap<String, PoseSource>,
    clip: Vec<ClipSource>,
}
fn default_step() -> usize {
    4
}
#[derive(Deserialize)]
struct PoseSource {
    /// Six rows of 32 palette letters: each cell is 2x2 "bricks", `.` is transparent.
    art: String,
    /// `[column, row, glyph, fg, bg]` replaces one cell, for eyes and finer details.
    #[serde(default)]
    cells: Vec<(usize, usize, char, char, char)>,
}
#[derive(Deserialize)]
struct ClipSource {
    name: String,
    /// `[pose, ticks]` in order.
    frames: Vec<(String, usize)>,
}

impl Pack {
    fn parse(text: &str) -> Result<Self> {
        let source: Source = toml::from_str(text)?;
        ensure!(source.step_ticks > 0, "step_ticks must be positive");
        let mut palette = vec![Color::Reset];
        let mut letters = BTreeMap::from([('.', 0u8)]);
        for (&letter, hex) in &source.palette {
            let rgb = u32::from_str_radix(hex.trim_start_matches('#'), 16)
                .ok()
                .filter(|_| hex.len() == 7)
                .with_context(|| format!("palette {letter}: expected #rrggbb, got {hex:?}"))?;
            letters.insert(letter, u8::try_from(palette.len())?);
            palette.push(Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8));
        }
        let color = |letter: char, pose: &str| {
            letters
                .get(&letter)
                .copied()
                .with_context(|| format!("pose {pose}: unknown color {letter:?}"))
        };
        let mut names = BTreeMap::new();
        let mut poses = Vec::new();
        for (name, pose) in &source.poses {
            let rows: Vec<Vec<char>> = pose
                .art
                .lines()
                .filter(|row| !row.is_empty())
                .map(|row| row.chars().collect())
                .collect();
            ensure!(
                rows.len() == usize::from(HEIGHT) * 2
                    && rows.iter().all(|row| row.len() == usize::from(WIDTH) * 2),
                "pose {name}: art must be {} rows of {} letters",
                HEIGHT * 2,
                WIDTH * 2
            );
            let mut cells = [Cell {
                glyph: ' ',
                fg: 0,
                bg: 0,
            }; CELLS];
            for (i, cell) in cells.iter_mut().enumerate() {
                let (x, y) = (i % usize::from(WIDTH) * 2, i / usize::from(WIDTH) * 2);
                let mut bricks = [0; 4];
                for (brick, (dx, dy)) in bricks.iter_mut().zip([(0, 0), (1, 0), (0, 1), (1, 1)]) {
                    *brick = color(rows[y + dy][x + dx], name)?;
                }
                // A terminal cell has one foreground and one background.
                let fg = *bricks.iter().max().unwrap();
                let bg = *bricks.iter().min().unwrap();
                ensure!(
                    bricks.iter().all(|&b| b == fg || b == bg),
                    "pose {name}: three colors in cell {},{}",
                    x / 2,
                    y / 2
                );
                let mask = (0..4)
                    .filter(|&b| bricks[b] == fg && fg != 0)
                    .map(|b| 1 << b);
                *cell = Cell {
                    glyph: QUADRANTS[mask.sum::<usize>()],
                    fg,
                    bg: if bg == fg { 0 } else { bg },
                };
            }
            for &(x, y, glyph, fg, bg) in &pose.cells {
                ensure!(
                    x < usize::from(WIDTH) && y < usize::from(HEIGHT),
                    "pose {name}: cell {x},{y} is outside the canvas"
                );
                ensure!(
                    unicode_width::UnicodeWidthChar::width(glyph) == Some(1),
                    "pose {name}: glyph {glyph:?} must be one column wide"
                );
                cells[y * usize::from(WIDTH) + x] = Cell {
                    glyph,
                    fg: color(fg, name)?,
                    bg: color(bg, name)?,
                };
            }
            names.insert(name.as_str(), poses.len());
            poses.push(cells);
        }
        let mut clips = Vec::new();
        for clip in &source.clip {
            let mut ticks = Vec::new();
            for (pose, count) in &clip.frames {
                let index = *names
                    .get(pose.as_str())
                    .with_context(|| format!("clip {}: unknown pose {pose:?}", clip.name))?;
                ticks.extend(std::iter::repeat_n(index, *count));
            }
            ensure!(!ticks.is_empty(), "clip {}: no frames", clip.name);
            clips.push(Clip {
                name: clip.name.clone(),
                ticks,
                left: None,
            });
        }
        let find = |name: &str| clips.iter().position(|c| c.name == name);
        let lefts: Vec<_> = clips
            .iter()
            .map(|c| find(&format!("{}-left", c.name)))
            .collect();
        for (clip, &left) in clips.iter().zip(&lefts) {
            ensure!(
                left.is_none_or(|l| clips[l].ticks.len() == clip.ticks.len()),
                "clip {}-left must last as long as {}",
                clip.name,
                clip.name
            );
        }
        let walking = find("walking").context("missing the walking clip")?;
        let turning = find("turning").context("missing the turning clip")?;
        ensure!(
            clips[walking].ticks.len() % source.step_ticks == 0,
            "the walking clip must be a whole number of steps"
        );
        let actions: Vec<_> = (0..clips.len())
            .filter(|&i| i != walking && i != turning && !clips[i].name.ends_with("-left"))
            .collect();
        ensure!(!actions.is_empty(), "no action clips");
        for (clip, left) in clips.iter_mut().zip(lefts) {
            clip.left = left;
        }
        Ok(Self {
            palette,
            poses,
            clips,
            walking,
            turning,
            actions,
            mirror: source.mirror,
            step_ticks: source.step_ticks,
        })
    }
}

/// The glyph as seen in a mirror, and whether its two colors trade places.
fn mirrored(glyph: char) -> (char, bool) {
    if let Some(mask) = QUADRANTS.iter().position(|&q| q == glyph) {
        let flipped = (mask & 1) << 1 | (mask & 2) >> 1 | (mask & 4) << 1 | (mask & 8) >> 1;
        return (QUADRANTS[flipped], false);
    }
    match glyph {
        // Left n/8 becomes right n/8: the left (8-n)/8 glyph with the colors swapped.
        '▉'..='▏' => (
            char::from_u32('▉' as u32 + '▏' as u32 - glyph as u32).unwrap(),
            true,
        ),
        '▕' => ('▏', false),
        '(' => (')', false),
        ')' => ('(', false),
        '<' => ('>', false),
        '>' => ('<', false),
        '/' => ('\\', false),
        '\\' => ('/', false),
        '┐' => ('┌', false),
        '┌' => ('┐', false),
        '┘' => ('└', false),
        '└' => ('┘', false),
        other => (other, false),
    }
}

#[derive(Clone, Copy)]
enum Motion {
    Walking,
    Acting(usize),
    Turning,
}
pub struct Mascot {
    pack: Arc<Pack>,
    last: Option<f64>,
    /// Seconds into the current motion.
    phase: f64,
    /// Cells from the left end of the lane.
    x: u16,
    right: bool,
    motion: Motion,
    /// Steps already taken, and ticks to walk, in the current walk.
    stepped: usize,
    walk_ticks: usize,
    random: u64,
    recent: Vec<usize>,
    palette: Vec<Color>,
}
impl Default for Mascot {
    fn default() -> Self {
        Self::new(Pet::default(), true)
    }
}
impl Mascot {
    pub fn new(pet: Pet, truecolor: bool) -> Self {
        Self::with(pet.pack(), truecolor)
    }
    /// A pet from pack text, as documented in `assets/pets/README.md`.
    pub fn from_pack(text: &str, truecolor: bool) -> Result<Self> {
        Ok(Self::with(Arc::new(Pack::parse(text)?), truecolor))
    }
    fn with(pack: Arc<Pack>, truecolor: bool) -> Self {
        let palette = pack
            .palette
            .iter()
            .map(|&c| {
                if truecolor {
                    c
                } else {
                    crate::theme::nearest_256(c)
                }
            })
            .collect();
        let mut mascot = Self {
            pack,
            last: None,
            phase: 0.0,
            x: 0,
            right: true,
            motion: Motion::Walking,
            stepped: 0,
            walk_ticks: 0,
            random: 0,
            recent: Vec::new(),
            palette,
        };
        mascot.walk(3.0);
        mascot
    }
    pub fn hide(&mut self) {
        self.last = None;
    }
    fn random(&mut self) -> f64 {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 7;
        self.random ^= self.random << 17;
        (self.random >> 11) as f64 / ((1u64 << 53) as f64)
    }
    /// Walks for about `seconds`, ending on a whole walk cycle so the feet finish together.
    fn walk(&mut self, seconds: f64) {
        let cycle = self.pack.clips[self.pack.walking].ticks.len();
        self.motion = Motion::Walking;
        self.phase = 0.0;
        self.stepped = 0;
        self.walk_ticks = ((seconds * FPS / cycle as f64).ceil() as usize).max(1) * cycle;
    }
    fn act(&mut self) {
        let choices: Vec<_> = self
            .pack
            .actions
            .iter()
            .copied()
            .filter(|i| !self.recent.contains(i))
            .collect();
        // A small pack can have fewer actions than the recent list holds.
        let choices = if choices.is_empty() {
            self.recent.clear();
            self.pack.actions.clone()
        } else {
            choices
        };
        let index = choices[(self.random() * choices.len() as f64) as usize];
        self.recent.push(index);
        if self.recent.len() > 5 {
            self.recent.remove(0);
        }
        self.phase = 0.0;
        self.motion = Motion::Acting(index);
    }
    /// Whole ticks into the current motion. Frame times are sums of fractions, so a tick
    /// boundary reached "exactly" must not read as the tick before it.
    fn tick(&self) -> usize {
        (self.phase * FPS + 1e-6) as usize
    }
    fn advance(&mut self, now: f64, limit: u16) {
        if self.random == 0 {
            self.random = now.to_bits() | 1;
        }
        // No catch-up after a hidden view, suspend, clock jump, or slow frame.
        let dt = self.last.map_or(0.0, |last| (now - last).clamp(0.0, 0.25));
        self.last = Some(now);
        self.phase += dt;
        self.x = self.x.min(limit);
        match self.motion {
            Motion::Walking => {
                // The body moves a cell on the same ticks the walking clip changes pose.
                while self.stepped < self.tick() / self.pack.step_ticks {
                    self.stepped += 1;
                    if self.right && self.x < limit {
                        self.x += 1;
                    } else if !self.right && self.x > 0 {
                        self.x -= 1;
                    }
                    if limit > 0 && self.x == if self.right { limit } else { 0 } {
                        self.motion = Motion::Turning;
                        self.phase = 0.0;
                        return;
                    }
                }
                if self.tick() >= self.walk_ticks {
                    self.act();
                }
            }
            Motion::Acting(index) => {
                if self.tick() >= self.pack.clips[index].ticks.len() {
                    let seconds = 2.2 + self.random() * 2.4;
                    self.walk(seconds);
                }
            }
            Motion::Turning => {
                let length = self.pack.clips[self.pack.turning].ticks.len();
                if self.tick() >= length {
                    self.right = self.heading();
                    let seconds = 2.2 + self.random() * 2.4;
                    self.walk(seconds);
                }
            }
        }
    }
    /// Whether the pet faces right now: a turn changes its facing halfway through.
    fn heading(&self) -> bool {
        match self.motion {
            Motion::Turning
                if self.tick() >= self.pack.clips[self.pack.turning].ticks.len().div_ceil(2) =>
            {
                !self.right
            }
            _ => self.right,
        }
    }
    /// The pose to draw and whether to flip it.
    fn pose(&self) -> (&[Cell; CELLS], bool) {
        let pack = &self.pack;
        let tick = self.tick();
        let (index, tick) = match self.motion {
            Motion::Walking => (pack.walking, tick % pack.clips[pack.walking].ticks.len()),
            Motion::Turning => (pack.turning, tick),
            Motion::Acting(index) => (index, tick),
        };
        let (index, flip) = match (self.heading(), pack.clips[index].left) {
            (true, _) => (index, false),
            (false, Some(left)) => (left, false),
            (false, None) => (index, pack.mirror),
        };
        let ticks = &pack.clips[index].ticks;
        (&pack.poses[ticks[tick.min(ticks.len() - 1)]], flip)
    }
    pub fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        now: f64,
        protected: &[Rect],
    ) -> Vec<Rect> {
        let area = area.intersection(frame.area());
        if area.width < MIN_WIDTH || area.height < HEIGHT {
            self.hide();
            return Vec::new();
        }
        self.advance(now, area.width - WIDTH - 2);
        let x = area.x + 1 + self.x;
        let y = area.bottom() - HEIGHT;
        let (pose, flip) = self.pose();
        let mut painted = Vec::new();
        for (i, sample) in pose.iter().enumerate() {
            let (mut col, row) = (i as u16 % WIDTH, i as u16 / WIDTH);
            let (mut glyph, mut fg, mut bg) = (sample.glyph, sample.fg, sample.bg);
            if flip {
                col = WIDTH - 1 - col;
                let (seen, swap) = mirrored(glyph);
                glyph = seen;
                if swap {
                    (fg, bg) = (bg, fg);
                }
            }
            if (glyph == ' ' || fg == 0) && bg == 0 {
                continue;
            }
            let position = (x + col, y + row);
            if protected.iter().any(|r| r.contains(position.into())) {
                continue;
            }
            let cell = &mut frame.buffer_mut()[position];
            let under = cell.bg;
            let color = |index: u8| self.palette[usize::from(index)];
            let mut symbol = [0; 4];
            cell.reset();
            cell.set_symbol(glyph.encode_utf8(&mut symbol));
            match (fg, bg) {
                // The glyph's own shape is the transparent part. The terminal's default
                // background has no color to draw it with, so that cell is reversed instead.
                (0, bg) if under == Color::Reset => {
                    cell.set_fg(color(bg)).set_bg(under);
                    cell.modifier.insert(Modifier::REVERSED);
                }
                (0, bg) => {
                    cell.set_fg(under).set_bg(color(bg));
                }
                (fg, 0) => {
                    cell.set_fg(color(fg)).set_bg(under);
                }
                (fg, bg) => {
                    cell.set_fg(color(fg)).set_bg(color(bg));
                }
            }
            painted.push(Rect::new(position.0, position.1, 1, 1));
        }
        painted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-pose pack, or why it is refused.
    fn pack(art: &str, cells: &str, pose: &str, extra: &str) -> Result<Pack, String> {
        Pack::parse(&format!(
            "[palette]\nA = \"#ff0000\"\nB = \"#00ff00\"\n[poses.p]\nart = '''\n{art}'''\ncells = [{cells}]\n\
             [[clip]]\nname = \"walking\"\nframes = [[\"{pose}\", 4]]\n\
             [[clip]]\nname = \"turning\"\nframes = [[\"p\", 1]]\n\
             [[clip]]\nname = \"act\"\nframes = [[\"p\", 1]]\n{extra}"
        ))
        .map_err(|e| e.to_string())
    }
    #[test]
    fn broken_packs_are_refused_with_the_reason() {
        let row = ".".repeat(32) + "\n";
        let blank = row.repeat(6);
        assert!(pack(&blank, "", "p", "").is_ok());
        // Two colors in one cell are a foreground and a background.
        let two = format!("AB{}\n", ".".repeat(30)).repeat(2) + &row.repeat(4);
        assert!(pack(&two, "", "p", "").is_ok());
        let reason = |art: &str, cells: &str, pose: &str| pack(art, cells, pose, "").err().unwrap();
        assert!(reason(&row.repeat(5), "", "p").contains("art must be"));
        let three = format!("AB{}\n", ".".repeat(30)) + &row.repeat(5);
        assert!(reason(&three, "", "p").contains("three colors"));
        assert!(reason(&blank, "[16, 0, \"x\", \"A\", \".\"]", "p").contains("outside"));
        assert!(reason(&blank, "[0, 0, \"x\", \"Z\", \".\"]", "p").contains("unknown color"));
        assert!(reason(&blank, "[0, 0, \"宽\", \"A\", \".\"]", "p").contains("one column"));
        assert!(reason(&blank, "", "q").contains("unknown pose"));
    }
    #[test]
    fn left_clips_pair_with_their_clip_and_are_never_actions() {
        let blank = (".".repeat(32) + "\n").repeat(6);
        let left = |ticks| format!("[[clip]]\nname = \"act-left\"\nframes = [[\"p\", {ticks}]]\n");
        let parsed = pack(&blank, "", "p", &left(1)).unwrap();
        let names: Vec<_> = parsed
            .actions
            .iter()
            .map(|&i| parsed.clips[i].name.as_str())
            .collect();
        assert_eq!(names, ["act"]);
        assert_eq!(parsed.clips[parsed.actions[0]].left, Some(3));
        assert!(
            pack(&blank, "", "p", &left(2))
                .err()
                .unwrap()
                .contains("must last as long")
        );
    }
    #[test]
    fn mirroring_flips_shapes_and_swaps_colors_only_for_side_bars() {
        assert_eq!(mirrored('▌'), ('▐', false));
        assert_eq!(mirrored('▙'), ('▟', false));
        assert_eq!(mirrored('▄'), ('▄', false));
        // A left quarter bar becomes "everything but the right quarter", colors swapped.
        assert_eq!(mirrored('▎'), ('▊', true));
        assert_eq!(mirrored('▉'), ('▏', true));
        assert_eq!(mirrored('▂'), ('▂', false));
        assert_eq!(mirrored('('), (')', false));
        assert_eq!(mirrored('▪'), ('▪', false));
    }
}
