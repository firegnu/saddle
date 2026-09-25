use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType},
};

pub const BG: Color = Color::Reset;
pub const OVERLAY: Color = Color::Reset;
pub const SELECTED: Color = Color::DarkGray;
// Subtle warm tint matched to the current Terminal theme.
pub const AGENT_SELECTED: Color = Color::Rgb(0x30, 0x2a, 0x23);
pub const AGENT_WORKING: Color = Color::Rgb(0x7f, 0xb4, 0xee);
pub const AGENT_IDLE: Color = Color::Rgb(0x9c, 0xbd, 0x80);
pub const AGENT_BLOCKED: Color = Color::Rgb(0xe6, 0xb5, 0x66);
pub const AGENT_STALLED: Color = Color::Rgb(0xe7, 0x9b, 0x65);
pub const AGENT_ERROR: Color = Color::Rgb(0xef, 0x81, 0x74);
pub const AGENT_STARTING: Color = Color::Rgb(0xb0, 0xa1, 0xd8);
pub const BORDER: Color = Color::DarkGray;
pub const TEXT: Color = Color::Reset;
pub const BRIGHT: Color = Color::White;
pub const MUTED: Color = Color::Gray;
pub const DIM: Color = Color::DarkGray;
pub const FOCUS: Color = Color::Yellow;
pub const CONNECTED: Color = Color::Cyan;
pub const WORKING: Color = Color::Blue;
pub const BLOCKED: Color = Color::Yellow;
pub const WARNING: Color = Color::Yellow;
pub const SUCCESS: Color = Color::Green;
pub const DANGER: Color = Color::Red;
pub const UNREAD: Color = Color::Magenta;

/// Startup palette. Queue deliberately shares the Agents status accents.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Theme {
    #[serde(deserialize_with = "deserialize_color")]
    pub bg: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub overlay: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub selected: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agent_selected: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agent_working: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agent_idle: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agent_blocked: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agent_stalled: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agent_error: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agent_starting: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub border: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub text: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub bright: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub muted: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub dim: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub focus: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub connected: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub working: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub danger: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub unread: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub input_text: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub reply_code: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub reply_heading: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub claude: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub codex: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub pi: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub omp: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: BG,
            overlay: OVERLAY,
            selected: SELECTED,
            agent_selected: AGENT_SELECTED,
            agent_working: AGENT_WORKING,
            agent_idle: AGENT_IDLE,
            agent_blocked: AGENT_BLOCKED,
            agent_stalled: AGENT_STALLED,
            agent_error: AGENT_ERROR,
            agent_starting: AGENT_STARTING,
            border: BORDER,
            text: TEXT,
            bright: BRIGHT,
            muted: MUTED,
            dim: DIM,
            focus: FOCUS,
            connected: CONNECTED,
            working: WORKING,
            danger: DANGER,
            unread: UNREAD,
            input_text: Color::Black,
            reply_code: Color::Yellow,
            reply_heading: Color::Cyan,
            claude: Color::Rgb(0xd9, 0x77, 0x57),
            codex: Color::Rgb(0x8e, 0xd9, 0xc1),
            pi: Color::Rgb(0xff, 0xff, 0xff),
            omp: Color::Rgb(0xa8, 0x55, 0xf7),
        }
    }
}

fn deserialize_color<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Color, D::Error> {
    let value = <String as serde::Deserialize>::deserialize(deserializer)?;
    let color = match value.as_str() {
        "default" | "reset" => Color::Reset,
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" => Color::Gray,
        "dark_gray" => Color::DarkGray,
        "light_red" => Color::LightRed,
        "light_green" => Color::LightGreen,
        "light_yellow" => Color::LightYellow,
        "light_blue" => Color::LightBlue,
        "light_magenta" => Color::LightMagenta,
        "light_cyan" => Color::LightCyan,
        "white" => Color::White,
        _ => {
            if let Some(hex) = value.strip_prefix('#')
                && hex.len() == 6
                && hex.bytes().all(|b| b.is_ascii_hexdigit())
                && let Ok(rgb) = u32::from_str_radix(hex, 16)
            {
                return Ok(Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8));
            }
            return Err(serde::de::Error::custom(format!(
                "invalid color {value:?}: expected default, an ANSI color name, or #RRGGBB"
            )));
        }
    };
    Ok(color)
}

impl Theme {
    pub fn base(&self) -> Style {
        Style::default().fg(self.text).bg(self.bg)
    }
    pub fn block(
        &self,
        title: impl Into<ratatui::text::Line<'static>>,
        focused: bool,
    ) -> Block<'static> {
        Block::bordered()
            .title(title)
            .border_type(if focused {
                BorderType::Thick
            } else {
                BorderType::Plain
            })
            .border_style(Style::default().fg(if focused { self.focus } else { self.border }))
    }
}
pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}
