use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, BorderType},
};

pub const BG: Color = Color::Reset;
pub const OVERLAY: Color = Color::Reset;
pub const SELECTED: Color = Color::DarkGray;
// Subtle warm tint matched to the current Terminal theme.
pub const AGENT_SELECTED: Color = Color::Rgb(0x2b, 0x26, 0x21);
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
// The Agents panel's own palette (design 3a); other areas keep the shared colors above.
pub const AGENTS_BG: Color = Color::Rgb(0x1d, 0x1a, 0x16);
pub const AGENTS_BORDER: Color = Color::Rgb(0x5a, 0x52, 0x47);
pub const AGENTS_RULE: Color = Color::Rgb(0x3a, 0x35, 0x2e);
pub const AGENTS_FAINT: Color = Color::Rgb(0x3d, 0x38, 0x30);
pub const AGENTS_TEXT: Color = Color::Rgb(0xe8, 0xdf, 0xcc);
pub const AGENTS_BRANCH: Color = Color::Rgb(0xcf, 0xc6, 0xb2);
pub const AGENTS_DIM: Color = Color::Rgb(0x8c, 0x83, 0x74);
pub const AGENTS_DIMMER: Color = Color::Rgb(0x75, 0x6c, 0x5e);
pub const AGENTS_ACCENT: Color = Color::Rgb(0xbd, 0xb8, 0x6a);
pub const AGENTS_GREEN: Color = Color::Rgb(0xa8, 0xc4, 0x7c);
pub const AGENTS_RED: Color = Color::Rgb(0xe3, 0x72, 0x64);
pub const AGENTS_BLUE: Color = Color::Rgb(0x7f, 0xa9, 0xea);
pub const AGENTS_YELLOW: Color = Color::Rgb(0xd9, 0xb2, 0x5f);
pub const AGENTS_PURPLE: Color = Color::Rgb(0xa5, 0x8b, 0xdc);

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
    pub agents_bg: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agents_border: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agents_rule: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agents_faint: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agents_text: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agents_branch: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agents_dim: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agents_dimmer: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agents_accent: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agents_green: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agents_red: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agents_blue: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agents_yellow: Color,
    #[serde(deserialize_with = "deserialize_color")]
    pub agents_purple: Color,
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
            agents_bg: AGENTS_BG,
            agents_border: AGENTS_BORDER,
            agents_rule: AGENTS_RULE,
            agents_faint: AGENTS_FAINT,
            agents_text: AGENTS_TEXT,
            agents_branch: AGENTS_BRANCH,
            agents_dim: AGENTS_DIM,
            agents_dimmer: AGENTS_DIMMER,
            agents_accent: AGENTS_ACCENT,
            agents_green: AGENTS_GREEN,
            agents_red: AGENTS_RED,
            agents_blue: AGENTS_BLUE,
            agents_yellow: AGENTS_YELLOW,
            agents_purple: AGENTS_PURPLE,
            claude: Color::Rgb(0xe2, 0x83, 0x5e),
            codex: Color::Rgb(0x79, 0xd4, 0xb4),
            pi: Color::Rgb(0xff, 0xff, 0xff),
            omp: Color::Rgb(0xa8, 0x55, 0xf7),
        }
    }
}

/// Whether the terminal announces 24-bit color (`COLORTERM`).
pub fn truecolor(colorterm: Option<&str>) -> bool {
    colorterm.is_some_and(|value| {
        value.eq_ignore_ascii_case("truecolor") || value.eq_ignore_ascii_case("24bit")
    })
}
/// The nearest xterm 256-color entry for an RGB color, from the 6×6×6 cube or the gray ramp;
/// other colors are kept.
pub fn nearest_256(color: Color) -> Color {
    let Color::Rgb(r, g, b) = color else {
        return color;
    };
    const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    let level = |v: u8| (0..6).min_by_key(|&i| LEVELS[i].abs_diff(v)).unwrap();
    let distance = |(x, y, z): (u8, u8, u8)| {
        [(x, r), (y, g), (z, b)]
            .iter()
            .map(|&(a, b)| u32::from(a.abs_diff(b)).pow(2))
            .sum::<u32>()
    };
    let (ri, gi, bi) = (level(r), level(g), level(b));
    let cube = (LEVELS[ri], LEVELS[gi], LEVELS[bi]);
    let average = (u32::from(r) + u32::from(g) + u32::from(b)) / 3;
    let step = ((average.saturating_sub(8) + 5) / 10).min(23) as u8;
    let gray = 8 + 10 * step;
    if distance((gray, gray, gray)) < distance(cube) {
        Color::Indexed(232 + step)
    } else {
        Color::Indexed(16 + 36 * ri as u8 + 6 * gi as u8 + bi as u8)
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
    /// Without 24-bit color, the Agents-only colors take their nearest 256-color entries;
    /// colors shared with other areas, and ANSI names, are left as configured.
    pub fn for_terminal(mut self, truecolor: bool) -> Self {
        if truecolor {
            return self;
        }
        for color in [
            &mut self.agents_bg,
            &mut self.agents_border,
            &mut self.agents_rule,
            &mut self.agents_faint,
            &mut self.agents_text,
            &mut self.agents_branch,
            &mut self.agents_dim,
            &mut self.agents_dimmer,
            &mut self.agents_accent,
            &mut self.agents_green,
            &mut self.agents_red,
            &mut self.agents_blue,
            &mut self.agents_yellow,
            &mut self.agents_purple,
            &mut self.agent_selected,
            &mut self.claude,
            &mut self.codex,
            &mut self.pi,
            &mut self.omp,
        ] {
            *color = nearest_256(*color);
        }
        self
    }
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
