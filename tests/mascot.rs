use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect, style::Color};
use saddle::{agents::Panel, corral::Agent, mascot::Mascot, terminals::Terminals, ui};

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
                mascot_enabled: true,
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
#[test]
fn decoration_is_available_without_an_agent_and_small_views_hide_it() {
    let mut panes = Terminals::new("unused-fake-corral".into());
    let mut panel = Panel::default();
    let mut m = Mascot::default();
    let b = render(&panes, &mut panel, &mut m, 140, 40, 100.0);
    let viewer = saddle::layout::Panes::new(b.area, &Default::default()).viewer;
    assert!(
        (0..7)
            .any(|y| (viewer.x..viewer.right()).any(|x| b[(x, y)].fg == Color::Rgb(217, 119, 87)))
    );
    let id = panes.active_pane().id;
    panes.get_mut(id).unwrap().viewer.remembered = saddle::layout_state::Content::Plugin {
        id: "test.plugin".into(),
    };
    assert!(panes.mascot_area(viewer, &[]).is_some());
    for width in 0..28 {
        for height in 0..20 {
            render(&panes, &mut panel, &mut m, width, height, 100.5);
        }
    }
    assert!(panes.mascot_area(Rect::new(0, 0, 10, 40), &[]).is_none());
    assert!(panes.mascot_area(Rect::new(0, 0, 80, 2), &[]).is_none());
}

#[test]
fn decoration_does_not_follow_agent_state_or_instance() {
    let mut panes = Terminals::new("unused-fake-corral".into());
    let id = panes.active_pane().id;
    panes.get_mut(id).unwrap().viewer.showing = Some("p/a".into());
    let mut idle = Panel::default();
    idle.absorb(
        vec![Agent {
            name: "p/a".into(),
            state: Some("idle".into()),
            instance: Some("one".into()),
            ..Default::default()
        }],
        None,
        100.0,
    );
    let mut changed = Panel::default();
    changed.absorb(idle.agents.clone(), None, 100.0);
    let mut a = Mascot::default();
    let mut b = Mascot::default();
    let pixels = |buf: &Buffer| {
        let viewer = saddle::layout::Panes::new(buf.area, &Default::default()).viewer;
        (0..6)
            .flat_map(|y| (viewer.x..viewer.right()).map(move |x| (x, y)))
            .filter_map(|(x, y)| {
                let c = &buf[(x, y)];
                (c.fg == Color::Rgb(217, 119, 87)).then(|| (x, y, c.symbol().to_owned(), c.bg))
            })
            .collect::<Vec<_>>()
    };
    for step in 0..80 {
        let now = 100.0 + f64::from(step) / 12.0;
        if step == 8 {
            changed.agents[0].state = Some("blocked".into());
        }
        if step == 24 {
            changed.agents[0].state = Some("working".into());
            changed.agents[0].instance = Some("new".into());
        }
        let left = render(&panes, &mut idle, &mut a, 140, 40, now);
        let right = render(&panes, &mut changed, &mut b, 140, 40, now);
        assert!(!pixels(&left).is_empty());
        assert_eq!(
            pixels(&left),
            pixels(&right),
            "agent state/instance changed the decoration at tick {step}"
        );
    }
}

