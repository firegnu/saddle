use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect, style::Color};
use saddle::{
    agents::Panel,
    corral::Agent,
    mascot::Mascot,
    terminals::{Place, Terminals},
    ui,
};

fn render(
    terminals: &Terminals,
    panel: &mut Panel,
    m: &mut Mascot,
    width: u16,
    height: u16,
    now: f64,
) -> Buffer {
    let mut t = Terminal::new(TestBackend::new(width, height)).unwrap();
    t.draw(|f| {
        ui::draw_workspace(
            f,
            panel,
            ui::View {
                colors: &Default::default(),
                panes: saddle::layout::Panes::new(f.area(), &Default::default()),
                focus: saddle::input::Focus::Viewer,
                showing: terminals.active_pane().viewer.showing.as_deref(),
                local: &[],
                viewer: None,
                projects: &[],
                viewer_note: "",
                reply: "",
                now,
                pointer: &Default::default(),
            },
            Some(ui::Workspace {
                terminals,
                mascot: m,
                placement: None,
                search: None,
                form: None,
                program: "unused-fake-corral",
                modal: false,
                attention: ui::Attention {
                    items: &[],
                    loading: false,
                    popup: None,
                },
                settings: None,
            }),
        );
    })
    .unwrap();
    t.backend().buffer().clone()
}
fn band_text(buffer: &Buffer) -> String {
    let a = saddle::layout::Panes::new(buffer.area, &Default::default()).viewer;
    (0..3.min(buffer.area.height))
        .flat_map(|y| (a.x..a.right()).map(move |x| buffer[(x, y)].symbol()))
        .collect()
}
#[test]
fn mascot_follows_displayed_pane_and_existing_waiting_error_states() {
    let mut panes = Terminals::new("unused-fake-corral".into());
    let first = panes.active_pane().id;
    panes.get_mut(first).unwrap().viewer.showing = Some("p/a".into());
    let second = panes.reserve(Place::Right, None).pane;
    panes.get_mut(second).unwrap().viewer.showing = Some("p/b".into());
    let mut panel = Panel::default();
    panel.absorb(
        vec![
            Agent {
                name: "p/a".into(),
                instance: Some("a".into()),
                state: Some("blocked".into()),
                ..Default::default()
            },
            Agent {
                name: "p/b".into(),
                instance: Some("b".into()),
                state: Some("idle".into()),
                ..Default::default()
            },
        ],
        None,
        100.0,
    );
    // A is selected in Agents, but the right-hand pane shows idle B.
    panel.selected = Some("p/a".into());
    let mut m = Mascot::default();
    let b = render(&panes, &mut panel, &mut m, 140, 40, 100.0);
    assert!(!band_text(&b).contains('?'));
    let viewer = saddle::layout::Panes::new(b.area, &Default::default()).viewer;
    assert!(
        (0..3)
            .any(|y| (viewer.x..viewer.right()).any(|x| b[(x, y)].fg == Color::Rgb(217, 119, 87)))
    );
    assert!((0..3).any(|y| {
        (viewer.x..viewer.right())
            .any(|x| b[(x, y)].fg == Color::Rgb(0, 0, 0) || b[(x, y)].bg == Color::Rgb(0, 0, 0))
    }));
    panes.focus(first);
    assert!(band_text(&render(&panes, &mut panel, &mut m, 140, 40, 100.2)).contains('?'));
    panel.agents[0].error = Some("unavailable".into());
    assert!(band_text(&render(&panes, &mut panel, &mut m, 140, 40, 100.4)).contains('!'));
    let tab = panes.new_tab();
    assert!(panes.mascot_area(viewer, &[]).is_none());
    panes.get_mut(tab).unwrap().viewer.remembered = saddle::layout_state::Content::Plugin {
        id: "test.plugin".into(),
    };
    assert!(panes.mascot_area(viewer, &[]).is_none());
    panes.focus(first);
    for width in 0..28 {
        for height in 0..20 {
            render(&panes, &mut panel, &mut m, width, height, 100.5);
        }
    }
    assert!(panes.mascot_area(Rect::new(0, 0, 10, 40), &[]).is_none());
    assert!(panes.mascot_area(Rect::new(0, 0, 80, 2), &[]).is_none());
}
