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

mod common;

#[test]
fn reserving_a_new_target_cancels_the_old_pending_attach() {
    use std::{
        fs, thread,
        time::{Duration, Instant},
    };

    let temp = tempfile::tempdir().unwrap();
    let program = common::script(temp.path(), "corral", include_str!("fixtures/corral.py"));
    let mut terminals = Terminals::new(program);
    let area = Rect::new(0, 0, 80, 24);
    let events = || fs::read_to_string(temp.path().join("events")).unwrap_or_default();
    let tick_until = |terminals: &mut Terminals, done: &dyn Fn(&Terminals) -> bool| {
        let deadline = Instant::now() + Duration::from_secs(3);
        while !done(terminals) {
            assert!(Instant::now() < deadline, "timed out:\n{}", events());
            terminals.tick(area).unwrap();
            thread::sleep(Duration::from_millis(10));
        }
    };
    let a = terminals.reserve(Place::Current, Some("p/a".into()));
    terminals.complete(a, Some("p/a".into())).unwrap();
    tick_until(&mut terminals, &|t| {
        t.active_pane().viewer.showing.as_deref() == Some("p/a") && events().contains("size p/a")
    });
    fs::write(temp.path().join("hold-detach"), "").unwrap();
    let b = terminals.reserve(Place::Current, Some("p/b".into()));
    terminals.complete(b, Some("p/b".into())).unwrap(); // B's status has succeeded.
    tick_until(&mut terminals, &|_| events().contains("detaching p/a"));
    let c = terminals.reserve(Place::Current, Some("p/c".into())); // C's status is pending.
    fs::remove_file(temp.path().join("hold-detach")).unwrap();
    // A must drain to an empty Viewer, without spawning obsolete B while C awaits status.
    tick_until(&mut terminals, &|t| {
        t.active_pane().viewer.closed() || events().contains("size p/b")
    });
    let obsolete = events().contains("attach p/b");
    terminals.complete(c, Some("p/c".into())).unwrap();
    tick_until(&mut terminals, &|t| {
        t.active_pane().viewer.showing.as_deref() == Some("p/c") && events().contains("size p/c")
    });
    drop(terminals);
    assert!(
        !obsolete,
        "obsolete B was attached after reserving C:\n{}",
        events()
    );
    assert!(!events().contains("stop "));
}

#[test]
fn reserving_a_new_target_reaps_an_unreceived_spawn_even_if_status_fails() {
    use std::{
        fs, thread,
        time::{Duration, Instant},
    };

    let temp = tempfile::tempdir().unwrap();
    let program = common::script(temp.path(), "corral", include_str!("fixtures/corral.py"));
    let mut terminals = Terminals::new(program);
    let area = Rect::new(0, 0, 80, 24);
    let events = || fs::read_to_string(temp.path().join("events")).unwrap_or_default();
    let b = terminals.reserve(Place::Current, Some("p/b".into()));
    terminals.complete(b, Some("p/b".into())).unwrap();
    terminals.tick(area).unwrap(); // Starts the worker; no later tick has received it yet.
    let deadline = Instant::now() + Duration::from_secs(3);
    while !events().contains("size p/b") {
        assert!(Instant::now() < deadline, "{}", events());
        thread::sleep(Duration::from_millis(10));
    }
    let c = terminals.reserve(Place::Current, Some("p/c".into()));
    terminals.complete(c, None).unwrap(); // A failed status must not revive B's old spawn.
    let obsolete_target = terminals.find("p/b");
    while !terminals.active_pane().viewer.closed()
        && terminals.active_pane().viewer.showing.as_deref() != Some("p/b")
    {
        assert!(Instant::now() < deadline, "{}", events());
        terminals.tick(area).unwrap();
        thread::sleep(Duration::from_millis(10));
    }
    let accepts_input = terminals.active_pane().input_session().is_some();
    let reaped = terminals.active_pane().viewer.closed();
    drop(terminals);
    assert!(
        obsolete_target.is_none() && !accepts_input && reaped,
        "obsolete spawn remained a target/input session: target={obsolete_target:?}, input={accepts_input}, reaped={reaped}\n{}",
        events()
    );
    assert!(events().contains("detached p/b"));
    assert!(!events().contains("stop "));
}

