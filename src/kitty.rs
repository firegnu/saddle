//! Pictures through the Kitty graphics protocol, for terminals that answer its query (Ghostty,
//! kitty, WezTerm and others). Anything else keeps the pet drawn with glyphs.
use crate::mascot::Sprite;
use base64::Engine;
use std::{
    collections::HashMap,
    io::{self, Read, Write},
    os::unix::fs::OpenOptionsExt,
    time::{Duration, Instant},
};

/// A one-pixel graphics query, the cell size in pixels, and the primary device attributes,
/// which every terminal answers, after the others.
const QUERY: &[u8] = b"\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\\x1b[16t\x1b[c";
/// Picture ids start here, clear of the query's id.
const FIRST_ID: u32 = 0x5add_0000;

/// The cell size in pixels, when the terminal can show pictures. Call it in raw mode before
/// anything else reads input.
pub fn probe(timeout: Duration) -> Option<(u16, u16)> {
    // macOS cannot poll a terminal device, so the reply is read without blocking instead.
    let mut tty = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/tty")
        .ok()?;
    let mut out = io::stdout();
    out.write_all(QUERY).ok()?;
    out.flush().ok()?;
    match listen(&mut tty, Instant::now() + timeout)? {
        (true, cell) => cell_size().or(cell),
        (false, _) => None,
    }
}
/// The answer read from a non-blocking `tty`, unless it is incomplete at `deadline`.
fn listen(tty: &mut impl Read, deadline: Instant) -> Option<(bool, Option<(u16, u16)>)> {
    let mut reply = Vec::new();
    loop {
        if let Some(found) = answer(&reply) {
            return Some(found);
        }
        if Instant::now() >= deadline {
            return None;
        }
        let mut chunk = [0; 256];
        match tty.read(&mut chunk) {
            Ok(0) => return None,
            Ok(n) => reply.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(_) => return None,
        }
    }
}

/// Whether the terminal accepted the graphics query and the cell size it reported, once the
/// device attributes have arrived after them.
fn answer(reply: &[u8]) -> Option<(bool, Option<(u16, u16)>)> {
    let start = find(reply, b"\x1b[?")?;
    let rest = &reply[start + 3..];
    let end = rest
        .iter()
        .position(|b| !b.is_ascii_digit() && *b != b';')?;
    if rest[end] != b'c' {
        return None;
    }
    let before = &reply[..start];
    let images = find(before, b"\x1b_Gi=31;OK\x1b\\").is_some();
    let cell = find(before, b"\x1b[6;").and_then(|at| {
        let text = std::str::from_utf8(&before[at + 4..]).ok()?;
        let (height, rest) = text.split_once(';')?;
        let (width, _) = rest.split_once('t')?;
        Some((width.parse().ok()?, height.parse().ok()?)).filter(|&(w, h)| w > 0 && h > 0)
    });
    Some((images, cell))
}
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// The size of a cell in pixels, from the window size the terminal reports.
pub fn cell_size() -> Option<(u16, u16)> {
    let size = crossterm::terminal::window_size().ok()?;
    (size.width > 0 && size.height > 0 && size.columns > 0 && size.rows > 0)
        .then(|| (size.width / size.columns, size.height / size.rows))
        .filter(|&(w, h)| w > 0 && h > 0)
}

