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
            Rect::new(25, 3, 25, 20),
            Rect::new(0, 3, 25, 20),
        ),
        (
            Place::Right,
            Rect::new(0, 3, 25, 20),
            Rect::new(25, 3, 25, 20),
        ),
        (Place::Up, Rect::new(0, 13, 50, 10), Rect::new(0, 3, 50, 10)),
        (
            Place::Down,
            Rect::new(0, 3, 50, 10),
            Rect::new(0, 13, 50, 10),
        ),
    ] {
        let mut terminals = Terminals::new("unused-fake-corral".into());
        let old = terminals.active_pane().id;
        let ticket = terminals.reserve(place, None);
        let rects = terminals.rects(Rect::new(0, 0, 50, 23));
        assert_eq!(rects.iter().find(|(id, _)| *id == old).unwrap().1, old_rect);
        assert_eq!(
            rects.iter().find(|(id, _)| *id == ticket.pane).unwrap().1,
            new_rect
        );
        terminals.close_pane(ticket.pane).unwrap();
        assert!(!terminals.valid(ticket));
        assert_eq!(
            terminals.rects(Rect::new(0, 0, 50, 23)),
            vec![(old, Rect::new(0, 3, 50, 20))]
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
                    saddle::terminals::draw(&Theme::default(), frame, area, &terminals, true, &[]);
                for (hit, _) in hits {
                    assert_eq!(hit.area.intersection(area), hit.area);
                }
            })
            .unwrap();
    }
}

mod common;

#[test]
fn plugin_panes_split_in_four_directions_move_and_restore_without_duplicates() {
    for (place, expected) in [
        (Place::Left, Rect::new(0, 3, 40, 40)),
        (Place::Right, Rect::new(40, 3, 40, 40)),
        (Place::Up, Rect::new(0, 3, 80, 20)),
        (Place::Down, Rect::new(0, 23, 80, 20)),
    ] {
        let mut terminals = Terminals::new("unused-corral".into());
        let anchor = terminals.active_pane().id;
        let plugin = terminals
            .place_plugin(anchor, place, "test.counter")
            .unwrap();
        assert_eq!(terminals.tabs.len(), 1);
        assert_eq!(terminals.active_pane().id, plugin);
        assert_eq!(
            terminals
                .rects(Rect::new(0, 0, 80, 43))
                .into_iter()
                .find(|(id, _)| *id == plugin)
                .unwrap()
                .1,
            expected
        );
        terminals
            .get_mut(plugin)
            .unwrap()
            .plugin
            .as_mut()
            .unwrap()
            .note = "preserved".into();
        assert_eq!(
            terminals
                .place_plugin(anchor, Place::Tab, "test.counter")
                .unwrap(),
            plugin
        );
        assert_eq!(terminals.tabs.len(), 2);
        assert_eq!(
            terminals
                .place_plugin(anchor, place, "test.counter")
                .unwrap(),
            plugin
        );
        assert_eq!(terminals.tabs.len(), 1);
        assert_eq!(
            terminals.active_pane().plugin.as_ref().unwrap().note,
            "preserved"
        );
        // Moving within the same split must retain both leaves and the view identity.
        assert_eq!(
            terminals
                .place_plugin(anchor, place, "test.counter")
                .unwrap(),
            plugin
        );
        assert_eq!(terminals.tab().panes.len(), 2);
        let saved = terminals.snapshot();
        saved.validate().unwrap();
        let restored = Terminals::restore("unused-corral".into(), saved).unwrap();
        assert_eq!(restored.active_pane().plugin_id(), Some("test.counter"));
        assert_eq!(
            restored.rects(Rect::new(0, 0, 80, 43)),
            terminals.rects(Rect::new(0, 0, 80, 43))
        );
        terminals.close_pane(plugin).unwrap();
        assert_eq!(terminals.active_pane().id, anchor);
        assert_eq!(terminals.tab().panes.len(), 1);
    }
}

