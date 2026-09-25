use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind as E};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};
use saddle::{
    buttons::{self, Button, Hit, Pointer},
    input::Focus,
};
fn event(kind: E, x: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column: x,
        row: 2,
        modifiers: KeyModifiers::NONE,
    }
}
#[test]
fn release_activates_only_the_original_enabled_target() {
    let hit = Hit {
        area: Rect::new(2, 2, 8, 1),
        key: crossterm::event::KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
    };
    let hits = vec![(Focus::Queue, hit.clone())];
    let mut p = Pointer::default();
    assert!(
        p.event(event(E::Down(MouseButton::Left), 3), &hits)
            .is_none()
    );
    assert!(p.event(event(E::Moved, 11), &hits).is_none());
    assert!(p.event(event(E::Up(MouseButton::Left), 3), &hits).is_none());
    p.event(event(E::Down(MouseButton::Left), 3), &hits);
    assert!(p.event(event(E::Up(MouseButton::Left), 3), &[]).is_none());
    p.event(event(E::Down(MouseButton::Left), 3), &hits);
    assert_eq!(
        p.event(event(E::Up(MouseButton::Left), 3), &hits),
        Some((Focus::Queue, hit.key))
    );
    p.event(event(E::Down(MouseButton::Left), 3), &hits);
    p.cancel();
    assert!(p.event(event(E::Up(MouseButton::Left), 3), &hits).is_none());
}
#[test]
fn disabled_buttons_have_no_target_and_wrapped_targets_stay_inside() {
    let mut terminal = Terminal::new(TestBackend::new(20, 11)).unwrap();
    terminal
        .draw(|frame| {
            let area = Rect::new(1, 1, 18, 9);
            let (_, hits) = buttons::draw(
                frame,
                area,
                &[
                    Button::new("Go g", KeyCode::Char('g'), false),
                    Button::new("Next n", KeyCode::Char('n'), true),
                    Button::new("Pause p", KeyCode::Char('p'), true),
                ],
            );
            assert_eq!(hits.len(), 2);
            for h in hits {
                assert_eq!(h.area.intersection(area), h.area);
                assert_ne!(h.key.code, KeyCode::Char('g'));
            }
        })
        .unwrap();
}

#[test]
fn outlined_buttons_use_terminal_background_in_every_pointer_state() {
    use ratatui::{layout::Position, style::Color};
    use saddle::theme;
    for compact in [false, true] {
        let mut terminal = Terminal::new(TestBackend::new(50, 8)).unwrap();
        let mut pointer = Pointer::default();
        let mut hits: Vec<(Focus, Hit)> = Vec::new();
        for phase in 0..3 {
            if phase > 0 {
                let hit = &hits[0];
                pointer.hover = Some(Position::new(hit.1.area.x, hit.1.area.y));
                if phase == 2 {
                    pointer.event(
                        MouseEvent {
                            kind: E::Down(MouseButton::Left),
                            column: hit.1.area.x,
                            row: hit.1.area.y,
                            modifiers: KeyModifiers::NONE,
                        },
                        &hits,
                    );
                }
            }
            terminal
                .draw(|frame| {
                    frame.buffer_mut().set_style(
                        Rect::new(0, 0, 50, 8),
                        ratatui::style::Style::default().bg(theme::BG),
                    );
                    let draw = if compact {
                        buttons::draw_compact
                    } else {
                        buttons::draw
                    };
                    let (content, buttons) = draw(
                        frame,
                        Rect::new(1, 1, 46, 6),
                        &[
                            Button::new("Attach ↵", KeyCode::Enter, true),
                            Button::new("Reply r", KeyCode::Char('r'), true),
                            Button::new("Sort s", KeyCode::Char('s'), true),
                            Button::new("Stop x", KeyCode::Char('x'), true).danger(),
                        ],
                    );
                    assert_eq!(content.height, if compact { 5 } else { 3 });
                    hits = buttons.into_iter().map(|h| (Focus::Agents, h)).collect();
                    pointer.paint(frame, &hits);
                })
                .unwrap();
            assert_eq!(hits.len(), 4);
            for (_, h) in &hits {
                assert_eq!(h.area.height, if compact { 1 } else { 3 });
                assert_eq!(
                    terminal.backend().buffer()[(h.area.x, h.area.y)].symbol(),
                    if compact { "‹" } else { "╭" }
                );
                for y in h.area.y..h.area.bottom() {
                    for x in h.area.x..h.area.right() {
                        assert_eq!(terminal.backend().buffer()[(x, y)].bg, Color::Reset);
                    }
                }
            }
            let corner = &terminal.backend().buffer()[(hits[0].1.area.x, hits[0].1.area.y)];
            assert_eq!(
                corner.fg,
                [theme::BORDER, theme::BRIGHT, theme::FOCUS][phase]
            );
        }
    }
}
