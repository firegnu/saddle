//! A decorative pet patrolling spare tab-strip space, drawn from a text pet pack. No agent
//! state or input.
use anyhow::{Context, Result, ensure};
use ratatui::{Frame, layout::Rect, style::Color};
use serde::Deserialize;
use std::{collections::BTreeMap, sync::LazyLock};

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
}
struct Pack {
    /// Index 0 is unused: it stands for transparent.
    palette: Vec<Color>,
    poses: Vec<[Cell; CELLS]>,
    clips: Vec<Clip>,
}

#[derive(Deserialize)]
struct Source {
    palette: BTreeMap<char, String>,
    poses: BTreeMap<String, PoseSource>,
    clip: Vec<ClipSource>,
}
#[derive(Deserialize)]
struct PoseSource {
    /// Six rows of 32 palette letters: each cell is 2x2 "bricks", `.` is transparent.
    art: String,
    /// `[column, row, glyph, fg, bg]` replaces one cell, for eyes and other glyphs.
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
                let mask = (0..4).filter(|&b| bricks[b] == fg && fg != 0).map(|b| 1 << b);
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
            });
        }
        for required in ["walking", "turning"] {
            ensure!(
                clips.iter().any(|c| c.name == required),
                "missing the {required} clip"
            );
        }
        Ok(Self {
            palette,
            poses,
            clips,
        })
    }
    fn clip(&self, name: &str) -> &Clip {
        self.clips.iter().find(|c| c.name == name).unwrap()
    }
}

// Built-in packs are checked in and covered by tests; nothing is read from disk at runtime.
static CLAWD: LazyLock<Pack> = LazyLock::new(|| {
    Pack::parse(include_str!("../assets/pets/clawd.toml")).expect("built-in pet pack")
});

