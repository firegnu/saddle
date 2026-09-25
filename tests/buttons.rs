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
    let mut terminal = Terminal::new(TestBackend::new(20, 5)).unwrap();
    terminal
        .draw(|frame| {
            let area = Rect::new(1, 1, 18, 3);
            let (_, hits) = buttons::draw(
                frame,
                area,
                &[
                    Button::new("放行 g", KeyCode::Char('g'), false),
                    Button::new("下一件 n", KeyCode::Char('n'), true),
                    Button::new("暂停 p", KeyCode::Char('p'), true),
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
