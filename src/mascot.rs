//! A decorative patrol using offline frames from claude.dev. No agent state or input.
use ratatui::{Frame, layout::Rect, style::Color};
use std::sync::LazyLock;

pub const HEIGHT: u16 = 7;
const WIDTH: u16 = 18;
pub const MIN_WIDTH: u16 = WIDTH + 2;
const CELLS: usize = WIDTH as usize * HEIGHT as usize;
const FPS: f64 = 12.0;
const QUADRANTS: [&str; 16] = [
    " ", "▘", "▝", "▀", "▖", "▌", "▞", "▛", "▗", "▚", "▐", "▜", "▄", "▙", "▟", "█",
];
struct Clip {
    name: &'static str,
    frames: &'static [u8],
    top_padding: usize,
}
impl Clip {
    fn len(&self) -> usize {
        self.frames.len() / (CELLS * 3)
    }
    fn frame(&self, index: usize) -> &[u8] {
        let start = index.min(self.len() - 1) * CELLS * 3;
        &self.frames[start..start + CELLS * 3]
    }
}
struct Sprites {
    palette: Vec<Color>,
    clips: Vec<Clip>,
}
static SPRITES: LazyLock<Sprites> = LazyLock::new(|| {
    // Trusted, checked-in asset. No external paths, parsers, or runtime downloads.
    let data = include_bytes!("../assets/clawd/frames.bin");
    assert_eq!(&data[..7], b"CLWD1\x12\x07");
    let palette = data[9..9 + usize::from(data[7]) * 3]
        .chunks_exact(3)
        .map(|p| Color::Rgb(p[0], p[1], p[2]))
        .collect();
    let mut cursor = 9 + usize::from(data[7]) * 3;
    let mut clips = Vec::new();
    for _ in 0..data[8] {
        let len = usize::from(data[cursor]);
        cursor += 1;
        let name = std::str::from_utf8(&data[cursor..cursor + len]).unwrap();
        cursor += len;
        let frames = usize::from(u16::from_le_bytes([data[cursor], data[cursor + 1]]));
        cursor += 2;
        let end = cursor + frames * CELLS * 3;
        let frames = &data[cursor..end];
        let top_padding = frames
            .chunks_exact(CELLS * 3)
            .flat_map(|frame| {
                frame
                    .chunks_exact(3)
                    .enumerate()
                    .filter(|(_, c)| c[0] != 0 || c[2] != 0)
                    .map(|(i, _)| i / WIDTH as usize)
            })
            .min()
            .unwrap_or(0)
            .min(2);
        clips.push(Clip {
            name,
            frames,
            top_padding,
        });
        cursor = end;
    }
    assert_eq!(cursor, data.len());
    Sprites { palette, clips }
});

#[derive(Clone, Copy)]
enum Motion {
    Walking,
    Acting(usize),
    Turning,
}
pub struct Mascot {
    last: Option<f64>,
    phase: f64,
    x: f64,
    right: bool,
    motion: Motion,
    walk_for: f64,
    hold: f64,
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
        Self {
            last: None,
            phase: 0.0,
            x: 0.0,
            right: true,
            motion: Motion::Walking,
            walk_for: 3.0,
            hold: 0.0,
            random: 0,
            recent: Vec::new(),
            palette: SPRITES
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
        let choices: Vec<_> = SPRITES
            .clips
            .iter()
            .enumerate()
            .filter(|(i, c)| !matches!(c.name, "walking" | "turning") && !self.recent.contains(i))
            .map(|(i, _)| i)
            .collect();
        let index = choices[(self.random() * choices.len() as f64) as usize];
        self.recent.push(index);
        if self.recent.len() > 5 {
            self.recent.remove(0);
        }
        self.hold = match SPRITES.clips[index].name {
            "coffee" => 4.5,
            "watering" => 5.0,
            "kite" => 38.0 + self.random() * 10.0,
            _ => 0.0,
        };
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
                if self.phase >= SPRITES.clips[index].len() as f64 / FPS + self.hold {
                    self.walk();
                }
            }
            Motion::Turning => {
                if self.phase >= 7.0 / FPS {
                    self.right = !self.right;
                    self.walk();
                }
            }
        }
        self.x = self.x.clamp(0.0, limit);
    }
    fn pose(&self) -> (&Clip, usize, bool) {
        let tick = (self.phase * FPS) as usize;
        match self.motion {
            Motion::Walking => {
                let clip = SPRITES.clips.iter().find(|c| c.name == "walking").unwrap();
                (
                    clip,
                    if tick < 4 { tick } else { 8 + (tick - 4) % 16 },
                    !self.right,
                )
            }
            Motion::Turning => {
                let clip = SPRITES.clips.iter().find(|c| c.name == "turning").unwrap();
                (
                    clip,
                    if self.right {
                        15 + tick.min(7)
                    } else {
                        22 - tick.min(7)
                    },
                    false,
                )
            }
            Motion::Acting(index) => {
                let clip = &SPRITES.clips[index];
                let start = match clip.name {
                    "coffee" => (clip.len() as f64 * 0.55).round() as usize,
                    "watering" => (clip.len() as f64 * 0.60).round() as usize,
                    "kite" => 6,
                    _ => 0,
                };
                let hold_ticks = (self.hold * FPS) as usize;
                let frame = if tick < start {
                    tick
                } else if tick < start + hold_ticks {
                    if clip.name == "kite" {
                        6 + (tick - start) % 84
                    } else {
                        start
                    }
                } else {
                    tick - hold_ticks
                };
                (clip, frame, !self.right)
            }
        }
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
        let (clip, index, mirror) = self.pose();
        let x = area.x + 1 + self.x as u16;
        let y = area.bottom() - HEIGHT;
        let mut painted = Vec::new();
        for (i, sample) in clip.frame(index).chunks_exact(3).enumerate() {
            let [mut mask, fg, bg] = <[u8; 3]>::try_from(sample).unwrap();
            if mask == 0 && bg == 0 {
                continue;
            }
            let col = i as u16 % WIDTH;
            let col = if mirror {
                mask = ((mask & 0b0101) << 1) | ((mask & 0b1010) >> 1);
                WIDTH - 1 - col
            } else {
                col
            };
            let row = i / WIDTH as usize;
            if row < clip.top_padding {
                continue;
            }
            let position = (x + col, y + (row - clip.top_padding) as u16);
            if protected.iter().any(|r| r.contains(position.into())) {
                continue;
            }
            let cell = &mut frame.buffer_mut()[position];
            let background = if bg == 0 {
                cell.bg
            } else {
                self.palette[usize::from(bg)]
            };
            cell.reset();
            cell.set_symbol(QUADRANTS[usize::from(mask)])
                .set_fg(self.palette[usize::from(fg)])
                .set_bg(background);
            painted.push(Rect::new(position.0, position.1, 1, 1));
        }
        painted
    }
}
