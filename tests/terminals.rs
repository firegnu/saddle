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