#[test]
fn placing_an_open_agent_moves_its_pane_with_the_pending_request() {
    let mut terminals = Terminals::new("unused-fake-corral".into());
    let area = Rect::new(0, 0, 50, 21);
    let a = terminals.reserve(Place::Current, Some("p/a".into()));
    let b = terminals.place(a.pane, Place::Tab, "p/b").unwrap().unwrap();
    assert_eq!(terminals.tabs.len(), 2);
    // A moves beside B: same pane and ticket; its emptied tab is dropped.
    assert!(
        terminals
            .place(b.pane, Place::Left, "p/a")
            .unwrap()
            .is_none()
    );
    assert_eq!(terminals.tabs.len(), 1);
    assert!(terminals.valid(a) && terminals.valid(b));
    assert_eq!(terminals.find("p/a"), Some(a.pane));
    assert_eq!(terminals.active_pane().id, a.pane);
    assert_eq!(
        terminals.rects(area),
        vec![
            (a.pane, Rect::new(0, 1, 25, 20)),
            (b.pane, Rect::new(25, 1, 25, 20))
        ]
    );
    // A pane is never split beside itself.
    assert!(
        terminals
            .place(a.pane, Place::Right, "p/a")
            .unwrap()
            .is_none()
    );
    assert_eq!(terminals.rects(area).len(), 2);
    // Moving B into a new tab collapses the split it left.
    assert!(
        terminals
            .place(a.pane, Place::Tab, "p/b")
            .unwrap()
            .is_none()
    );
    assert_eq!(terminals.tabs.len(), 2);
    assert_eq!(terminals.active_pane().id, b.pane);
    assert_eq!(
        terminals.rects(area),
        vec![(b.pane, Rect::new(0, 1, 50, 20))]
    );
    terminals.focus(a.pane);
    assert_eq!(
        terminals.rects(area),
        vec![(a.pane, Rect::new(0, 1, 50, 20))]
    );
    assert!(terminals.valid(a) && terminals.valid(b));
}

#[test]
fn placement_popups_stay_inside_small_screens_and_show_an_empty_list() {
    use saddle::placement::{self, Placement};
    let mut terminals = Terminals::new("unused-fake-corral".into());
    terminals.reserve(Place::Right, None);
    let pane = terminals.active_pane().id;
    for place in [None, Some(Place::Right), Some(Place::Tab)] {
        let placement = Placement {
            pane,
            place,
            selected: 0,
            pressed: None,
        };
        for (width, height) in [(160, 48), (80, 24), (15, 6), (5, 3), (1, 1), (0, 0)] {
            let area = Rect::new(0, 0, width, height);
            let mut screen = Terminal::new(TestBackend::new(width, height)).unwrap();
            screen
                .draw(|frame| {
                    let hits = placement::draw(
                        &Theme::default(),
                        frame,
                        area,
                        &terminals,
                        &placement,
                        &[],
                    );
                    for (hit, _) in hits {
                        assert_eq!(hit.area.intersection(area), hit.area);
                    }
                })
                .unwrap();
            if width >= 80 {
                let buffer = screen.backend().buffer();
                let text: String = buffer.content().iter().map(|c| c.symbol()).collect();
                assert!(text.contains("‹Cancel Esc›"), "{text}");
                assert!(
                    text.contains(if place.is_none() {
                        "│ Right → │"
                    } else {
                        "No agents to open here."
                    }),
                    "{text}"
                );
            }
        }
    }
}

