//! Looking back through a pane's retained output: scrolling, search and selection.
//! The view position and selection live in the terminal grid, so they follow output that
//! arrives while the history is open instead of being pulled back to the bottom.
use crate::terminal::Screen;
use alacritty_terminal::{
    grid::{Dimensions, Scroll},
    index::{Boundary, Column, Direction, Line, Point, Side},
    selection::{Selection, SelectionType},
    term::search::RegexSearch,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Default)]
pub struct History {
    /// The search being typed after `/`.
    pub input: Option<String>,
    /// The last confirmed search, repeated by `n`／`N`.
    pub query: String,
    /// A short result of the last search or copy.
    pub status: String,
    /// The pressed cell of a mouse drag.
    anchor: Option<Point>,
}
enum Find {
    First,
    Older,
    Newer,
}
impl Screen {
    pub fn enter_history(&mut self) {
        self.term.selection = None;
        self.history = Some(History::default());
    }
    pub fn exit_history(&mut self) {
        self.term.selection = None;
        self.term.scroll_display(Scroll::Bottom);
        self.history = None;
    }
    pub fn in_history(&self) -> bool {
        self.history.is_some()
    }
    pub fn history(&self) -> Option<&History> {
        self.history.as_ref()
    }
    pub fn history_status(&mut self, status: String) {
        if let Some(history) = &mut self.history {
            history.status = status;
        }
    }
    /// Keys of the history view; nothing reaches the terminal's program.
    pub fn history_key(&mut self, key: KeyEvent) {
        let Some(history) = &mut self.history else {
            return;
        };
        let plain = !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        if let Some(input) = &mut history.input {
            match key.code {
                KeyCode::Esc => history.input = None,
                KeyCode::Enter => {
                    let query = history.input.take().unwrap();
                    if !query.is_empty() {
                        history.query = query;
                        self.find(Find::First);
                    }
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(c) if plain => input.push(c),
                _ => {}
            }
            return;
        }
        let page = self.term.screen_lines() as i32;
        match key.code {
            KeyCode::Esc => self.exit_history(),
            KeyCode::Up => self.history_scroll(1),
            KeyCode::Down => self.history_scroll(-1),
            KeyCode::PageUp => self.history_scroll(page),
            KeyCode::PageDown => self.history_scroll(-page),
            KeyCode::Char('/') if plain => {
                history.input = Some(String::new());
                history.status.clear();
            }
            KeyCode::Char('n') if plain => self.find(Find::Older),
            KeyCode::Char('N') if plain => self.find(Find::Newer),
            _ => {}
        }
    }
    /// Scrolls `lines` towards older output (negative: newer).
    pub fn history_scroll(&mut self, lines: i32) {
        if self.history.is_some() {
            self.term.scroll_display(Scroll::Delta(lines));
        }
    }
    /// Starts (`start`) or extends a selection at a cell of the visible pane.
    pub fn history_select(&mut self, (x, y): (u16, u16), start: bool) {
        let offset = self.term.grid().display_offset() as i32;
        let point = Point::new(
            Line(i32::from(y).min(self.term.screen_lines() as i32 - 1) - offset),
            Column(usize::from(x).min(self.term.columns() - 1)),
        );
        let Some(history) = &mut self.history else {
            return;
        };
        if start {
            history.anchor = Some(point);
            self.term.selection = None;
            return;
        }
        let Some(anchor) = history.anchor else {
            return;
        };
        // Both the pressed cell and the one under the pointer are included.
        let selection = if point >= anchor {
            let mut s = Selection::new(SelectionType::Simple, anchor, Side::Left);
            s.update(point, Side::Right);
            s
        } else {
            let mut s = Selection::new(SelectionType::Simple, anchor, Side::Right);
            s.update(point, Side::Left);
            s
        };
        self.term.selection = (!selection.is_empty()).then_some(selection);
    }
    pub fn history_paste(&mut self, text: &str) {
        if let Some(input) = self.history.as_mut().and_then(|h| h.input.as_mut()) {
            input.extend(text.chars().filter(|c| !c.is_control()));
        }
    }
    pub fn selected_text(&self) -> Option<String> {
        self.term.selection_to_string().filter(|s| !s.is_empty())
    }
    /// Jumps to a match of the query and selects it: the first search looks up from the view
    /// bottom, `n` continues to older output and `N` back to newer, wrapping at either end.
    fn find(&mut self, how: Find) {
        let term = &self.term;
        let history = self.history.as_mut().unwrap();
        if history.query.is_empty() {
            return;
        }
        let Ok(mut regex) = RegexSearch::new(&literal(&history.query)) else {
            history.status = "invalid search".into();
            return;
        };
        let offset = term.grid().display_offset() as i32;
        let bottom = Point::new(
            Line(term.screen_lines() as i32 - 1 - offset),
            term.last_column(),
        );
        let top = Point::new(Line(-offset), Column(0));
        let current = term.selection.as_ref().and_then(|s| s.to_range(term));
        let found = match how {
            Find::First => term.search_next(&mut regex, bottom, Direction::Left, Side::Left, None),
            Find::Older => {
                let origin = current.map_or(bottom, |r| r.start.sub(term, Boundary::None, 1));
                term.search_next(&mut regex, origin, Direction::Left, Side::Left, None)
            }
            Find::Newer => {
                let origin = current.map_or(top, |r| r.end.add(term, Boundary::None, 1));
                term.search_next(&mut regex, origin, Direction::Right, Side::Left, None)
            }
        };
        let Some(found) = found else {
            history.status = "no match".into();
            return;
        };
        history.status.clear();
        let mut selection = Selection::new(SelectionType::Simple, *found.start(), Side::Left);
        selection.update(*found.end(), Side::Right);
        self.term.selection = Some(selection);
        self.term.scroll_to_point(*found.start());
        self.term.scroll_to_point(*found.end());
    }
}

/// The query as a literal pattern; letter case is ignored unless it has an upper-case letter.
fn literal(query: &str) -> String {
    let mut pattern = String::new();
    for c in query.chars() {
        if "\\.+*?()|[]{}^$#&-~".contains(c) {
            pattern.push('\\');
        }
        pattern.push(c);
    }
    pattern
}