#[test]
fn patrol_plays_colored_prop_actions_between_walks() {
    let mut mascot = Mascot::default();
    let mut terminal = Terminal::new(TestBackend::new(64, 7)).unwrap();
    let mut colors = Vec::new();
    let mut positions = Vec::new();
    for tick in 0..1440 {
        terminal
            .draw(|f| {
                mascot.draw(f, f.area(), f64::from(tick) / 12.0, &[]);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let mut left = 64;
        for y in 0..7 {
            for x in 0..64 {
                let c = &buffer[(x, y)];
                for color in [c.fg, c.bg] {
                    if matches!(color, Color::Rgb(..)) && !colors.contains(&color) {
                        colors.push(color);
                    }
                }
                if c.fg == Color::Rgb(217, 119, 87) || c.bg == Color::Rgb(217, 119, 87) {
                    left = left.min(x);
                }
            }
        }
        if left < 64 {
            positions.push(left);
        }
    }
    assert!(
        positions.iter().max().unwrap() - positions.iter().min().unwrap() >= 5,
        "walking must travel"
    );
    assert!(
        colors.len() > 2,
        "curated actions must include colored props, not only the old two-color feet/blinks: {colors:?}"
    );
}

#[test]
fn overlay_replaces_underlying_glyphs_but_preserves_controls_and_hidden_time() {
    use ratatui::style::Modifier;
    let mut mascot = Mascot::default();
    let mut terminal = Terminal::new(TestBackend::new(50, 10)).unwrap();
    let protected = Rect::new(7, 1, 2, 1);
    let mut painted = Vec::new();
    let mut render = |m: &mut Mascot, now| {
        terminal
            .draw(|f| {
                for y in 0..10 {
                    for x in 0..50 {
                        f.buffer_mut()[(x, y)]
                            .set_symbol("─")
                            .set_fg(Color::White)
                            .set_bg(Color::Black)
                            .set_style(
                                ratatui::style::Style::default().add_modifier(Modifier::UNDERLINED),
                            );
                    }
                }
                painted = m.draw(f, Rect::new(0, 0, 50, 3), now, &[protected]);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    };
    let initial = render(&mut mascot, 0.0);
    mascot.hide();
    let resumed = render(&mut mascot, 900.0);
    assert_eq!(
        initial, resumed,
        "hidden time must not advance the animation"
    );
    assert!(painted.iter().any(|r| r.y < 3));
    assert!(
        painted.iter().all(|r| r.y < 3),
        "the smaller standing body fits above the existing border"
    );
    for r in painted {
        assert!(!r.intersects(protected));
        let cell = &initial[(r.x, r.y)];
        assert_ne!(cell.symbol(), "─", "border cannot show through the body");
        assert!(!cell.modifier.contains(Modifier::UNDERLINED));
    }
    for x in protected.x..protected.right() {
        assert_eq!(initial[(x, 1)].symbol(), "─");
        assert!(initial[(x, 1)].modifier.contains(Modifier::UNDERLINED));
    }
    let mut indexed = Mascot::new(false);
    terminal
        .draw(|f| {
            indexed.draw(f, f.area(), 0.0, &[]);
        })
        .unwrap();
    assert!(
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .all(|c| !matches!(c.fg, Color::Rgb(..)) && !matches!(c.bg, Color::Rgb(..)))
    );
}

#[test]
fn standing_eyes_are_inside_an_unbroken_forehead() {
    let mut mascot = Mascot::default();
    let mut terminal = Terminal::new(TestBackend::new(20, 3)).unwrap();
    terminal
        .draw(|f| {
            mascot.draw(f, f.area(), 0.0, &[]);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let mut eyes = Vec::new();
    for y in 0..3 {
        for x in 0..20 {
            let cell = &buffer[(x, y)];
            if cell.symbol() == "▪" {
                assert_eq!(cell.fg, Color::Rgb(20, 20, 19));
                assert_eq!(
                    cell.bg,
                    Color::Rgb(217, 119, 87),
                    "the forehead must remain solid around each eye"
                );
                eyes.push((x, y));
            }
        }
    }
    assert_eq!(
        eyes.len(),
        2,
        "standing eyes must be two small squares, not half-cell bars"
    );
    assert_eq!(eyes[0].1, eyes[1].1, "eyes must be level");
    let outside_right_eye = &buffer[(eyes[1].0 + 1, eyes[1].1)];
    assert_eq!(
        outside_right_eye.symbol(),
        "█",
        "the right eye needs a solid forehead margin toward the head edge"
    );
    assert_eq!(outside_right_eye.fg, Color::Rgb(217, 119, 87));
    // Reach the right edge and finish turning: the approved open eyes return.
    for tick in 1..=30 {
        terminal
            .draw(|f| {
                mascot.draw(f, f.area(), f64::from(tick) / 12.0, &[]);
            })
            .unwrap();
    }
    assert_eq!(
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .filter(|c| c.symbol() == "▪")
            .count(),
        2
    );
}

#[test]
fn curated_patrol_stays_above_the_agent_border() {
    let panes = Terminals::new("unused-fake-corral".into());
    let area = Rect::new(0, 0, 70, 16);
    let lane = panes.mascot_area(area, &[]).unwrap();
    let mut mascot = Mascot::default();
    let mut terminal = Terminal::new(TestBackend::new(70, 16)).unwrap();
    for tick in 0..7200 {
        let mut painted = Vec::new();
        terminal
            .draw(|f| {
                for x in 0..70 {
                    f.buffer_mut()[(x, 3)].set_symbol("─");
                    f.buffer_mut()[(x, 4)].set_symbol("A");
                }
                painted = mascot.draw(f, lane, f64::from(tick) / 12.0, &[]);
            })
            .unwrap();
        assert!(
            !painted.is_empty(),
            "the mascot must not vanish between poses"
        );
        assert!(
            painted.iter().all(|r| r.y < 3),
            "animation crossed the agent border at tick {tick}"
        );
        for x in 0..70 {
            assert_eq!(terminal.backend().buffer()[(x, 3)].symbol(), "─");
            assert_eq!(terminal.backend().buffer()[(x, 4)].symbol(), "A");
        }
    }
}

#[test]
fn sunglasses_lenses_preserve_the_forehead() {
    let mut mascot = Mascot::default();
    let mut terminal = Terminal::new(TestBackend::new(64, 3)).unwrap();
    for tick in 0..7200 {
        terminal
            .draw(|f| {
                mascot.draw(f, f.area(), f64::from(tick) / 12.0, &[]);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        for x in 1..63 {
            let bridge = &buffer[(x, 1)];
            if bridge.symbol() == "─"
                && bridge.fg == Color::Rgb(20, 20, 19)
                && buffer
                    .content()
                    .iter()
                    .filter(|c| c.symbol() == "■")
                    .count()
                    == 2
            {
                for lens_x in [x - 1, x + 1] {
                    let lens = &buffer[(lens_x, 1)];
                    assert_eq!(
                        lens.symbol(),
                        "■",
                        "lenses must sit inside the face, not cut into its top edge"
                    );
                    assert_eq!(lens.bg, Color::Rgb(217, 119, 87));
                }
                return;
            }
        }
    }
    panic!("patrol never displayed sunglasses");
}

#[test]
fn patrol_includes_excited_cheering_above_the_border() {
    let mut mascot = Mascot::default();
    let mut terminal = Terminal::new(TestBackend::new(64, 3)).unwrap();
    let mut cheering = false;
    let mut sparkles = false;
    for tick in 0..7200 {
        terminal
            .draw(|f| {
                mascot.draw(f, f.area(), f64::from(tick) / 12.0, &[]);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let smiles = buffer
            .content()
            .iter()
            .filter(|c| c.symbol() == "^" && c.fg == Color::Rgb(20, 20, 19))
            .count();
        if smiles == 2 {
            cheering = true;
            assert!(
                buffer
                    .content()
                    .iter()
                    .filter(|c| c.symbol() == "^")
                    .all(|c| c.bg == Color::Rgb(217, 119, 87))
            );
            sparkles |= (0..64).any(|x| buffer[(x, 0)].symbol() == "*");
        }
        if cheering && sparkles {
            return;
        }
    }
    assert!(
        cheering && sparkles,
        "patrol must include the new smiling, sparkling cheer"
    );
}

#[test]
fn pirate_keeps_one_visible_eye_and_an_inset_patch_with_connected_strap() {
    let mut mascot = Mascot::default();
    let mut terminal = Terminal::new(TestBackend::new(64, 3)).unwrap();
    for tick in 0..7200 {
        terminal
            .draw(|f| {
                mascot.draw(f, f.area(), f64::from(tick) / 12.0, &[]);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        if buffer
            .content()
            .iter()
            .filter(|c| c.symbol() == "■")
            .count()
            != 1
        {
            continue;
        }
        for x in 2..63 {
            let patch = &buffer[(x, 1)];
            if patch.symbol() == "■" {
                let eye = &buffer[(x - 2, 1)];
                assert!(
                    matches!(eye.symbol(), "▪" | "–"),
                    "one eye remains visible beside the strap"
                );
                for strap_x in [x - 1, x + 1] {
                    let strap = &buffer[(strap_x, 1)];
                    assert_eq!(
                        strap.symbol(),
                        "─",
                        "strap must connect directly to both sides of the patch"
                    );
                    assert_eq!(strap.fg, Color::Rgb(20, 20, 19));
                    assert_eq!(strap.bg, Color::Rgb(217, 119, 87));
                }
                assert_eq!(
                    patch.bg,
                    Color::Rgb(217, 119, 87),
                    "patch must preserve the forehead"
                );
                assert_eq!(eye.bg, Color::Rgb(217, 119, 87));
                assert!(
                    buffer
                        .content()
                        .iter()
                        .any(|c| c.fg == Color::Rgb(159, 66, 54)),
                    "red bandana accompanies the eye patch"
                );
                return;
            }
        }
    }
    panic!("patrol never displayed the pirate costume");
}