#[derive(Clone, Copy)]
enum Motion {
    Walking,
    Acting(usize),
    Turning,
}
pub struct Mascot {
    pack: &'static Pack,
    last: Option<f64>,
    phase: f64,
    x: f64,
    right: bool,
    motion: Motion,
    walk_for: f64,
    random: u64,
    recent: Vec<usize>,
    palette: Vec<Color>,
}
impl Default for Mascot {
    fn default() -> Self {
        Self::new(true)
    }
}
impl Mascot {
    pub fn new(truecolor: bool) -> Self {
        let pack: &'static Pack = &CLAWD;
        Self {
            pack,
            last: None,
            phase: 0.0,
            x: 0.0,
            right: true,
            motion: Motion::Walking,
            walk_for: 3.0,
            random: 0,
            recent: Vec::new(),
            palette: pack
                .palette
                .iter()
                .map(|&c| {
                    if truecolor {
                        c
                    } else {
                        crate::theme::nearest_256(c)
                    }
                })
                .collect(),
        }
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
    fn walk(&mut self) {
        self.motion = Motion::Walking;
        self.phase = 0.0;
        self.walk_for = 2.2 + self.random() * 2.4;
    }
    fn act(&mut self) {
        let choices: Vec<_> = self
            .pack
            .clips
            .iter()
            .enumerate()
            .filter(|(i, c)| {
                !matches!(c.name.as_str(), "walking" | "turning") && !self.recent.contains(i)
            })
            .map(|(i, _)| i)
            .collect();
        let index = choices[(self.random() * choices.len() as f64) as usize];
        self.recent.push(index);
        if self.recent.len() > 5 {
            self.recent.remove(0);
        }
        self.phase = 0.0;
        self.motion = Motion::Acting(index);
    }
    fn advance(&mut self, now: f64, limit: f64) {
        if self.random == 0 {
            self.random = now.to_bits() | 1;
        }
        // No catch-up after a hidden view, suspend, clock jump, or slow frame.
        let dt = self.last.map_or(0.0, |last| (now - last).clamp(0.0, 0.25));
        self.last = Some(now);
        self.phase += dt;
        self.x = self.x.clamp(0.0, limit);
        match self.motion {
            Motion::Walking => {
                self.x += if self.right { dt * 3.0 } else { -dt * 3.0 };
                if limit > 0.0
                    && ((self.right && self.x >= limit) || (!self.right && self.x <= 0.0))
                {
                    self.x = self.x.clamp(0.0, limit);
                    self.motion = Motion::Turning;
                    self.phase = 0.0;
                } else if self.phase >= self.walk_for {
                    self.act();
                }
            }
            Motion::Acting(index) => {
                if self.phase >= self.pack.clips[index].ticks.len() as f64 / FPS {
                    self.walk();
                }
            }
            Motion::Turning => {
                if self.phase >= self.pack.clip("turning").ticks.len() as f64 / FPS {
                    self.right = !self.right;
                    self.walk();
                }
            }
        }
        self.x = self.x.clamp(0.0, limit);
    }
    fn pose(&self) -> &[Cell; CELLS] {
        let tick = (self.phase * FPS) as usize;
        let (clip, tick) = match self.motion {
            Motion::Walking => {
                let clip = self.pack.clip("walking");
                (clip, tick % clip.ticks.len())
            }
            Motion::Turning => (self.pack.clip("turning"), tick),
            Motion::Acting(index) => (&self.pack.clips[index], tick),
        };
        &self.pack.poses[clip.ticks[tick.min(clip.ticks.len() - 1)]]
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
        self.advance(now, f64::from(area.width - WIDTH - 2));
        let x = area.x + 1 + self.x as u16;
        let y = area.bottom() - HEIGHT;
        let mut painted = Vec::new();
        for (i, sample) in self.pose().iter().enumerate() {
            if (sample.glyph == ' ' || sample.fg == 0) && sample.bg == 0 {
                continue;
            }
            let col = i as u16 % WIDTH;
            let row = i as u16 / WIDTH;
            let position = (x + col, y + row);
            if protected.iter().any(|r| r.contains(position.into())) {
                continue;
            }
            let cell = &mut frame.buffer_mut()[position];
            let background = if sample.bg == 0 {
                cell.bg
            } else {
                self.palette[usize::from(sample.bg)]
            };
            let mut glyph = [0; 4];
            cell.reset();
            cell.set_symbol(sample.glyph.encode_utf8(&mut glyph))
                .set_fg(self.palette[usize::from(sample.fg)])
                .set_bg(background);
            painted.push(Rect::new(position.0, position.1, 1, 1));
        }
        painted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Why a one-pose pack is refused, or None when it parses.
    fn refusal(art: &str, cells: &str, pose: &str) -> Option<String> {
        Pack::parse(&format!(
            "[palette]\nA = \"#ff0000\"\nB = \"#00ff00\"\n[poses.p]\nart = '''\n{art}'''\ncells = [{cells}]\n\
             [[clip]]\nname = \"walking\"\nframes = [[\"{pose}\", 2]]\n\
             [[clip]]\nname = \"turning\"\nframes = [[\"p\", 1]]\n"
        ))
        .err()
        .map(|e| e.to_string())
    }
    #[test]
    fn broken_packs_are_refused_with_the_reason() {
        let row = ".".repeat(32) + "\n";
        let blank = row.repeat(6);
        assert_eq!(refusal(&blank, "", "p"), None);
        // Two colors in one cell are a foreground and a background.
        let two = format!("AB{}\n", ".".repeat(30)).repeat(2) + &row.repeat(4);
        assert_eq!(refusal(&two, "", "p"), None);
        let reason = |art: &str, cells: &str, pose: &str| refusal(art, cells, pose).unwrap();
        assert!(reason(&row.repeat(5), "", "p").contains("art must be"));
        let three = format!("AB{}\n", ".".repeat(30)) + &row.repeat(5);
        assert!(reason(&three, "", "p").contains("three colors"));
        assert!(reason(&blank, "[16, 0, \"x\", \"A\", \".\"]", "p").contains("outside"));
        assert!(reason(&blank, "[0, 0, \"x\", \"Z\", \".\"]", "p").contains("unknown color"));
        assert!(reason(&blank, "[0, 0, \"宽\", \"A\", \".\"]", "p").contains("one column"));
        assert!(reason(&blank, "", "q").contains("unknown pose"));
    }
}
