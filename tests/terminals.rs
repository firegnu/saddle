use ratatui::{Terminal, backend::TestBackend, layout::Rect};
use saddle::{
    terminals::{Place, Terminals},
    theme::Theme,
};

#[test]
fn four_directions_split_relative_to_the_active_pane_and_close_collapses_it() {
    for (place, old_rect, new_rect) in [
        (
            Place::Left,
            Rect::new(25, 1, 25, 20),
            Rect::new(0, 1, 25, 20),
        ),
        (
            Place::Right,
            Rect::new(0, 1, 25, 20),
            Rect::new(25, 1, 25, 20),
        ),
        (Place::Up, Rect::new(0, 11, 50, 10), Rect::new(0, 1, 50, 10)),
        (
            Place::Down,
            Rect::new(0, 1, 50, 10),
            Rect::new(0, 11, 50, 10),
        ),
    ] {
        let mut terminals = Terminals::new("unused-fake-corral".into());
        let old = terminals.active_pane().id;
        let ticket = terminals.reserve(place, None);
        let rects = terminals.rects(Rect::new(0, 0, 50, 21));
        assert_eq!(rects.iter().find(|(id, _)| *id == old).unwrap().1, old_rect);
        assert_eq!(
            rects.iter().find(|(id, _)| *id == ticket.pane).unwrap().1,
            new_rect
        );
        terminals.close_pane(ticket.pane).unwrap();
        assert!(!terminals.valid(ticket));
        assert_eq!(
            terminals.rects(Rect::new(0, 0, 50, 21)),
            vec![(old, Rect::new(0, 1, 50, 20))]
        );
    }
}

#[test]
fn nested_layout_and_controls_stay_inside_small_screens() {
    let mut terminals = Terminals::new("unused-fake-corral".into());
    terminals.reserve(Place::Right, None);
    terminals.reserve(Place::Down, None);
    let active = terminals.active_pane().id;
    for (width, height) in [
        (160, 48),
        (120, 36),
        (80, 24),
        (15, 6),
        (5, 3),
        (1, 1),
        (0, 0),
    ] {
        let area = Rect::new(0, 0, width, height);
        let rects = terminals.rects(area);
        assert!(rects.iter().any(|(id, _)| *id == active));
        for (_, rect) in &rects {
            assert_eq!(rect.intersection(area), *rect);
        }
        let mut screen = Terminal::new(TestBackend::new(width, height)).unwrap();
        screen
            .draw(|frame| {
                let hits =
                    saddle::terminals::draw(&Theme::default(), frame, area, &terminals, true);
                for (hit, _) in hits {
                    assert_eq!(hit.area.intersection(area), hit.area);
                }
            })
            .unwrap();
    }
}
