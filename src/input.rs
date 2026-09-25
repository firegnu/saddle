use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Focus {
    #[default]
    Agents,
    Queue,
    Viewer,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Ignore,
    Panel,
    Terminal,
    Quit,
}
impl Focus {
    pub fn route(&mut self, key: KeyEvent) -> Route {
        if key.kind == KeyEventKind::Release {
            return Route::Ignore;
        }
        if key.code == KeyCode::Char(']') && key.modifiers.contains(KeyModifiers::CONTROL) {
            *self = Self::Agents;
            return Route::Ignore;
        }
        if *self != Self::Agents {
            return Route::Terminal;
        }
        match key.code {
            KeyCode::Tab if key.modifiers.is_empty() => {
                *self = Self::Queue;
                Route::Ignore
            }
            KeyCode::BackTab => {
                *self = Self::Viewer;
                Route::Ignore
            }
            KeyCode::Char('q') if key.modifiers.is_empty() => Route::Quit,
            _ => Route::Panel,
        }
    }
}

pub fn encode_key(key: KeyEvent, application_cursor: bool) -> Vec<u8> {
    if key.kind == KeyEventKind::Release {
        return Vec::new();
    }
    let modifiers = key.modifiers;
    let shift = modifiers.contains(KeyModifiers::SHIFT);
    let alt = modifiers.contains(KeyModifiers::ALT);
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);
    let parameter = 1 + u8::from(shift) + 2 * u8::from(alt) + 4 * u8::from(ctrl);
    let special = match key.code {
        KeyCode::Up => Some(('A', 1)),
        KeyCode::Down => Some(('B', 1)),
        KeyCode::Right => Some(('C', 1)),
        KeyCode::Left => Some(('D', 1)),
        KeyCode::Home => Some(('H', 1)),
        KeyCode::End => Some(('F', 1)),
        KeyCode::Insert => Some(('~', 2)),
        KeyCode::Delete => Some(('~', 3)),
        KeyCode::PageUp => Some(('~', 5)),
        KeyCode::PageDown => Some(('~', 6)),
        KeyCode::F(n @ 1..=4) => Some(((b'P' + n - 1) as char, 1)),
        KeyCode::F(n @ 5..=12) => Some(('~', [15, 17, 18, 19, 20, 21, 23, 24][usize::from(n - 5)])),
        _ => None,
    };
    if let Some((suffix, number)) = special {
        let text = if parameter > 1 {
            format!("\x1b[{number};{parameter}{suffix}")
        } else if suffix == '~' {
            format!("\x1b[{number}~")
        } else if application_cursor || matches!(key.code, KeyCode::F(_)) {
            format!("\x1bO{suffix}")
        } else {
            format!("\x1b[{suffix}")
        };
        return text.into_bytes();
    }
    let mut bytes = match key.code {
        KeyCode::Char(c) if ctrl => {
            let c = c.to_ascii_uppercase();
            match c {
                ' '..='_' => vec![(c as u8) & 0x1f],
                '?' => vec![0x7f],
                _ => c.to_string().into_bytes(),
            }
        }
        KeyCode::Char(c) => c.to_string().into_bytes(),
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Esc => vec![0x1b],
        _ => Vec::new(),
    };
    if alt {
        bytes.insert(0, 0x1b);
    }
    bytes
}
