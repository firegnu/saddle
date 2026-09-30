use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub panel: String,
    pub size_revision: u64,
    pub frame_id: u64,
    pub cols: u16,
    pub rows_count: u16,
    pub rows: Vec<Vec<Span>>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub text: String,
    pub fg: Color,
    pub bg: Color,
    pub modifiers: Vec<Modifier>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Color {
    Default(DefaultColor),
    Indexed { indexed: u8 },
    Rgb { rgb: [u8; 3] },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefaultColor {
    #[serde(rename = "default")]
    Default,
}
impl Default for Color {
    fn default() -> Self {
        Self::Default(DefaultColor::Default)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modifier {
    Bold,
    Dim,
    Italic,
    Underlined,
    Reversed,
    CrossedOut,
}
impl Frame {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.panel == "main" && self.frame_id > 0 && self.size_revision > 0,
            "invalid frame identity"
        );
        ensure!(
            self.cols > 0
                && self.rows_count > 0
                && usize::from(self.cols) * usize::from(self.rows_count) <= MAX_CELLS,
            "frame_too_large"
        );
        ensure!(
            self.rows.len() == usize::from(self.rows_count),
            "incorrect row count"
        );
        for row in &self.rows {
            let whole: String = row.iter().map(|s| s.text.as_str()).collect();
            let boundaries: std::collections::HashSet<_> = whole
                .grapheme_indices(true)
                .map(|(i, _)| i)
                .chain(std::iter::once(whole.len()))
                .collect();
            let mut offset = 0;
            let mut width = 0;
            for span in row {
                ensure!(
                    !span.text.is_empty() && !span.text.chars().any(char::is_control),
                    "unsupported_glyph"
                );
                ensure!(
                    span.modifiers
                        .iter()
                        .enumerate()
                        .all(|(i, m)| !span.modifiers[..i].contains(m)),
                    "duplicate modifier"
                );
                ensure!(
                    boundaries.contains(&offset)
                        && boundaries.contains(&(offset + span.text.len())),
                    "split grapheme"
                );
                for g in span.text.graphemes(true) {
                    let w = g.width();
                    ensure!(g.len() <= 128 && (1..=2).contains(&w), "unsupported_glyph");
                    width += w;
                    ensure!(width <= usize::from(self.cols), "glyph crosses row edge");
                }
                offset += span.text.len();
            }
            ensure!(width == usize::from(self.cols), "incorrect row width");
        }
        Ok(())
    }
}

pub const PROFILE: &str = "saddle-grapheme-v1";
pub const MAX_LINE: usize = 4 * 1024 * 1024;
pub const MAX_CELLS: usize = 65536;
pub const MAX_PASTE: usize = 65536;
pub const MAX_QUEUE: usize = 64;
pub const MAX_QUEUE_BYTES: usize = 256 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Message {
    Request {
        id: u64,
        method: String,
        params: serde_json::Value,
    },
    Response {
        id: u64,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "present"
        )]
        result: Option<serde_json::Value>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "present"
        )]
        error: Option<Error>,
    },
    Event {
        name: String,
        data: serde_json::Value,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Error {
    pub code: String,
    pub message: String,
}
impl Message {
    pub fn request(id: u64, method: &str, params: serde_json::Value) -> Self {
        Self::Request {
            id,
            method: method.into(),
            params,
        }
    }
    pub fn event(name: &str, data: serde_json::Value) -> Self {
        Self::Event {
            name: name.into(),
            data,
        }
    }
    pub fn response(id: u64, result: serde_json::Value) -> Self {
        Self::Response {
            id,
            result: Some(result),
            error: None,
        }
    }
    pub fn error(id: u64, code: &str, message: &str) -> Self {
        Self::Response {
            id,
            result: None,
            error: Some(Error {
                code: code.into(),
                message: message.chars().take(512).collect(),
            }),
        }
    }
}
/// Decode one bounded line; unknown optional object fields are permitted by v1.
pub fn decode(bytes: &[u8]) -> Result<Message> {
    ensure!(
        bytes.len() <= MAX_LINE && bytes.last() == Some(&b'\n'),
        "invalid message length or delimiter"
    );
    let mut depth = 0u8;
    let mut string = false;
    let mut escape = false;
    for &b in bytes {
        if string {
            if escape {
                escape = false
            } else if b == b'\\' {
                escape = true
            } else if b == b'"' {
                string = false
            }
        } else {
            match b {
                b'"' => string = true,
                b'[' | b'{' => {
                    depth += 1;
                    ensure!(depth <= 32, "JSON too deep");
                }
                b']' | b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    let msg: Message = serde_json::from_slice(bytes)?;
    match &msg {
        Message::Request { id, .. } => ensure!(*id > 0, "zero request ID"),
        Message::Response { id, result, error } => ensure!(
            *id > 0 && result.is_some() != error.is_some(),
            "invalid response"
        ),
        _ => {}
    }
    Ok(msg)
}
pub fn encode(msg: &Message) -> Result<Vec<u8>> {
    struct Bounded(Vec<u8>);
    impl std::io::Write for Bounded {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            if self.0.len() + b.len() >= MAX_LINE {
                return Err(std::io::Error::other("message too large"));
            }
            self.0.extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut out = Bounded(Vec::new());
    serde_json::to_writer(&mut out, msg)?;
    out.0.push(b'\n');
    Ok(out.0)
}
pub fn read(reader: &mut impl std::io::BufRead) -> Result<Option<Message>> {
    let mut line = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            ensure!(line.is_empty(), "partial message at EOF");
            return Ok(None);
        }
        let n = available
            .iter()
            .position(|b| *b == b'\n')
            .map_or(available.len(), |i| i + 1);
        ensure!(line.len() + n <= MAX_LINE, "message too large");
        line.extend_from_slice(&available[..n]);
        reader.consume(n);
        if line.last() == Some(&b'\n') {
            return decode(&line).map(Some);
        }
    }
}
pub fn limits() -> serde_json::Value {
    serde_json::json!({"message_bytes":MAX_LINE,"cells":MAX_CELLS,"paste_bytes":MAX_PASTE,"queue_messages":MAX_QUEUE,"queue_bytes":MAX_QUEUE_BYTES})
}

fn present<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// Host and SDK features negotiated during initialize.
pub const CAPABILITIES: &[&str] = &["panel.v1", "notify.v1", "ui.entry.v1", "panel.overlay.v1"];
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    Workspace,
    Overlay,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewDeclaration {
    pub id: String,
    pub placement: Placement,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenAction {
    pub id: String,
    pub title: String,
    pub view: String,
}
