//! A read-only Clawd decoration. No agent commands, input targets or persisted state.
use crate::agents::Status;
use ratatui::{Frame, layout::Rect, style::Color};

pub const HEIGHT: u16 = 2;
const WIDTH: usize = 7;
pub const MIN_WIDTH: u16 = WIDTH as u16 + 4;
// Reference: Claude Code's Clawd sticker. Pixel RGB, not the terminal's ANSI orange.
const CLAY: Color = Color::Rgb(0xd9, 0x77, 0x57);
const EYES: Color = Color::Rgb(0, 0, 0);
const REST: [&str; 4] = [" ##### ", "#o###o#", "#######", "# # # #"];

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
        if status == Status::Idle {
            // Slow stroll with a short pause every six seconds.
            if self.phase % 6.0 < 5.0 {
                self.x += if self.right { dt * 3.0 } else { -dt * 3.0 };
            }
            if self.x >= limit {
                self.x = limit;
                self.right = false;
            }
            if self.x <= 0.0 {
                self.x = 0.0;
                self.right = true;
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
            for (col, (&top, &bottom)) in pair[0].iter().zip(&pair[1]).enumerate() {
                if top == b' ' && bottom == b' ' {
                    continue;
                }
                let color = |p| if p == b'o' { EYES } else { self.clay };
                let cell = &mut frame.buffer_mut()[(x + col as u16, y + row as u16)];
                if top == b' ' {
                    cell.set_symbol("▄").set_fg(color(bottom));
                } else if bottom == b' ' {
                    cell.set_symbol("▀").set_fg(color(top));
                } else {
                    cell.set_symbol("▀")
                        .set_fg(color(top))
                        .set_bg(color(bottom));
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

fn pixels(status: Status, frame: u64, right: bool) -> [[u8; WIDTH]; 4] {
    let mut p = REST.map(|row| row.as_bytes().try_into().unwrap());
    match status {
        Status::Idle => {
            // Lift alternate short feet vertically; never splay them sideways.
            // The last second of each stroll is a pause with all feet grounded.
            if frame % 36 < 30 {
                p[3] = if frame % 4 < 2 {
                    *b"#   #  "
                } else {
                    *b"  #   #"
                };
            }
            if frame % 36 == 33 {
                p[1][1] = b'#';
                p[1][5] = b'#';
            }
        }
        Status::Working => {
            let arm = if frame.is_multiple_of(2) { 0 } else { 6 };
            p[0][arm] = b'#';
            p[1][arm] = b' ';
        }
        Status::Waiting => {
            p[0][6] = if frame % 8 < 4 { b'#' } else { b' ' };
            p[1][6] = b'#';
        }
        Status::Starting => {
            if frame % 8 < 4 {
                p[0][0] = b'#';
            }
        }
        Status::Exited => {
            p[1][1] = b'#';
            p[1][5] = b'#';
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
        assert!(m.x > 0.0 && m.x < 4.0 && !m.right);
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
    fn frames_keep_clawd_eyes_silhouette_and_four_legs() {
        assert_eq!(
            pixels(Status::Unknown, 0, true),
            REST.map(|r| <[u8; WIDTH]>::try_from(r.as_bytes()).unwrap())
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
            for n in 0..16 {
                let p = pixels(status, n, true);
                assert_eq!(p[1][1], b'o');
                assert_eq!(p[1][5], b'o');
                assert_eq!(
                    p[3].iter().filter(|&&v| v == b'#').count(),
                    if status == Status::Idle { 2 } else { 4 }
                );
                assert!([1, 3, 5].iter().all(|&x| p[3][x] == b' '));
                assert!(p.iter().flatten().all(|&v| [b' ', b'#', b'o'].contains(&v)));
            }
        }
        assert_eq!(pixels(Status::Idle, 30, true)[3], *b"# # # #");
        assert_eq!(
            pixels(Status::Idle, 0, true)[..3],
            pixels(Status::Idle, 2, true)[..3]
        );
        assert_ne!(pixels(Status::Idle, 0, true), pixels(Status::Idle, 2, true));
        assert_ne!(
            pixels(Status::Working, 0, true),
            pixels(Status::Working, 1, true)
        );
        assert_ne!(
            pixels(Status::Waiting, 0, true),
            pixels(Status::Waiting, 4, true)
        );
        assert_eq!(
            pixels(Status::Error, 0, true),
            pixels(Status::Error, 8, true)
        );
        assert_eq!(Mascot::default().clay, Color::Rgb(217, 119, 87));
        assert!(matches!(Mascot::new(false).clay, Color::Indexed(_)));
    }
}
