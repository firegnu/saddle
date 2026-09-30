//! A read-only Clawd decoration. No agent commands, input targets or persisted state.
use crate::agents::Status;
use ratatui::{Frame, layout::Rect, style::Color};

pub const HEIGHT: u16 = 3;
const WIDTH: usize = 8;
pub const MIN_WIDTH: u16 = WIDTH as u16 + 4;
// Reference: Claude Code's Clawd sticker. Pixel RGB, not the terminal's ANSI orange.
const CLAY: Color = Color::Rgb(0xd9, 0x77, 0x57);
const EYES: Color = Color::Rgb(0, 0, 0);
// Two horizontal and two vertical subpixels per terminal cell.
const REST: [&str; 6] = [
    "                ",
    "  ############  ",
    "  ##o######o##  ",
    "################",
    "  ############  ",
    "  # #      # #  ",
];
const QUADRANTS: [&str; 16] = [
    " ", "▘", "▝", "▀", "▖", "▌", "▞", "▛", "▗", "▚", "▐", "▜", "▄", "▙", "▟", "█",
];

pub struct Mascot {
    target: Option<(String, Option<String>)>,
    status: Status,
    last: Option<f64>,
    phase: f64,
    x: f64,
    right: bool,
    clay: Color,
}
impl Default for Mascot {
    fn default() -> Self {
        Self::new(true)
    }
}
impl Mascot {
    pub fn new(truecolor: bool) -> Self {
        Self {
            target: None,
            status: Status::Unknown,
            last: None,
            phase: 0.0,
            x: 0.0,
            right: true,
            clay: if truecolor {
                CLAY
            } else {
                crate::theme::nearest_256(CLAY)
            },
        }
    }
    pub fn hide(&mut self) {
        self.last = None;
    }
    fn advance(&mut self, target: (&str, Option<&str>), status: Status, now: f64, limit: f64) {
        if self
            .target
            .as_ref()
            .is_none_or(|(name, instance)| name != target.0 || instance.as_deref() != target.1)
        {
            self.target = Some((target.0.to_owned(), target.1.map(str::to_owned)));
            self.x = 0.0;
            self.right = true;
            self.last = None;
            self.phase = 0.0;
        }
        if self.status != status {
            self.status = status;
            self.phase = 0.0;
            self.last = None;
        }
        // No catch-up after a hidden view, suspend, clock jump or slow frame.
        let dt = self.last.map_or(0.0, |last| (now - last).clamp(0.0, 0.25));
        self.last = Some(now);
        self.phase += dt;
        self.x = self.x.min(limit);
        if status == Status::Idle && dt > 0.0 && limit > 0.0 {
            // Three seconds strolling, four resting; ease into and out of each stroll.
            let stroll = self.phase % 7.0;
            if stroll < 3.0 {
                let speed = 3.0 * (stroll / 0.4).min(1.0).min((3.0 - stroll) / 0.4);
                self.x += if self.right { dt * speed } else { -dt * speed };
                if (self.right && self.x >= limit) || (!self.right && self.x <= 0.0) {
                    self.x = self.x.clamp(0.0, limit);
                    self.right = self.x <= 0.0;
                    // Rest at the edge before taking the first step back.
                    self.phase = 3.0;
                }
            }
        }
    }

    pub fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        target: (&str, Option<&str>),
        status: Status,
        now: f64,
    ) {
        let area = area.intersection(frame.area());
        if area.width < MIN_WIDTH || area.height < HEIGHT {
            self.hide();
            return;
        }
        let limit = f64::from(area.width - WIDTH as u16 - 4);
        self.advance(target, status, now, limit);
        let pixels = pixels(status, (self.phase * 6.0) as u64, self.right);
        let x = area.x + 1 + self.x as u16;
        let y = area.bottom() - HEIGHT;
        for (row, pair) in pixels.chunks_exact(2).enumerate() {
            for col in 0..WIDTH {
                let samples = [
                    pair[0][col * 2],
                    pair[0][col * 2 + 1],
                    pair[1][col * 2],
                    pair[1][col * 2 + 1],
                ];
                if samples.iter().all(|&p| p == b' ') {
                    continue;
                }
                let mask = samples
                    .iter()
                    .enumerate()
                    .fold(0, |mask, (bit, &p)| mask | (usize::from(p == b'#') << bit));
                let cell = &mut frame.buffer_mut()[(x + col as u16, y + row as u16)];
                cell.set_symbol(QUADRANTS[mask]).set_fg(self.clay);
                // Eye cells contain only clay and black; other cells retain the surface background.
                if samples.contains(&b'o') {
                    cell.set_bg(EYES);
                }
            }
        }
        let mark = match status {
            Status::Waiting => Some("?"),
            Status::Error | Status::Stalled => Some("!"),
            _ => None,
        };
        if let Some(mark) = mark {
            frame.buffer_mut()[(x + WIDTH as u16 + 1, y)]
                .set_symbol(mark)
                .set_fg(self.clay);
        }
    }
}

