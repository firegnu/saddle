//! Synthetic draws of the host dialogs after the design-language polish.
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect, style::Modifier};
use saddle::{
    attention::{Item, Kind, Popup, Target},
    corral::Agent,
    placement::{self, Placement},
    search::Search,
    terminals::{Control, Place, Terminals},
    theme::Theme,
};

fn rows(buffer: &Buffer) -> Vec<String> {
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect()
        })
        .collect()
}
fn agent(name: &str, cwd: &str) -> Agent {
    Agent {
        name: name.into(),
        cwd: Some(cwd.into()),
        ..Default::default()
    }
}

#[test]
fn split_sides_share_one_width_and_line_up() {
    let mut terminals = Terminals::new("unused-fake-corral".into());
    let pane = terminals.active_pane().id;
    terminals.get_mut(pane).unwrap().viewer.showing =
        Some("p/一个非常长的代理名字-with-a-long-tail".into());
    let placement = Placement {
        pane,
        place: None,
        selected: 0,
        pressed: None,
    };
    let area = Rect::new(0, 0, 60, 20);
    let mut screen = Terminal::new(TestBackend::new(60, 20)).unwrap();
    let mut hits = Vec::new();
    screen
        .draw(|f| hits = placement::draw(&Theme::default(), f, area, &terminals, &placement, &[]))
        .unwrap();
    let text = rows(screen.backend().buffer());
    println!("{}", text.join("\n"));
    let sides: Vec<_> = hits
        .iter()
        .filter(|(_, c)| matches!(c, Control::Side(_)))
        .map(|(h, _)| h.area)
        .collect();
    assert_eq!(sides.len(), 4);
    assert!(sides.iter().all(|s| s.width == sides[0].width));
    assert_eq!((sides[0].x, sides[1].x), (sides[2].x, sides[3].x));
    // The long title is cut inside the frame instead of running over the corner.
    let top = text.iter().find(|r| r.contains("Split")).unwrap();
    assert!(top.contains("… ━"), "{top}");
}

#[test]
fn open_content_rules_off_fixed_actions_from_existing_agents() {
    let terminals = Terminals::new("unused-fake-corral".into());
    let placement = Placement {
        pane: terminals.active_pane().id,
        place: Some(Place::Tab),
        selected: 2,
        pressed: None,
    };
    let agents = [agent("p/alpha", "/tmp/p"), agent("p/beta", "/tmp/p")];
    let area = Rect::new(0, 0, 80, 24);
    let mut screen = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut hits = Vec::new();
    screen
        .draw(|f| {
            hits = placement::draw(&Theme::default(), f, area, &terminals, &placement, &agents)
        })
        .unwrap();
    let buffer = screen.backend().buffer();
    let text = rows(buffer);
    println!("{}", text.join("\n"));
    let picks: Vec<_> = hits
        .iter()
        .filter_map(|(h, c)| match c {
            Control::Pick(i) => Some((h.area.y, *i)),
            _ => None,
        })
        .collect();
    // Terminal, New agent…, a rule, then the agents; the rule is not pickable.
    assert_eq!(picks.iter().map(|p| p.1).collect::<Vec<_>>(), [0, 1, 2, 3]);
    assert_eq!(picks[2].0, picks[1].0 + 2);
    assert!(text[usize::from(picks[1].0 + 1)].contains("───"));
    let selected = &text[usize::from(picks[2].0)];
    assert!(selected.contains("› p/alpha"), "{selected}");
    let x = selected.find("p/alpha").unwrap() as u16;
    let x = selected[..x as usize].chars().count() as u16;
    assert!(buffer[(x, picks[2].0)].modifier.contains(Modifier::BOLD));
}

#[test]
fn unified_search_finds_a_settings_page_without_agents() {
    let mut search = Search::default();
    search.paste("cOLors");
    let mut screen = Terminal::new(TestBackend::new(80, 24)).unwrap();
    screen
        .draw(|f| {
            search.draw(&Theme::default(), f, &[], |_| false);
        })
        .unwrap();
    let text = rows(screen.backend().buffer()).join("\n");
    assert!(text.contains("Settings › Colors"), "{text}");
    assert!(!text.contains("Settings › General"), "{text}");
}

#[test]
fn search_input_drops_the_second_thick_frame() {
    let agents = [agent("p/alpha", "/tmp/demo"), agent("p/beta", "/tmp/demo")];
    let mut search = Search::default();
    let mut screen = Terminal::new(TestBackend::new(80, 24)).unwrap();
    screen
        .draw(|f| {
            search.draw(&Theme::default(), f, &agents, |_| false);
        })
        .unwrap();
    let text = rows(screen.backend().buffer());
    println!("{}", text.join("\n"));
    let input = text
        .iter()
        .find(|r| r.contains("Project, agent or settings"))
        .unwrap();
    assert!(input.contains("┌ Project, agent or settings"), "{input}");
    assert!(text.iter().any(|r| r.contains("›Agent · p/alpha  demo")));
}

#[test]
fn attention_failed_source_reads_as_not_openable_even_when_selected() {
    let items = [Item {
        target: Target::Source("corral".into()),
        kind: Kind::ReadFailed,
        label: "corral".into(),
        note: "corral ls exited 1".into(),
    }];
    let t = Theme::default();
    let mut popup = Popup::default();
    let mut screen = Terminal::new(TestBackend::new(80, 24)).unwrap();
    screen
        .draw(|f| {
            popup.draw(&t, f, &items, false);
        })
        .unwrap();
    let buffer = screen.backend().buffer();
    let text = rows(buffer);
    println!("{}", text.join("\n"));
    let (y, row) = text
        .iter()
        .enumerate()
        .find(|(_, r)| r.contains("Read failed"))
        .unwrap();
    assert!(
        row.contains("›! corral  Read failed · corral ls exited 1"),
        "{row}"
    );
    let x = row[..row.find("corral").unwrap()].chars().count() as u16;
    let cell = &buffer[(x, y as u16)];
    assert_eq!(cell.fg, t.muted);
    assert!(!cell.modifier.contains(Modifier::BOLD));
}