#[test]
fn invalid_plugin_placement_preserves_the_layout() {
    let mut terminals = Terminals::new("unused-corral".into());
    let plugin = terminals.open_plugin("test.counter");
    let before = serde_json::to_value(terminals.snapshot()).unwrap();
    for (anchor, place) in [
        (u64::MAX, Place::Right),
        (plugin, Place::Current),
        (plugin, Place::Right),
    ] {
        assert!(
            terminals
                .place_plugin(anchor, place, "test.counter")
                .is_err()
        );
        assert_eq!(serde_json::to_value(terminals.snapshot()).unwrap(), before);
    }
    // A plugin alone in its tab can be moved to a new tab without duplicating it.
    assert_eq!(
        terminals
            .place_plugin(plugin, Place::Tab, "test.counter")
            .unwrap(),
        plugin
    );
    assert_eq!(terminals.tabs.len(), 2);
    terminals.snapshot().validate().unwrap();
}

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
    let area = Rect::new(0, 0, 50, 23);
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
            (a.pane, Rect::new(0, 3, 25, 20)),
            (b.pane, Rect::new(25, 3, 25, 20))
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
        vec![(b.pane, Rect::new(0, 3, 50, 20))]
    );
    terminals.focus(a.pane);
    assert_eq!(
        terminals.rects(area),
        vec![(a.pane, Rect::new(0, 3, 50, 20))]
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
                        "New agent…"
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
        .draw(|frame| hits = terminals::draw(&t, frame, area, &terminals, true, &[]))
        .unwrap();
    let buffer = screen.backend().buffer();
    let row = |y: u16| -> String { (0..60).map(|x| buffer[(x, y)].symbol()).collect() };
    println!("{}\n{}\n{}\n{}", row(0), row(1), row(2), row(3));
    assert_eq!(row(1).trim_end(), "│ + │ ‹ › │p/a ×│ │p/b ×│");
    assert!(row(3).starts_with("┏"), "panes start below the strip");
    fn find(hits: &[terminals::Hit], f: impl Fn(&Control) -> bool) -> Vec<Rect> {
        hits.iter()
            .filter(|(_, c)| f(c))
            .map(|(h, _)| h.area)
            .collect()
    }
    let new = find(&hits, |c| matches!(c, Control::NewTab));
    let tabs = find(&hits, |c| matches!(c, Control::Tab(_)));
    let closes = find(&hits, |c| matches!(c, Control::CloseTab(_)));
    assert_eq!(new, vec![Rect::new(0, 0, 5, 3)]);
    assert_eq!(tabs, vec![Rect::new(10, 0, 5, 3), Rect::new(18, 0, 5, 3)]);
    assert_eq!(
        closes[..2],
        [Rect::new(15, 0, 2, 3), Rect::new(23, 0, 2, 3)]
    );
    for (tab, close) in tabs.iter().zip(&closes) {
        assert_eq!(buffer[(close.x, 1)].symbol(), "×");
        assert_eq!(buffer[(tab.x, 0)].symbol(), "╭");
        assert_eq!(buffer[(close.right() - 1, 0)].symbol(), "╮");
    }
    // The current agent has an accent boundary and bold name; the other stays dim.
    assert_eq!(buffer[(10, 0)].fg, theme::BORDER);
    assert_eq!(buffer[(18, 0)].fg, theme::FOCUS);
    assert_eq!(buffer[(19, 1)].fg, theme::BRIGHT);
    assert!(buffer[(19, 1)].modifier.contains(Modifier::BOLD));
    assert!(!buffer[(11, 1)].modifier.contains(Modifier::BOLD));
    for y in 0..3 {
        for x in 0..60 {
            assert_eq!(buffer[(x, y)].bg, Color::Reset, "no fill at {x},{y}");
        }
    }

    assert!(!row(0).contains('●'), "no decoration on the border");

    let second = terminals.active_pane().id;
    terminals.focus(terminals.tabs[0].active);
    screen
        .draw(|frame| hits = terminals::draw(&t, frame, area, &terminals, true, &[]))
        .unwrap();
    let buffer = screen.backend().buffer();
    assert_eq!(buffer[(10, 0)].fg, theme::FOCUS, "accent follows selection");
    assert_eq!(buffer[(18, 0)].fg, theme::BORDER);
    terminals.focus(second);

    // A split follows its focused agent, and long Unicode names stay inside their hit area.
    let original = terminals.active_pane().id;
    let long = terminals.reserve(Place::Right, Some("项目/非常长的开发agent名字-后缀".into()));
    screen
        .draw(|frame| hits = terminals::draw(&t, frame, area, &terminals, true, &[]))
        .unwrap();
    let buffer = screen.backend().buffer();
    let mut text = String::new();
    let mut x = 0;
    while x < 60 {
        let symbol = buffer[(x, 1)].symbol();
        text.push_str(symbol);
        x += unicode_width::UnicodeWidthStr::width(symbol).max(1) as u16;
    }
    assert!(
        text.contains("项目/非常长的开发") && text.contains("… ×│"),
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
        .draw(|frame| hits = terminals::draw(&t, frame, area, &terminals, true, &[]))
        .unwrap();
    let text: String = (0..60)
        .map(|x| screen.backend().buffer()[(x, 1)].symbol())
        .collect();
    assert!(text.contains("│p/b ×│"), "{text}");
    terminals.focus(long.pane);
    let narrow = Rect::new(0, 0, 24, 20);
    screen
        .draw(|frame| hits = terminals::draw(&t, frame, narrow, &terminals, true, &[]))
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

#[test]
fn content_picker_offers_terminal_and_new_agent_even_without_agents() {
    use saddle::placement::{self, Placement};
    let terminals = Terminals::new("unused-fake-corral".into());
    let placement = Placement {
        pane: terminals.active_pane().id,
        place: Some(Place::Tab),
        selected: 0,
        pressed: None,
    };
    let mut screen = Terminal::new(TestBackend::new(80, 24)).unwrap();
    screen
        .draw(|f| {
            placement::draw(
                &Theme::default(),
                f,
                Rect::new(0, 0, 80, 24),
                &terminals,
                &placement,
                &[],
            );
        })
        .unwrap();
    let text: String = screen
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(
        text.contains("Terminal") && text.contains("New agent…"),
        "{text}"
    );
}

#[test]
fn pending_source_cwd_survives_the_handoff_without_committing_current_metadata() {
    let mut terminals = Terminals::new("unused-fake-corral".into());
    assert_eq!(terminals.active_pane().source_cwd(), None);
    let ticket = terminals.reserve(Place::Tab, Some("p/new".into()));
    terminals.get_mut(ticket.pane).unwrap().pending_agent = saddle::viewer::AgentMetadata {
        cwd: Some("/synthetic/agent-project".into()),
        instance: Some("new-instance".into()),
    };
    let pane = terminals.get(ticket.pane).unwrap();
    assert_eq!(pane.source_cwd(), Some("/synthetic/agent-project"));
    assert!(pane.viewer.metadata.cwd.is_none());
    assert!(pane.viewer.metadata.instance.is_none());

    // Public status succeeded. complete hands the target to Viewer, before any
    // PTY exists; directory inheritance must not depend on finishing the attach.
    terminals.complete(ticket, Some("p/new".into())).unwrap();
    let pane = terminals.get(ticket.pane).unwrap();
    assert_eq!(pane.viewer.state(), "attaching");
    assert!(pane.requested().is_none());
    assert!(pane.pending_agent.cwd.is_none());
    assert!(pane.viewer.metadata.cwd.is_none());
    assert!(pane.viewer.metadata.instance.is_none());
    assert_eq!(pane.source_cwd(), Some("/synthetic/agent-project"));
}

#[test]
fn zoom_is_display_only_and_ends_when_its_pane_loses_focus_or_closes() {
    use saddle::terminals::{self, Control};
    let area = Rect::new(0, 0, 60, 23);
    let full = Rect::new(0, 3, 60, 20);
    let buttons = |terminals: &Terminals| -> Vec<String> {
        let mut screen = Terminal::new(TestBackend::new(60, 23)).unwrap();
        let mut hits = Vec::new();
        screen
            .draw(|frame| {
                hits = terminals::draw(&Theme::default(), frame, area, terminals, true, &[])
            })
            .unwrap();
        let buffer = screen.backend().buffer().clone();
        hits.iter()
            .filter(|(_, c)| matches!(c, Control::Zoom(_)))
            .map(|(h, _)| {
                (h.area.x..h.area.right())
                    .map(|x| buffer[(x, h.area.y)].symbol())
                    .collect::<String>()
            })
            .collect()
    };
    let mut terminals = Terminals::new("unused-fake-corral".into());
    let first = terminals.active_pane().id;
    assert!(buttons(&terminals).is_empty(), "a single pane has no zoom");
    terminals.toggle_zoom(first);
    assert_eq!(terminals.rects(area), vec![(first, full)]);
    let right = terminals.reserve(Place::Right, None);
    let below = terminals.reserve(Place::Down, None);
    let split = terminals.rects(area);
    assert_eq!(split.len(), 3);
    assert_eq!(buttons(&terminals), [" Zoom "]);

    terminals.toggle_zoom(right.pane);
    assert_eq!(terminals.active_pane().id, right.pane);
    assert_eq!(terminals.rects(area), vec![(right.pane, full)]);
    assert_eq!(buttons(&terminals), [" Restore "]);
    assert!(terminals.valid(below) && terminals.get(first).is_some());
    terminals.toggle_zoom(right.pane);
    assert_eq!(
        terminals.rects(area),
        split,
        "restore keeps layout and ratio"
    );
    assert_eq!(terminals.active_pane().id, right.pane);

    // Focusing another pane shows the split again, so input never goes to a hidden pane.
    terminals.toggle_zoom(right.pane);
    terminals.focus(first);
    assert_eq!(terminals.rects(area), split);
    terminals.focus(right.pane);
    assert_eq!(
        terminals.rects(area),
        split,
        "the old zoom does not come back"
    );
    // A new split beside a zoomed pane is shown with the others.
    terminals.toggle_zoom(right.pane);
    let extra = terminals.reserve(Place::Left, None);
    assert_eq!(terminals.rects(area).len(), 4);
    assert_eq!(terminals.active_pane().id, extra.pane);
    // Closing the zoomed pane leaves the remaining split; a lone pane cannot stay zoomed.
    terminals.toggle_zoom(extra.pane);
    terminals.close_pane(extra.pane).unwrap();
    assert_eq!(terminals.rects(area), split);
    terminals.toggle_zoom(below.pane);
    terminals.close_pane(right.pane).unwrap();
    terminals.close_pane(first).unwrap();
    assert_eq!(terminals.rects(area), vec![(below.pane, full)]);
    let again = terminals.reserve(Place::Right, None);
    assert_eq!(terminals.rects(area).len(), 2, "{again:?}");
    // Other tabs keep their own layout while one is zoomed.
    terminals.toggle_zoom(again.pane);
    let tab = terminals.active;
    terminals.new_tab();
    terminals.reserve(Place::Down, None);
    assert_eq!(terminals.rects(area).len(), 2);
    terminals.active = tab;
    assert_eq!(terminals.rects(area), vec![(again.pane, full)]);
}

#[test]
fn search_matches_project_or_name_and_stays_inside_small_screens() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use saddle::{
        corral::Agent,
        search::{Outcome, Search},
    };
    let agent = |name: &str, cwd: &str| Agent {
        name: name.into(),
        cwd: Some(cwd.into()),
        ..Default::default()
    };
    let agents = [
        agent("saddle/main", "/work/saddle"),
        agent("corral/Dev", "/work/corral-worktrees/t9/"),
        agent("drover/main", "/work/drover"),
    ];
    let names = |search: &Search| -> Vec<String> {
        search
            .matches(&agents)
            .iter()
            .map(|a| a.name.clone())
            .collect()
    };
    let press =
        |search: &mut Search, code| search.key(KeyEvent::new(code, KeyModifiers::NONE), &agents);
    let mut search = Search::default();
    assert_eq!(names(&search).len(), 3);
    search.paste("T9");
    assert_eq!(
        names(&search),
        ["corral/Dev"],
        "project directory, any case"
    );
    press(&mut search, KeyCode::Backspace);
    press(&mut search, KeyCode::Backspace);
    search.paste("main");
    assert_eq!(names(&search), ["drover/main", "saddle/main"]);
    press(&mut search, KeyCode::Down);
    press(&mut search, KeyCode::Down);
    assert!(matches!(press(&mut search, KeyCode::Enter), Outcome::Open(n) if n == "saddle/main"));
    search.paste("x");
    assert!(names(&search).is_empty());
    assert!(matches!(press(&mut search, KeyCode::Enter), Outcome::Stay));
    assert!(matches!(press(&mut search, KeyCode::Esc), Outcome::Cancel));
    let mut search = Search::default();
    for (width, height) in [(120, 40), (40, 10), (20, 6), (5, 3), (1, 1)] {
        let mut screen = Terminal::new(TestBackend::new(width, height)).unwrap();
        screen
            .draw(|frame| {
                let hits = search.draw(&Theme::default(), frame, &agents, |_| false);
                for hit in hits {
                    assert_eq!(hit.area.intersection(frame.area()), hit.area);
                }
            })
            .unwrap();
        let cursor = screen.backend().cursor_position();
        assert!(cursor.x < width && cursor.y < height);
    }
}

#[test]
fn history_entry_sits_in_the_pane_footer_and_switches_to_copy_and_live() {
    use saddle::{
        pty::Session,
        terminal::Size,
        terminals::{self, Control},
    };
    let area = Rect::new(0, 0, 70, 23);
    let footer = |terminals: &Terminals| -> Vec<String> {
        let mut screen = Terminal::new(TestBackend::new(70, 23)).unwrap();
        let mut hits = Vec::new();
        screen
            .draw(|frame| {
                hits = terminals::draw(&Theme::default(), frame, area, terminals, true, &[])
            })
            .unwrap();
        let buffer = screen.backend().buffer().clone();
        let mut hits: Vec<_> = hits
            .iter()
            .filter(|(_, c)| matches!(c, Control::History(_) | Control::Copy(_) | Control::Live(_)))
            .collect();
        hits.sort_by_key(|(h, _)| h.area.x);
        hits.iter()
            .map(|(h, _)| {
                (h.area.x..h.area.right())
                    .map(|x| buffer[(x, h.area.y)].symbol())
                    .collect::<String>()
            })
            .collect()
    };
    let mut terminals = Terminals::new("unused-fake-corral".into());
    assert!(
        footer(&terminals).is_empty(),
        "an empty pane has no history"
    );
    let id = terminals.active_pane().id;
    let session = Session::spawn(&["/bin/cat".into()], None, Size { rows: 20, cols: 68 }).unwrap();
    terminals.get_mut(id).unwrap().viewer.session = Some(session);
    assert_eq!(footer(&terminals), [" History "]);
    let screen = terminals
        .active_pane()
        .viewer
        .session
        .as_ref()
        .unwrap()
        .screen
        .clone();
    screen.lock().unwrap().enter_history();
    assert_eq!(footer(&terminals), [" Copy ", " Live Esc "]);
    screen.lock().unwrap().exit_history();
    assert_eq!(footer(&terminals), [" History "]);
}

#[test]
fn history_and_copy_stay_in_normal_split_and_narrow_pane_footers() {
    use saddle::{
        pty::Session,
        terminal::Size,
        terminals::{self, Control},
    };
    // The footer's history controls and the bottom border text of the active pane.
    let footer = |terminals: &Terminals, width: u16| -> (Vec<String>, String) {
        let area = Rect::new(0, 0, width, 23);
        let mut screen = Terminal::new(TestBackend::new(width, 23)).unwrap();
        let mut hits = Vec::new();
        screen
            .draw(|frame| {
                hits = terminals::draw(&Theme::default(), frame, area, terminals, true, &[])
            })
            .unwrap();
        let buffer = screen.backend().buffer().clone();
        let mut hits: Vec<_> = hits
            .iter()
            .filter(|(_, c)| matches!(c, Control::History(_) | Control::Copy(_) | Control::Live(_)))
            .collect();
        hits.sort_by_key(|(h, _)| h.area.x);
        let labels = hits
            .iter()
            .map(|(h, _)| {
                (h.area.x..h.area.right())
                    .map(|x| buffer[(x, h.area.y)].symbol())
                    .collect::<String>()
            })
            .collect();
        let rect = terminals
            .rects(area)
            .into_iter()
            .find(|(id, _)| *id == terminals.active_pane().id)
            .unwrap()
            .1;
        let border = (rect.x..rect.right())
            .map(|x| buffer[(x, rect.bottom() - 1)].symbol())
            .collect();
        (labels, border)
    };
    let attach = |terminals: &mut Terminals| {
        let id = terminals.active_pane().id;
        let session =
            Session::spawn(&["/bin/cat".into()], None, Size { rows: 18, cols: 33 }).unwrap();
        terminals.get_mut(id).unwrap().viewer.session = Some(session);
        terminals
            .active_pane()
            .viewer
            .session
            .as_ref()
            .unwrap()
            .screen
            .clone()
    };

    // A 70-column terminal area split in two gives 35-column panes.
    let mut split = Terminals::new("unused-fake-corral".into());
    split.reserve(Place::Right, None);
    let screen = attach(&mut split);
    assert_eq!(footer(&split, 70).0, [" History "], "35-column split pane");
    screen.lock().unwrap().enter_history();
    let (labels, border) = footer(&split, 70);
    assert_eq!(labels, [" Copy ", " Live Esc "], "35-column split pane");
    assert!(border.contains(" History "), "{border}");

    let mut single = Terminals::new("unused-fake-corral".into());
    let screen = attach(&mut single);
    assert_eq!(footer(&single, 50).0, [" History "]);
    screen.lock().unwrap().enter_history();
    let (labels, border) = footer(&single, 50);
    assert_eq!(labels, [" Copy ", " Live Esc "], "50-column pane");
    assert!(border.contains(" History "), "{border}");
}

#[test]
fn mascot_overlay_keeps_tabs_and_terminal_geometry() {
    use saddle::terminals::{self, Control};
    let mut panes = Terminals::new("unused-fake-corral".into());
    let id = panes.active_pane().id;
    panes.get_mut(id).unwrap().viewer.showing = Some("demo/main".into());
    let area = Rect::new(0, 0, 80, 30);
    let expected = Rect::new(0, 3, 80, 27);
    assert_eq!(
        panes.rects(area),
        vec![(id, expected)],
        "overlay must not reserve any terminal space"
    );
    let mut terminal = Terminal::new(TestBackend::new(80, 30)).unwrap();
    terminal
        .draw(|f| {
            let hits = terminals::draw(&Theme::default(), f, area, &panes, true, &[]);
            let pane = hits
                .iter()
                .find(|(_, c)| matches!(c,Control::Pane(p) if *p==id))
                .unwrap()
                .0
                .area;
            assert_eq!(pane.y, expected.y);
            assert_eq!(pane.intersection(expected), pane);
            assert!(
                hits.iter()
                    .any(|(h, c)| matches!(c, Control::NewTab) && h.area.y == 0)
            );
            let mascot = panes.mascot_area(area, &hits).unwrap();
            assert_eq!(mascot.height, 3);
            assert!(
                hits.iter()
                    .filter(|(h, _)| h.area.y < 3)
                    .all(|(h, _)| !h.area.intersects(mascot))
            );
        })
        .unwrap();
    panes.get_mut(id).unwrap().viewer.showing = None;
    assert_eq!(
        panes.rects(area),
        vec![(id, expected)],
        "empty panes keep the same decoration geometry"
    );
    assert!(panes.mascot_area(area, &[]).is_some());
    assert_eq!(panes.rects(area), vec![(id, Rect::new(0, 3, 80, 27))]);
}

#[test]
fn crowded_tabs_hide_mascot_without_changing_tab_controls_or_geometry() {
    use saddle::terminals::{self, Control};
    let mut panes = Terminals::new("unused-fake-corral".into());
    let id = panes.active_pane().id;
    panes.get_mut(id).unwrap().viewer.showing = Some("project/very-long-agent-name".into());
    let area = Rect::new(0, 0, 40, 20);
    let mut terminal = Terminal::new(TestBackend::new(40, 20)).unwrap();
    terminal
        .draw(|f| {
            let hits = terminals::draw(&Theme::default(), f, area, &panes, true, &[]);
            assert!(panes.mascot_area(area, &hits).is_none());
            assert!(hits.iter().any(|(_, c)| matches!(c, Control::Tab(_))));
        })
        .unwrap();
    assert_eq!(panes.rects(area), vec![(id, Rect::new(0, 3, 40, 17))]);
}