/// Where a picture is placed: its id, cell and pixel offset in that cell.
type Spot = (u32, (u16, u16), (u16, u16));
/// The pictures sent to the terminal, and the one on screen.
#[derive(Default)]
pub struct Images {
    ids: HashMap<(usize, usize, bool), u32>,
    /// The size each picture was last sent at, while the terminal is known to keep it.
    sent: HashMap<u32, (u16, u16)>,
    placed: Option<Spot>,
    screen: (u16, u16),
}
impl Images {
    /// Shows `sprite` in place of the picture before it, or nothing; a picture's pixels are
    /// sent the first time it is shown. Only changes are written.
    pub fn show(
        &mut self,
        out: &mut impl Write,
        screen: (u16, u16),
        sprite: Option<Sprite>,
        pixels: impl FnOnce(&Sprite) -> Vec<u8>,
    ) -> io::Result<()> {
        let mut bytes = Vec::new();
        if screen != self.screen {
            // A resize clears the screen, which can take pictures and placements with it.
            // Sending again under the same id replaces a picture rather than adding one.
            self.screen = screen;
            self.sent.clear();
            if let Some((id, ..)) = self.placed.take() {
                remove(&mut bytes, id);
            }
        }
        match sprite {
            None => {
                if let Some((id, ..)) = self.placed.take() {
                    remove(&mut bytes, id);
                }
            }
            Some(sprite) => {
                let next = FIRST_ID + self.ids.len() as u32;
                let id = *self.ids.entry(sprite.key).or_insert(next);
                if self.sent.get(&id) != Some(&sprite.size) {
                    transmit(&mut bytes, id, sprite.size, &pixels(&sprite))?;
                    self.sent.insert(id, sprite.size);
                }
                let spot = (id, sprite.cell, sprite.offset);
                if self.placed != Some(spot) {
                    // Place the new picture before taking the old one away, so nothing flickers.
                    place(&mut bytes, spot);
                    if let Some((old, ..)) = self.placed.filter(|placed| placed.0 != id) {
                        remove(&mut bytes, old);
                    }
                    self.placed = Some(spot);
                }
            }
        }
        if !bytes.is_empty() {
            out.write_all(&bytes)?;
            out.flush()?;
        }
        Ok(())
    }
    /// Frees every picture sent, before leaving the screen.
    pub fn free(&mut self, out: &mut impl Write) -> io::Result<()> {
        let mut bytes = Vec::new();
        for id in self.ids.values() {
            bytes.extend(format!("\x1b_Ga=d,d=I,i={id},q=2\x1b\\").bytes());
        }
        self.ids.clear();
        self.sent.clear();
        self.placed = None;
        out.write_all(&bytes)?;
        out.flush()
    }
}
/// RGBA pixels, compressed and sent in chunks; `q=2` keeps the terminal from answering.
fn transmit(
    out: &mut Vec<u8>,
    id: u32,
    (width, height): (u16, u16),
    rgba: &[u8],
) -> io::Result<()> {
    let mut zlib = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    zlib.write_all(rgba)?;
    let data = base64::engine::general_purpose::STANDARD.encode(zlib.finish()?);
    let chunks: Vec<_> = data.as_bytes().chunks(4096).collect();
    for (i, chunk) in chunks.iter().enumerate() {
        let more = u8::from(i + 1 < chunks.len());
        if i == 0 {
            write!(
                out,
                "\x1b_Ga=t,i={id},f=32,o=z,s={width},v={height},q=2,m={more};"
            )?;
        } else {
            write!(out, "\x1b_Gm={more};")?;
        }
        out.extend_from_slice(chunk);
        out.extend_from_slice(b"\x1b\\");
    }
    Ok(())
}
/// Places the picture below the text (`z=-1`) without moving the cursor, which is saved and
/// restored around the move.
fn place(out: &mut Vec<u8>, (id, (x, y), (dx, dy)): Spot) {
    let at = format!("\x1b7\x1b[{};{}H", y + 1, x + 1);
    out.extend(at.bytes());
    out.extend(format!("\x1b_Ga=p,i={id},p=1,X={dx},Y={dy},C=1,z=-1,q=2\x1b\\\x1b8").bytes());
}
/// Takes a picture's placements off the screen, keeping its pixels for later.
fn remove(out: &mut Vec<u8>, id: u32) {
    out.extend(format!("\x1b_Ga=d,d=i,i={id},q=2\x1b\\").bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn the_answer_waits_for_the_device_attributes() {
        let ok = b"\x1b_Gi=31;OK\x1b\\\x1b[6;19;8t";
        assert_eq!(answer(ok), None, "the attributes have not arrived yet");
        assert_eq!(answer(b"\x1b_Gi=31;OK\x1b\\\x1b[?62;"), None);
        let full = [&ok[..], b"\x1b[?62;22;52c"].concat();
        assert_eq!(answer(&full), Some((true, Some((8, 19)))));
        // A terminal without the protocol answers only the attributes, or an error.
        assert_eq!(answer(b"\x1b[?1;2c"), Some((false, None)));
        let refused = b"\x1b_Gi=31;ENOTSUPPORTED:no\x1b\\\x1b[?62c";
        assert_eq!(answer(refused), Some((false, None)));
        let typed = b"j\x1b_Gi=31;OK\x1b\\k\x1b[?1;2c";
        assert_eq!(
            answer(typed),
            Some((true, None)),
            "keys typed meanwhile are skipped"
        );
    }

    /// A terminal that has said `reply` so far and then stays silent.
    struct Silent(Vec<u8>);
    impl Read for Silent {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.0.is_empty() {
                return Err(io::ErrorKind::WouldBlock.into());
            }
            let n = buf.len().min(self.0.len());
            buf[..n].copy_from_slice(&self.0.drain(..n).collect::<Vec<_>>());
            Ok(n)
        }
    }

    #[test]
    fn a_terminal_that_never_finishes_answering_is_given_up_on() {
        let start = Instant::now();
        let deadline = start + Duration::from_millis(100);
        let partial = b"\x1b_Gi=31;OK\x1b\\".to_vec();
        assert_eq!(listen(&mut Silent(partial), deadline), None);
        assert!(start.elapsed() < Duration::from_secs(1));
        let full = b"\x1b_Gi=31;OK\x1b\\\x1b[6;19;8t\x1b[?62c".to_vec();
        let later = Instant::now() + Duration::from_millis(100);
        assert_eq!(
            listen(&mut Silent(full), later),
            Some((true, Some((8, 19))))
        );
    }

    fn sprite(pose: usize, cell: (u16, u16)) -> Sprite {
        Sprite {
            key: (1, pose, false),
            cell,
            offset: (3, 0),
            size: (2, 1),
        }
    }
    fn show(images: &mut Images, screen: (u16, u16), sprite: Option<Sprite>) -> String {
        let mut out = Vec::new();
        images
            .show(&mut out, screen, sprite, |_| {
                vec![255, 0, 0, 255, 0, 0, 0, 0]
            })
            .unwrap();
        String::from_utf8(out).unwrap()
    }
    const FIRST: &str = "1524432896";
    const SECOND: &str = "1524432897";

    #[test]
    fn a_picture_is_sent_once_and_only_changes_are_written() {
        let mut images = Images::default();
        let first = show(&mut images, (80, 24), Some(sprite(0, (5, 1))));
        let (send, place) = first.split_once("\x1b7").unwrap();
        let header = format!("\x1b_Ga=t,i={FIRST},f=32,o=z,s=2,v=1,q=2,m=0;");
        assert!(send.starts_with(&header), "{send:?}");
        // The pixels arrive compressed, exactly as given.
        let data = &send.as_bytes()[header.len()..send.len() - 2];
        let data = base64::engine::general_purpose::STANDARD
            .decode(data)
            .unwrap();
        let mut pixels = Vec::new();
        flate2::read::ZlibDecoder::new(&data[..])
            .read_to_end(&mut pixels)
            .unwrap();
        assert_eq!(pixels, [255, 0, 0, 255, 0, 0, 0, 0]);
        assert_eq!(
            place,
            format!("\x1b[2;6H\x1b_Ga=p,i={FIRST},p=1,X=3,Y=0,C=1,z=-1,q=2\x1b\\\x1b8")
        );
        assert_eq!(show(&mut images, (80, 24), Some(sprite(0, (5, 1)))), "");
        // Moving places it again; another pose is sent, placed, then the old one taken away.
        let moved = show(&mut images, (80, 24), Some(sprite(0, (6, 1))));
        assert!(moved.starts_with("\x1b7\x1b[2;7H") && !moved.contains("a=t"));
        let next = show(&mut images, (80, 24), Some(sprite(1, (6, 1))));
        let order: Vec<_> = ["a=t", "a=p", "a=d"]
            .iter()
            .map(|k| next.find(k).unwrap())
            .collect();
        assert!(order.is_sorted(), "{next:?}");
        assert!(next.contains(&format!("i={SECOND},")));
        assert!(next.ends_with(&format!("\x1b_Ga=d,d=i,i={FIRST},q=2\x1b\\")));
        // Back to the first pose: its pixels are still there.
        let back = show(&mut images, (80, 24), Some(sprite(0, (6, 1))));
        assert!(!back.contains("a=t") && back.contains(&format!("i={FIRST},p=1")));
        assert_eq!(
            show(&mut images, (80, 24), None),
            format!("\x1b_Ga=d,d=i,i={FIRST},q=2\x1b\\")
        );
        assert_eq!(show(&mut images, (80, 24), None), "");
    }

    #[test]
    fn a_resize_sends_and_places_again_and_exit_frees_everything() {
        let mut images = Images::default();
        show(&mut images, (80, 24), Some(sprite(0, (5, 1))));
        show(&mut images, (80, 24), Some(sprite(1, (5, 1))));
        let resized = show(&mut images, (100, 30), Some(sprite(1, (5, 1))));
        assert!(resized.starts_with(&format!("\x1b_Ga=d,d=i,i={SECOND},q=2\x1b\\")));
        assert!(resized.contains(&format!("a=t,i={SECOND},")) && resized.contains("a=p"));
        let mut out = Vec::new();
        images.free(&mut out).unwrap();
        let out = String::from_utf8(out).unwrap();
        for id in [FIRST, SECOND] {
            assert!(out.contains(&format!("\x1b_Ga=d,d=I,i={id},q=2\x1b\\")));
        }
        assert_eq!(show(&mut images, (100, 30), None), "");
    }

    #[test]
    fn large_pictures_are_sent_in_chunks() {
        let mut out = Vec::new();
        let mut seed = 1u32;
        let noise: Vec<u8> = (0..40_000)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                seed as u8
            })
            .collect();
        transmit(&mut out, 9, (100, 100), &noise).unwrap();
        let out = String::from_utf8(out).unwrap();
        let chunks: Vec<_> = out.split_terminator("\x1b\\").collect();
        assert!(chunks.len() > 1);
        assert!(chunks[0].contains("m=1;"));
        assert!(
            chunks[1..chunks.len() - 1]
                .iter()
                .all(|c| c.starts_with("\x1b_Gm=1;"))
        );
        assert!(chunks.last().unwrap().starts_with("\x1b_Gm=0;"));
    }
}