#[test]
fn compact_agent_tabs_and_outlined_split_sides_match_their_targets() {
    use ratatui::style::{Color, Modifier};
    use saddle::{
        placement::{self, Placement},
        terminals::{self, Control},
        theme,
    };
    let mut terminals = Terminals::new("unused-fake-corral".into());
    terminals.reserve(Place::Current, Some("p/a".into()));
    terminals.reserve(Place::Tab, Some("p/b".into()));
    let t = Theme::default();
    let area = Rect::new(0, 0, 60, 20);
    let mut screen = Terminal::new(TestBackend::new(60, 20)).unwrap();
    let mut hits = Vec::new();
    screen
        .draw(|frame| hits = terminals::draw(&t, frame, area, &terminals, true))
        .unwrap();
    let buffer = screen.backend().buffer();
    let row = |y: u16| -> String { (0..60).map(|x| buffer[(x, y)].symbol()).collect() };
    println!("{}\n{}\n{}\n{}", row(0), row(1), row(2), row(3));
    assert_eq!(row(0).trim_end(), "[+] ‹ › [p/a ×] [p/b ×]");
    assert!(row(1).starts_with("┏"), "panes start below the strip");
    fn find(hits: &[terminals::Hit], f: impl Fn(&Control) -> bool) -> Vec<Rect> {
        hits.iter()
            .filter(|(_, c)| f(c))
            .map(|(h, _)| h.area)
            .collect()
    }
    let new = find(&hits, |c| matches!(c, Control::NewTab));
    let tabs = find(&hits, |c| matches!(c, Control::Tab(_)));
    let closes = find(&hits, |c| matches!(c, Control::CloseTab(_)));
    assert_eq!(new, vec![Rect::new(0, 0, 3, 1)]);
    assert_eq!(tabs, vec![Rect::new(8, 0, 5, 1), Rect::new(16, 0, 5, 1)]);
    assert_eq!(
        closes[..2],
        [Rect::new(13, 0, 2, 1), Rect::new(21, 0, 2, 1)]
    );
    for (tab, close) in tabs.iter().zip(&closes) {
        assert_eq!(buffer[(close.x, 0)].symbol(), "×");
        assert_eq!(buffer[(tab.x, 0)].symbol(), "[");
        assert_eq!(buffer[(close.right() - 1, 0)].symbol(), "]");
    }
    // The current agent has a grey-white boundary and bold name; the other stays dim.
    assert_eq!(buffer[(8, 0)].fg, theme::BORDER);
    assert_eq!(buffer[(16, 0)].fg, theme::MUTED);
    assert_eq!(buffer[(17, 0)].fg, theme::BRIGHT);
    assert!(buffer[(17, 0)].modifier.contains(Modifier::BOLD));
    assert!(!buffer[(9, 0)].modifier.contains(Modifier::BOLD));
    for y in 0..1 {
        for x in 0..60 {
            assert_eq!(buffer[(x, y)].bg, Color::Reset, "no fill at {x},{y}");
        }
    }

    // A split follows its focused agent, and long Unicode names stay inside their hit area.
    let original = terminals.active_pane().id;
    let long = terminals.reserve(Place::Right, Some("项目/非常长的开发agent名字-后缀".into()));
    screen
        .draw(|frame| hits = terminals::draw(&t, frame, area, &terminals, true))
        .unwrap();
    let buffer = screen.backend().buffer();
    let mut text = String::new();
    let mut x = 0;
    while x < 60 {
        let symbol = buffer[(x, 0)].symbol();
        text.push_str(symbol);
        x += unicode_width::UnicodeWidthStr::width(symbol).max(1) as u16;
    }
    assert!(
        text.contains("项目/非常长的开发") && text.contains("… ×]"),
        "{text}"
    );
    assert!(!text.contains("Tab "));
    let active = hits
        .iter()
        .find(|(_, c)| matches!(c, Control::Tab(id) if *id == terminals.active))
        .unwrap()
        .0
        .area;
    assert!(active.right() <= area.right());
    terminals.focus(original);
    screen
        .draw(|frame| hits = terminals::draw(&t, frame, area, &terminals, true))
        .unwrap();
    let text: String = (0..60)
        .map(|x| screen.backend().buffer()[(x, 0)].symbol())
        .collect();
    assert!(text.contains("[p/b ×]"), "{text}");
    terminals.focus(long.pane);
    let narrow = Rect::new(0, 0, 24, 20);
    screen
        .draw(|frame| hits = terminals::draw(&t, frame, narrow, &terminals, true))
        .unwrap();
    assert!(
        hits.iter()
            .any(|(_, c)| matches!(c, Control::Tab(id) if *id == terminals.active))
    );
    assert!(
        hits.iter()
            .all(|(hit, _)| hit.area.right() <= narrow.right())
    );

    let placement = Placement {
        pane: terminals.active_pane().id,
        place: None,
        selected: 0,
        pressed: None,
    };
    screen
        .draw(|frame| hits = placement::draw(&t, frame, area, &terminals, &placement, &[]))
        .unwrap();
    let buffer = screen.backend().buffer();
    let sides = find(&hits, |c| matches!(c, Control::Side(_)));
    let cancel = find(&hits, |c| matches!(c, Control::Cancel));
    assert_eq!(sides.len(), 4);
    assert!(sides.iter().all(|s| s.height == 3));
    assert_eq!((sides[0].y, sides[1].y), (sides[2].y - 3, sides[3].y - 3));
    assert_eq!(sides[0].y, sides[1].y, "2×2 layout");
    assert_eq!(cancel[0].height, 1);
    assert_eq!(cancel[0].y, sides[2].bottom(), "Cancel sits right below");
    for s in &sides {
        assert_eq!(buffer[(s.x, s.y)].symbol(), "╭");
        assert_eq!(buffer[(s.x, s.y)].fg, theme::MUTED);
    }
}
