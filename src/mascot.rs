//! A decorative pet patrolling spare tab-strip space, drawn from a text pet pack: with block
//! glyphs, or as pictures where the terminal shows them (see `kitty`). No agent state or input.
use anyhow::{Context, Result, ensure};
use ratatui::{
    Frame,
    buffer::{self, Buffer},
    layout::{Position, Rect},
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
    /// The pet drawn as pixel images, parsed only when the terminal can show them.
    fn image_pack(self) -> Arc<Pack> {
        static PACKS: LazyLock<[Arc<Pack>; 2]> = LazyLock::new(|| {
            [
                include_str!("../assets/pets/clawd-image.toml"),
                include_str!("../assets/pets/cat-image.toml"),
            ]
            .map(|text| Arc::new(Pack::parse(text).expect("built-in pet image pack")))
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

/// How the pet is drawn: pictures where the terminal shows them, or always block glyphs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Display {
    #[default]
    Auto,
    Blocks,
}
impl Display {
    pub const ALL: [Display; 2] = [Display::Auto, Display::Blocks];
    /// How the display is written in the config file.
    pub fn name(self) -> &'static str {
        match self {
            Display::Auto => "auto",
            Display::Blocks => "blocks",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Display::Auto => "Auto",
            Display::Blocks => "Blocks",
        }
    }
    pub fn parse(value: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|d| d.name() == value)
            .ok_or_else(|| format!("unknown mascot_display {value:?}: expected auto or blocks"))
    }
}
impl<'de> Deserialize<'de> for Display {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Display::parse(&value).map_err(serde::de::Error::custom)
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
/// The poses of a pack: terminal cells, or pixel images of `width` x `height` palette indexes.
enum Art {
    Cells(Vec<[Cell; CELLS]>),
    Pixels {
        width: usize,
        height: usize,
        poses: Vec<Vec<u8>>,
    },
}
struct Pack {
    /// Index 0 is unused: it stands for transparent.
    palette: Vec<Color>,
    art: Art,
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
    /// `[width, height]` of the pixel images, in an image pack.
    size: Option<(usize, usize)>,
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
    art: Option<String>,
    /// `size` rows of palette letters, one per pixel, in an image pack.
    pixels: Option<String>,
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
        let mut images = Vec::new();
        if let Some((width, height)) = source.size {
            ensure!(width > 0 && height > 0, "size must be positive");
        }
        for (name, pose) in &source.poses {
            names.insert(name.as_str(), names.len());
            if let Some((width, height)) = source.size {
                ensure!(
                    pose.art.is_none() && pose.cells.is_empty(),
                    "pose {name}: an image pack draws its poses as pixels"
                );
                let pixels = pose
                    .pixels
                    .as_deref()
                    .with_context(|| format!("pose {name}: missing pixels"))?;
                let rows: Vec<_> = pixels.lines().filter(|row| !row.is_empty()).collect();
                ensure!(
                    rows.len() == height && rows.iter().all(|row| row.chars().count() == width),
                    "pose {name}: pixels must be {height} rows of {width} letters"
                );
                let image = rows.iter().flat_map(|row| row.chars());
                images.push(image.map(|c| color(c, name)).collect::<Result<_>>()?);
                continue;
            }
            ensure!(
                pose.pixels.is_none(),
                "pose {name}: pixels need the pack's size"
            );
            let rows: Vec<Vec<char>> = pose
                .art
                .as_deref()
                .with_context(|| format!("pose {name}: missing art"))?
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
        let art = match source.size {
            Some((width, height)) => Art::Pixels {
                width,
                height,
                poses: images,
            },
            None => Art::Cells(poses),
        };
        Ok(Self {
            palette,
            art,
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
    /// Pixels per terminal cell, for an image pack.
    cell: Option<(u16, u16)>,
    /// The picture drawn this frame, and the cells under it as the draw left them.
    shown: Option<(Sprite, Vec<(Position, buffer::Cell)>)>,
}
/// Where a picture goes on the canvas, in pixels: the cell size, the picture size, and the
/// picture's top-left corner.
struct Layout {
    cell: (u32, u32),
    size: (u32, u32),
    corner: (u32, u32),
}
/// The pet as one picture, for a terminal that shows images.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Sprite {
    /// Tells pictures apart: the pack, the pose, and whether it is flipped.
    pub key: (usize, usize, bool),
    /// The cell holding the top-left corner, and the corner's pixel offset in that cell.
    pub cell: (u16, u16),
    pub offset: (u16, u16),
    /// Size in pixels.
    pub size: (u16, u16),
}
/// Set on the blank cells under a picture. A space never shows it, and a cell that lost it was
/// drawn over later in the frame.
const MARKER: Color = Color::Rgb(1, 2, 3);
impl Default for Mascot {
    fn default() -> Self {
        Self::new(Pet::default(), true)
    }
}
impl Mascot {
    pub fn new(pet: Pet, truecolor: bool) -> Self {
        Self::with(pet.pack(), truecolor)
    }
    /// The pet as pixel images, for a terminal whose cells are `cell` pixels in size.
    pub fn with_images(pet: Pet, cell: (u16, u16)) -> Self {
        let mut mascot = Self::with(pet.image_pack(), true);
        mascot.cell = Some(cell);
        mascot
    }
    /// The pet as `display` asks, given the cell size in pixels when the terminal shows pictures.
    pub fn for_display(
        pet: Pet,
        display: Display,
        cell: Option<(u16, u16)>,
        truecolor: bool,
    ) -> Self {
        match (display, cell) {
            (Display::Auto, Some(cell)) => Self::with_images(pet, cell),
            _ => Self::new(pet, truecolor),
        }
    }
    /// A pet from pack text, as documented in `assets/pets/README.md`.
    pub fn from_pack(text: &str, truecolor: bool) -> Result<Self> {
        let pack = Pack::parse(text)?;
        ensure!(
            matches!(pack.art, Art::Cells(_)),
            "an image pack needs a cell size"
        );
        Ok(Self::with(Arc::new(pack), truecolor))
    }
    /// A pet from image pack text, drawn with cells of `cell` pixels.
    pub fn from_image_pack(text: &str, cell: (u16, u16)) -> Result<Self> {
        let pack = Pack::parse(text)?;
        ensure!(matches!(pack.art, Art::Pixels { .. }), "not an image pack");
        let mut mascot = Self::with(Arc::new(pack), true);
        mascot.cell = Some(cell);
        Ok(mascot)
    }
    /// Follows a change of font size; a pet drawn with glyphs ignores it.
    pub fn set_cell(&mut self, cell: (u16, u16)) {
        if self.cell.is_some() {
            self.cell = Some(cell);
        }
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
            cell: None,
            shown: None,
        };
        mascot.walk(3.0);
        mascot
    }
    pub fn hide(&mut self) {
        self.last = None;
        self.shown = None;
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
    fn pose(&self) -> (usize, bool) {
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
        (ticks[tick.min(ticks.len() - 1)], flip)
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
        self.shown = None;
        let pack = self.pack.clone();
        match &pack.art {
            Art::Cells(poses) => self.draw_cells(frame, &poses[pose], flip, (x, y), protected),
            Art::Pixels {
                width,
                height,
                poses,
            } => {
                let image = (*width as u32, *height as u32, &poses[pose][..]);
                let key = (Arc::as_ptr(&pack) as usize, pose, flip);
                self.draw_image(frame, image, key, (x, y), protected)
            }
        }
    }
    /// Where a source picture of `width` x `height` goes: as large as fits, keeping its shape,
    /// standing on the floor in the middle.
    fn layout(&self, width: u32, height: u32) -> Option<Layout> {
        let (cw, ch) = self.cell?;
        let cell = (u32::from(cw.max(1)), u32::from(ch.max(1)));
        let lane = (u32::from(WIDTH) * cell.0, u32::from(HEIGHT) * cell.1);
        let scale = f64::min(
            f64::from(lane.0) / f64::from(width),
            f64::from(lane.1) / f64::from(height),
        );
        let size = (
            ((f64::from(width) * scale) as u32).clamp(1, lane.0),
            ((f64::from(height) * scale) as u32).clamp(1, lane.1),
        );
        Some(Layout {
            cell,
            size,
            corner: ((lane.0 - size.0) / 2, lane.1 - size.1),
        })
    }
    /// Blanks the cells a picture covers, for the picture to show through, unless one of them
    /// is protected.
    fn draw_image(
        &mut self,
        frame: &mut Frame,
        (width, height, pixels): (u32, u32, &[u8]),
        key: (usize, usize, bool),
        (x, y): (u16, u16),
        protected: &[Rect],
    ) -> Vec<Rect> {
        let Some(Layout { cell, size, corner }) = self.layout(width, height) else {
            return Vec::new();
        };
        // Nearest neighbour: picture pixel `d` shows source pixel `d * width / size`, so source
        // pixel `s` covers picture pixels from `ceil(s * size / width)`.
        let span = |s: u32, source: u32, size: u32, start: u32| {
            (start + (s * size).div_ceil(source))..(start + ((s + 1) * size).div_ceil(source))
        };
        let mut covered = std::collections::BTreeSet::new();
        for sy in 0..height {
            for sx in 0..width {
                let source = if key.2 { width - 1 - sx } else { sx };
                if pixels[(sy * width + source) as usize] == 0 {
                    continue;
                }
                let (across, down) = (
                    span(sx, width, size.0, corner.0),
                    span(sy, height, size.1, corner.1),
                );
                if across.is_empty() || down.is_empty() {
                    continue;
                }
                for row in down.start / cell.1..=(down.end - 1) / cell.1 {
                    for col in across.start / cell.0..=(across.end - 1) / cell.0 {
                        covered.insert(Position::new(x + col as u16, y + row as u16));
                    }
                }
            }
        }
        if covered
            .iter()
            .any(|&p| protected.iter().any(|r| r.contains(p)))
        {
            return Vec::new();
        }
        let mut cells = Vec::new();
        for &position in &covered {
            let cell = &mut frame.buffer_mut()[position];
            let under = cell.bg;
            cell.reset();
            cell.set_bg(under).set_fg(MARKER);
            cells.push((position, cell.clone()));
        }
        let sprite = Sprite {
            key,
            cell: (
                x + (corner.0 / cell.0) as u16,
                y + (corner.1 / cell.1) as u16,
            ),
            offset: ((corner.0 % cell.0) as u16, (corner.1 % cell.1) as u16),
            size: (size.0 as u16, size.1 as u16),
        };
        self.shown = Some((sprite, cells));
        covered
            .into_iter()
            .map(|p| Rect::new(p.x, p.y, 1, 1))
            .collect()
    }
    /// The picture for the frame just drawn into `buffer`, unless something was drawn over it.
    pub fn sprite(&mut self, buffer: &Buffer) -> Option<Sprite> {
        let (sprite, cells) = self.shown.take()?;
        cells
            .iter()
            .all(|(position, cell)| buffer.cell(*position) == Some(cell))
            .then_some(sprite)
    }
    /// The picture's pixels, as RGBA rows.
    pub fn pixels(&self, sprite: &Sprite) -> Vec<u8> {
        let Art::Pixels {
            width,
            height,
            poses,
        } = &self.pack.art
        else {
            return Vec::new();
        };
        let (_, pose, flip) = sprite.key;
        let (w, h) = (*width as u32, *height as u32);
        let (sw, sh) = (u32::from(sprite.size.0), u32::from(sprite.size.1));
        let mut rgba = Vec::with_capacity((sw * sh * 4) as usize);
        for dy in 0..sh {
            let sy = dy * h / sh;
            for dx in 0..sw {
                let s = dx * w / sw;
                let sx = if flip { w - 1 - s } else { s };
                let index = poses[pose][(sy * w + sx) as usize];
                rgba.extend(match self.pack.palette[usize::from(index)] {
                    Color::Rgb(r, g, b) => [r, g, b, 255],
                    _ => [0; 4],
                });
            }
        }
        rgba
    }
    fn draw_cells(
        &self,
        frame: &mut Frame,
        pose: &[Cell; CELLS],
        flip: bool,
        (x, y): (u16, u16),
        protected: &[Rect],
    ) -> Vec<Rect> {
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
    fn broken_image_packs_are_refused_with_the_reason() {
        let text = |size: &str, pose: &str| {
            format!(
                "{size}[palette]\nA = \"#ff0000\"\n[poses.p]\n{pose}\n\
                 [[clip]]\nname = \"walking\"\nframes = [[\"p\", 4]]\n\
                 [[clip]]\nname = \"turning\"\nframes = [[\"p\", 1]]\n\
                 [[clip]]\nname = \"act\"\nframes = [[\"p\", 1]]\n"
            )
        };
        let reason = |size: &str, pose: &str| Pack::parse(&text(size, pose)).err().unwrap();
        let pixels = "pixels = '''\nA.\n.A\n'''";
        assert!(Pack::parse(&text("size = [2, 2]\n", pixels)).is_ok());
        let wrong = reason("size = [3, 2]\n", pixels).to_string();
        assert!(wrong.contains("2 rows of 3 letters"), "{wrong}");
        let unknown = "pixels = '''\nA.\n.Z\n'''";
        assert!(
            reason("size = [2, 2]\n", unknown)
                .to_string()
                .contains("unknown color")
        );
        let art = "art = 'x'";
        assert!(
            reason("size = [2, 2]\n", art)
                .to_string()
                .contains("as pixels")
        );
        assert!(
            reason("", pixels)
                .to_string()
                .contains("need the pack's size")
        );
        assert!(
            reason("size = [0, 2]\n", pixels)
                .to_string()
                .contains("positive")
        );
        // Each kind of pack goes with its own way of drawing.
        assert!(Mascot::from_pack(&text("size = [2, 2]\n", pixels), true).is_err());
        let blank = format!("art = '''\n{}'''", (".".repeat(32) + "\n").repeat(6));
        assert!(Mascot::from_image_pack(&text("", &blank), (8, 16)).is_err());
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