fn pixels(status: Status, frame: u64, right: bool) -> [[u8; WIDTH * 2]; 6] {
    let mut p = REST.map(|row| row.as_bytes().try_into().unwrap());
    match status {
        Status::Idle => {
            // A half-cell shuffle of the inner feet; all four thin legs stay visible.
            if frame % 42 < 18 && frame % 4 >= 2 {
                p[5] = *b"  #  #    #  #  ";
            }
            if frame % 42 == 28 {
                p[2][4] = b'#';
                p[2][11] = b'#';
            }
        }
        Status::Working => {
            // Small, regular steps under a steady body; outer feet remain planted.
            if frame % 6 >= 3 {
                p[5] = *b"  #  #    #  #  ";
            }
            if frame % 12 >= 9 {
                let arm = if (frame / 12).is_multiple_of(2) {
                    0
                } else {
                    14
                };
                p[2][arm..arm + 2].fill(b'#');
                p[3][arm..arm + 2].fill(b' ');
            }
        }
        Status::Waiting => {
            if frame % 24 == 10 {
                p[2][4] = b'#';
                p[2][11] = b'#';
            }
            if matches!(frame % 24, 16..=17 | 20..=21) {
                p[2][14..16].fill(b'#');
                p[3][14..16].fill(b' ');
            }
        }
        Status::Starting => {
            if frame % 8 < 4 {
                p[2][0..2].fill(b'#');
                p[3][0..2].fill(b' ');
            }
        }
        Status::Exited => {
            p[2][4] = b'#';
            p[2][11] = b'#';
        }
        _ => {}
    }
    if status == Status::Idle && !right {
        for row in &mut p {
            row.reverse();
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stroll_bounces_and_state_changes_stop_in_place() {
        let mut m = Mascot::default();
        let key = ("p/a", Some("one"));
        m.advance(key, Status::Idle, 0.0, 4.0);
        for n in 1..=8 {
            m.advance(key, Status::Idle, n as f64 / 4.0, 4.0);
        }
        assert!(m.x > 0.0 && m.x <= 4.0 && !m.right);
        let stopped = m.x;
        m.advance(key, Status::Working, 2.1, 4.0);
        for n in 9..=16 {
            m.advance(key, Status::Working, n as f64 / 4.0, 4.0);
        }
        assert_eq!(m.x, stopped);
        m.advance(key, Status::Waiting, 4.1, 4.0);
        assert_eq!(m.x, stopped);
        m.hide();
        m.advance(key, Status::Idle, 900.0, 4.0);
        assert_eq!(m.x, stopped, "no movement on return from a hidden view");
        m.advance(("p/a", Some("new-instance")), Status::Idle, 901.0, 4.0);
        assert_eq!(m.x, 0.0);
        m.advance(("p/a", Some("new-instance")), Status::Idle, 900.0, 0.0);
        assert_eq!(m.x, 0.0, "resize and backwards clock remain bounded");
    }
    #[test]
    fn idle_pauses_at_the_edge_before_returning() {
        let mut m = Mascot::default();
        let key = ("p/a", Some("one"));
        m.advance(key, Status::Idle, 0.0, 4.0);
        for n in 1..=8 {
            m.advance(key, Status::Idle, n as f64 / 4.0, 4.0);
        }
        assert_eq!(m.x, 4.0, "reaching the edge starts a pause");
        for n in 9..=16 {
            m.advance(key, Status::Idle, n as f64 / 4.0, 4.0);
            assert_eq!(m.x, 4.0, "do not immediately bounce back");
        }
        for n in 17..=24 {
            m.advance(key, Status::Idle, n as f64 / 4.0, 4.0);
        }
        assert!(m.x > 0.0 && m.x < 4.0 && !m.right);
    }
    #[test]
    fn working_upper_body_and_waiting_leave_quiet_intervals() {
        let rest = pixels(Status::Unknown, 0, true);
        for (status, frames) in [(Status::Working, 24), (Status::Waiting, 48)] {
            let quiet = (0..frames)
                .filter(|&n| {
                    let p = pixels(status, n, true);
                    if status == Status::Working {
                        p[..5] == rest[..5]
                    } else {
                        p == rest
                    }
                })
                .count();
            assert!(
                quiet >= frames as usize * 3 / 4,
                "most frames should be still: {status:?}"
            );
            assert!(
                quiet < frames as usize,
                "still provide an occasional gesture"
            );
        }
    }
    #[test]
    fn working_steps_and_waiting_gestures_keep_a_stable_body() {
        let rest = pixels(Status::Unknown, 0, true);
        assert_ne!(
            pixels(Status::Working, 0, true)[5],
            pixels(Status::Working, 3, true)[5],
            "working feet must move"
        );
        assert!(
            (0..24).any(|n| pixels(Status::Waiting, n, true) != rest),
            "waiting should animate within four seconds"
        );
        for n in 0..48 {
            for status in [Status::Working, Status::Waiting] {
                let p = pixels(status, n, true);
                assert_eq!(p[1], rest[1], "head stays still");
                assert_eq!(p[4], rest[4], "body stays still");
                assert_eq!(p[5].iter().filter(|&&v| v == b'#').count(), 4);
                assert_eq!(p[5][2], b'#', "outer feet stay grounded");
                assert_eq!(p[5][13], b'#');
                if status == Status::Waiting {
                    assert_eq!(p[5], rest[5]);
                }
            }
        }
    }
    #[test]
    fn frames_keep_clawd_eyes_silhouette_and_four_legs() {
        assert_eq!(
            pixels(Status::Unknown, 0, true),
            REST.map(|r| <[u8; WIDTH * 2]>::try_from(r.as_bytes()).unwrap())
        );
        for status in [
            Status::Idle,
            Status::Working,
            Status::Waiting,
            Status::Starting,
            Status::Error,
            Status::Stalled,
            Status::Unknown,
        ] {
            for n in 0..96 {
                let p = pixels(status, n, true);
                let blink = (status == Status::Idle && n % 42 == 28)
                    || (status == Status::Waiting && n % 24 == 10);
                let eye = if blink { b'#' } else { b'o' };
                assert_eq!(p[2][4], eye);
                assert_eq!(p[2][11], eye);
                assert_eq!(p[5].iter().filter(|&&v| v == b'#').count(), 4);
                assert!(
                    !p[5].windows(2).any(|w| w == b"##"),
                    "legs stay thin and separate"
                );
                for pair in p.chunks_exact(2) {
                    for x in (0..WIDTH * 2).step_by(2) {
                        let samples = [pair[0][x], pair[0][x + 1], pair[1][x], pair[1][x + 1]];
                        assert!(
                            !(samples.contains(&b'o') && samples.contains(&b' ')),
                            "each cell needs at most two colors"
                        );
                    }
                }
                assert!(p.iter().flatten().all(|&v| [b' ', b'#', b'o'].contains(&v)));
            }
        }
        assert_eq!(pixels(Status::Idle, 30, true)[5], *b"  # #      # #  ");
        assert_eq!(
            pixels(Status::Idle, 0, true)[..5],
            pixels(Status::Idle, 2, true)[..5]
        );
        assert_ne!(pixels(Status::Idle, 0, true), pixels(Status::Idle, 2, true));
        assert_ne!(
            pixels(Status::Working, 0, true),
            pixels(Status::Working, 9, true)
        );
        assert_ne!(
            pixels(Status::Waiting, 0, true),
            pixels(Status::Waiting, 40, true)
        );
        assert_eq!(
            pixels(Status::Error, 0, true),
            pixels(Status::Error, 8, true)
        );
        assert_eq!(Mascot::default().clay, Color::Rgb(217, 119, 87));
        assert!(matches!(Mascot::new(false).clay, Color::Indexed(_)));
    }
}
