use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Rect, style::Color};
use saddle::{
    agents::Panel,
    corral::Agent,
    mascot::{Mascot, Pet},
    terminals::Terminals,
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
                updates: false,
                pinned: None,
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
    let mut indexed = Mascot::new(Pet::Clawd, false);
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
    // The first walk lasts 40 ticks; the action after it opens on the standing pose.
    let standing = play(&mut mascot, Rect::new(0, 0, 60, 3), 0..42)
        .pop()
        .unwrap();
    let mut eyes = Vec::new();
    for y in 0..3 {
        for x in 0..60 {
            let cell = &standing[(x, y)];
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
    for outside in [eyes[0].0 - 1, eyes[1].0 + 1] {
        let margin = &standing[(outside, eyes[0].1)];
        assert_ne!(margin.symbol(), " ", "skin continues outside each eye");
        assert_eq!(margin.fg, Color::Rgb(217, 119, 87));
    }
    assert_eq!(
        standing[(eyes[1].0 + 1, eyes[1].1)].symbol(),
        "█",
        "the right eye needs a solid forehead margin toward the head edge"
    );
}

#[test]
fn clawd_keeps_both_eyes_while_walking_and_turning_either_way() {
    let mut mascot = Mascot::default();
    // Two cells of travel: it paces and turns at both ends, looking where it is going.
    let frames = play(&mut mascot, Rect::new(0, 0, 20, 3), 0..96);
    let mut eye_columns = std::collections::BTreeSet::new();
    for (tick, buffer) in frames.iter().enumerate() {
        let count = |glyph: &str| {
            buffer
                .content()
                .iter()
                .filter(|c| c.symbol() == glyph)
                .count()
        };
        assert!(
            (count("▪"), count("–")) == (2, 0) || (count("▪"), count("–")) == (0, 2),
            "tick {tick}: two open eyes, or both shut in the blink of a turn"
        );
        if let Some(x) = (0..60).find(|&x| buffer[(x, 1)].symbol() == "▪") {
            // Relative to the lane position the body is drawn at.
            eye_columns.insert(x);
        }
    }
    assert!(
        eye_columns.len() >= 3,
        "the gaze follows the heading: {eye_columns:?}"
    );
}

#[test]
fn clawd_walks_with_only_its_legs_moving() {
    const QUADRANTS: &str = " ▘▝▀▖▌▞▛▗▚▐▜▄▙▟█";
    let mut mascot = Mascot::default();
    // Two cells of travel: heading right on ticks 0..8, turning, heading left on 16..24.
    let frames = play(&mut mascot, Rect::new(0, 0, 20, 3), 0..24);
    // Everything above the feet, relative to where the body is drawn: the top two rows,
    // and the top half of the bottom row, whose bottom half holds the feet.
    let split = |tick: usize, at: u16| {
        let buffer = &frames[tick];
        let mut upper = Vec::new();
        let mut feet = Vec::new();
        for col in 0..16 {
            let x = 1 + at + col;
            for y in 0..2 {
                let c = &buffer[(x, y)];
                upper.push((c.symbol().to_owned(), c.fg, c.bg));
            }
            let c = &buffer[(x, 2)];
            let mask = QUADRANTS
                .chars()
                .position(|q| q.to_string() == c.symbol())
                .unwrap_or_else(|| panic!("tick {tick}: {:?} is not a quadrant", c.symbol()));
            let color = |bit: usize| if mask & bit != 0 { c.fg } else { c.bg };
            upper.push((String::new(), color(1), color(2)));
            feet.push((color(4), color(8)));
        }
        (upper, feet)
    };
    for (ticks, at) in [
        (
            0..8,
            Box::new(|t: usize| (t / 4) as u16) as Box<dyn Fn(usize) -> u16>,
        ),
        (16..24, Box::new(|t: usize| 2 - ((t - 16) / 4) as u16)),
    ] {
        let first = split(ticks.start, at(ticks.start));
        let mut feet = std::collections::BTreeSet::new();
        for tick in ticks {
            let (upper, legs) = split(tick, at(tick));
            assert_eq!(upper, first.0, "tick {tick}: only the legs may move");
            feet.insert(format!("{legs:?}"));
        }
        assert!(feet.len() > 1, "the legs step");
    }
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
            sparkles |= (0..64).any(|x| buffer[(x, 0)].symbol() == "▪");
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

/// A tiny pack: `a` marks the left half of the bottom-left cell, `b` adds a second marker,
/// `t` fills that cell, `c` marks cell 5 and `e` leaves the bottom of the top-left cell open.
fn pack(mirror: bool, extra: &str) -> String {
    let row = ".".repeat(32);
    let art = |bottom: &str| {
        let bottom = format!("{bottom}{}", ".".repeat(32 - bottom.len()));
        format!(
            "art = '''\n{}\n{}\n'''",
            [row.as_str(); 4].join("\n"),
            [bottom.as_str(); 2].join("\n")
        )
    };
    format!(
        "step_ticks = 4\nmirror = {mirror}\n[palette]\nA = \"#ff0000\"\nB = \"#00ff00\"\n\
         [poses.a]\n{}\n[poses.b]\n{}\n[poses.t]\n{}\n[poses.c]\n{}\n\
         [poses.e]\n{}\ncells = [[0, 0, \"▂\", \".\", \"A\"]]\n\
         [[clip]]\nname = \"walking\"\nframes = [[\"a\", 2], [\"b\", 2]]\n\
         [[clip]]\nname = \"turning\"\nframes = [[\"t\", 4]]\n\
         [[clip]]\nname = \"act\"\nframes = [[\"e\", 4]]\n{extra}",
        art("A"),
        art("A.BB"),
        art("AA"),
        art("..........BB"),
        art("A"),
    )
}
/// The buffer after each 12 fps tick, drawing into `area` of a 60x3 screen.
fn play(m: &mut Mascot, area: Rect, ticks: std::ops::Range<u32>) -> Vec<Buffer> {
    let mut terminal = Terminal::new(TestBackend::new(60, 3)).unwrap();
    ticks
        .map(|tick| {
            terminal
                .draw(|f| {
                    m.draw(f, area, f64::from(tick) / 12.0, &[]);
                })
                .unwrap();
            terminal.backend().buffer().clone()
        })
        .collect()
}
const RED: Color = Color::Rgb(255, 0, 0);
const GREEN: Color = Color::Rgb(0, 255, 0);
fn leftmost(buffer: &Buffer, color: Color) -> Option<u16> {
    (0..60).find(|&x| buffer[(x, 2)].fg == color)
}

#[test]
fn walking_moves_one_cell_exactly_when_the_feet_change() {
    let mut m = Mascot::from_pack(&pack(false, ""), true).unwrap();
    let frames = play(&mut m, Rect::new(0, 0, 60, 3), 0..17);
    let at: Vec<u16> = frames.iter().map(|b| leftmost(b, RED).unwrap()).collect();
    assert_eq!(at[0], 1, "the lane starts one column in");
    for tick in 1..17 {
        let moved = at[tick] - at[tick - 1];
        assert_eq!(
            moved,
            u16::from(tick % 4 == 0),
            "tick {tick}: a step is one cell, only on a step tick: {at:?}"
        );
    }
    // Pose `b` (the second marker) shows on ticks 2 and 3 of each four-tick cycle.
    for (tick, buffer) in frames.iter().enumerate() {
        assert_eq!(
            leftmost(buffer, GREEN).is_some(),
            tick % 4 >= 2,
            "the pose and the position are driven by the same tick"
        );
    }
}

#[test]
fn heading_left_mirrors_poses_and_the_turn_flips_halfway() {
    let mut m = Mascot::from_pack(&pack(true, ""), true).unwrap();
    // Twenty columns leave two cells of travel: the right edge is reached on tick 8.
    let frames = play(&mut m, Rect::new(0, 0, 20, 3), 0..24);
    let cell = |tick: usize, x: u16| {
        (
            frames[tick][(x, 2)].symbol().to_owned(),
            frames[tick][(x, 2)].fg,
        )
    };
    assert_eq!(cell(7, 2), ("▌".into(), RED), "walking right");
    assert_eq!(cell(8, 3), ("█".into(), RED), "turning starts on arrival");
    assert_eq!(cell(9, 3), ("█".into(), RED));
    // Mirrored about the 16-cell canvas from the middle of the turning clip.
    assert_eq!(cell(10, 18), ("█".into(), RED));
    assert_eq!(
        cell(12, 18),
        ("▐".into(), RED),
        "walking left shows the mirrored pose"
    );
    assert_eq!(
        cell(14, 17),
        ("█".into(), GREEN),
        "the second marker mirrors too"
    );
    assert_eq!(
        cell(16, 17),
        ("▐".into(), RED),
        "and steps left in time with the feet"
    );
    // The left edge turns it back: the second half of that turn faces right again.
    assert_eq!(cell(21, 16), ("█".into(), RED));
    assert_eq!(cell(23, 1), ("█".into(), RED));
}

#[test]
fn a_left_clip_replaces_mirroring() {
    let extra = "[[clip]]\nname = \"walking-left\"\nframes = [[\"c\", 4]]\n";
    let mut m = Mascot::from_pack(&pack(true, extra), true).unwrap();
    let frames = play(&mut m, Rect::new(0, 0, 20, 3), 0..14);
    assert_eq!(
        frames[13][(8, 2)].symbol(),
        "█",
        "the drawn left pose, not a mirror"
    );
    assert_eq!(frames[13][(8, 2)].fg, GREEN);
    assert_eq!(leftmost(&frames[13], RED), None);
}

#[test]
fn a_transparent_part_shows_what_is_underneath() {
    use ratatui::style::Modifier;
    for (under, fg, bg, reversed) in [
        (Color::Black, Color::Black, RED, false),
        // The terminal's own background has no color to draw with, so the cell is reversed.
        (Color::Reset, RED, Color::Reset, true),
    ] {
        let mut m = Mascot::from_pack(&pack(false, ""), true).unwrap();
        let mut terminal = Terminal::new(TestBackend::new(60, 3)).unwrap();
        let mut seen = false;
        for tick in 0..600 {
            terminal
                .draw(|f| {
                    for x in 0..60 {
                        f.buffer_mut()[(x, 0)].set_bg(under);
                    }
                    m.draw(f, f.area(), f64::from(tick) / 12.0, &[]);
                })
                .unwrap();
            let buffer = terminal.backend().buffer();
            if let Some(x) = (0..60).find(|&x| buffer[(x, 0)].symbol() == "▂") {
                let cell = &buffer[(x, 0)];
                assert_eq!((cell.fg, cell.bg), (fg, bg));
                assert_eq!(cell.modifier.contains(Modifier::REVERSED), reversed);
                seen = true;
                break;
            }
        }
        assert!(seen, "the action with the open-bottomed cell never played");
    }
}

#[test]
fn every_built_in_pet_parses_patrols_and_stays_in_its_three_rows() {
    assert_eq!(Pet::default(), Pet::Clawd);
    assert_eq!(Pet::parse("cat"), Ok(Pet::Cat));
    let capybara = Pet::parse("capybara").expect("capybara is a built-in pet");
    assert_eq!(capybara.label(), "Capybara");
    assert!(Pet::ALL.contains(&capybara));
    assert!(
        Pet::parse("dog")
            .unwrap_err()
            .contains("clawd, cat or capybara")
    );
    for pet in Pet::ALL {
        assert_eq!(Pet::parse(pet.name()), Ok(pet));
        let mut m = Mascot::new(pet, true);
        let mut terminal = Terminal::new(TestBackend::new(70, 6)).unwrap();
        let mut columns = std::collections::BTreeSet::new();
        for tick in 0..7200 {
            let mut painted = Vec::new();
            terminal
                .draw(|f| {
                    painted = m.draw(f, Rect::new(0, 0, 70, 3), f64::from(tick) / 12.0, &[]);
                })
                .unwrap();
            assert!(
                !painted.is_empty(),
                "{} vanished at tick {tick}",
                pet.name()
            );
            assert!(painted.iter().all(|r| r.y < 3));
            columns.insert(painted.iter().map(|r| r.x).min().unwrap());
        }
        assert!(columns.len() > 20, "{} must travel its lane", pet.name());
    }
    // The two pets are told apart by their own body color.
    let body = |pet| {
        let mut terminal = Terminal::new(TestBackend::new(40, 3)).unwrap();
        terminal
            .draw(|f| {
                Mascot::new(pet, true).draw(f, f.area(), 0.0, &[]);
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        let colors: std::collections::BTreeSet<_> = buffer
            .content()
            .iter()
            .filter_map(|c| match c.fg {
                Color::Rgb(r, g, b) => Some((r, g, b)),
                _ => None,
            })
            .collect();
        colors
    };
    assert!(body(Pet::Clawd).contains(&(217, 119, 87)));
    assert!(!body(Pet::Cat).is_empty() && !body(Pet::Cat).contains(&(217, 119, 87)));
}

/// A tiny image pack: a 4x2 picture whose left column is red, on top of a green right column.
fn image_pack() -> String {
    "size = [4, 2]\nmirror = true\n[palette]\nA = \"#ff0000\"\nB = \"#00ff00\"\n\
     [poses.p]\npixels = '''\nA...\nA..B\n'''\n\
     [[clip]]\nname = \"walking\"\nframes = [[\"p\", 4]]\n\
     [[clip]]\nname = \"turning\"\nframes = [[\"p\", 2]]\n\
     [[clip]]\nname = \"act\"\nframes = [[\"p\", 2]]\n"
        .into()
}
/// Draws into a 60x3 screen and returns the painted cells, the sprite and the buffer.
fn image_frame(
    m: &mut Mascot,
    now: f64,
    protected: &[Rect],
    over: Option<(u16, u16)>,
) -> (Vec<Rect>, Option<saddle::mascot::Sprite>, Buffer) {
    let mut terminal = Terminal::new(TestBackend::new(60, 3)).unwrap();
    let mut painted = Vec::new();
    let mut sprite = None;
    terminal
        .draw(|f| {
            for x in 0..60 {
                for y in 0..3 {
                    f.buffer_mut()[(x, y)].set_symbol("─").set_bg(Color::Blue);
                }
            }
            painted = m.draw(f, f.area(), now, protected);
            if let Some(at) = over {
                f.buffer_mut()[at].set_symbol("x");
            }
            sprite = m.sprite(f.buffer_mut());
        })
        .unwrap();
    (painted, sprite, terminal.backend().buffer().clone())
}

#[test]
fn an_image_pet_scales_to_the_lane_and_blanks_only_the_cells_it_covers() {
    // Cells of 2x4 pixels make a 32x12 pixel lane; the 4x2 picture scales by 6 to 24x12,
    // centred four pixels in and standing on the floor.
    let mut m = Mascot::from_image_pack(&image_pack(), (2, 4)).unwrap();
    let (painted, sprite, buffer) = image_frame(&mut m, 0.0, &[], None);
    let sprite = sprite.expect("nothing drew over the picture");
    assert_eq!(sprite.size, (24, 12));
    assert_eq!(
        (sprite.cell, sprite.offset),
        ((3, 0), (0, 0)),
        "lane starts one column in"
    );
    // The red column is picture pixels 0..6 (columns 4..10 of the lane, cells 2..5) on both
    // rows; the green pixel is pixels 18..24 (cells 11..14) on the bottom row (rows 1..3).
    let mut cells: Vec<_> = painted.iter().map(|r| (r.x - 1, r.y)).collect();
    cells.sort();
    let mut expected: Vec<_> = (2..5)
        .flat_map(|x| (0..3).map(move |y| (x, y)))
        .chain((11..14).flat_map(|x| (1..3).map(move |y| (x, y))))
        .collect();
    expected.sort();
    assert_eq!(cells, expected);
    for r in &painted {
        let cell = &buffer[(r.x, r.y)];
        assert_eq!(cell.symbol(), " ", "the picture shows through a blank");
        assert_eq!(cell.bg, Color::Blue, "the blank keeps what was behind it");
    }
    assert_eq!(
        buffer[(1, 0)].symbol(),
        "─",
        "uncovered cells are untouched"
    );
    let pixels = m.pixels(&sprite);
    assert_eq!(pixels.len(), 24 * 12 * 4);
    let at = |x: usize, y: usize| &pixels[(y * 24 + x) * 4..][..4];
    assert_eq!(at(5, 0), [255, 0, 0, 255]);
    assert_eq!(at(6, 0), [0, 0, 0, 0]);
    assert_eq!(at(18, 6), [0, 255, 0, 255]);
    assert_eq!(at(18, 5), [0, 0, 0, 0]);
}

#[test]
fn an_image_pet_is_hidden_under_later_drawing_and_never_covers_controls() {
    let mut m = Mascot::from_image_pack(&image_pack(), (2, 4)).unwrap();
    let (painted, sprite, _) = image_frame(&mut m, 0.0, &[], Some((3, 1)));
    assert!(!painted.is_empty());
    assert_eq!(sprite, None, "a popup drew over a covered cell");
    let (painted, sprite, _) = image_frame(&mut m, 0.0, &[], Some((40, 1)));
    assert!(
        !painted.is_empty() && sprite.is_some(),
        "drawing elsewhere is fine"
    );
    let (painted, sprite, buffer) = image_frame(&mut m, 0.0, &[Rect::new(13, 2, 1, 1)], None);
    assert!(
        painted.is_empty() && sprite.is_none(),
        "a control is in the way"
    );
    assert_eq!(buffer[(13, 2)].symbol(), "─");
    m.hide();
    assert_eq!(m.sprite(&buffer), None);
}

#[test]
fn an_image_pet_flips_when_it_heads_left() {
    let mut m = Mascot::from_image_pack(&image_pack(), (2, 4)).unwrap();
    // Twenty columns leave two cells of travel; the turn's second half heads left.
    let mut terminal = Terminal::new(TestBackend::new(20, 3)).unwrap();
    let mut sprites = Vec::new();
    for tick in 0..12 {
        terminal
            .draw(|f| {
                m.draw(f, f.area(), f64::from(tick) / 12.0, &[]);
                sprites.push(m.sprite(f.buffer_mut()).unwrap());
            })
            .unwrap();
    }
    let (right, left) = (sprites[0], sprites[11]);
    assert!(!right.key.2 && left.key.2, "{right:?} {left:?}");
    assert_ne!(
        right.key, left.key,
        "the flipped picture is a picture of its own"
    );
    let (a, b) = (m.pixels(&right), m.pixels(&left));
    for y in 0..12 {
        for x in 0..24 {
            assert_eq!(a[(y * 24 + x) * 4..][..4], b[(y * 24 + 23 - x) * 4..][..4]);
        }
    }
}

#[test]
fn every_built_in_pet_has_pictures_that_patrol_in_its_three_rows() {
    for pet in Pet::ALL {
        // Cells of 8x19 pixels, as in the user's terminal.
        let mut m = Mascot::with_images(pet, (8, 19));
        let mut terminal = Terminal::new(TestBackend::new(70, 6)).unwrap();
        let mut columns = std::collections::BTreeSet::new();
        let mut poses = std::collections::BTreeSet::new();
        for tick in 0..7200 {
            let mut painted = Vec::new();
            let mut sprite = None;
            terminal
                .draw(|f| {
                    painted = m.draw(f, Rect::new(0, 0, 70, 3), f64::from(tick) / 12.0, &[]);
                    sprite = m.sprite(f.buffer_mut());
                })
                .unwrap();
            let sprite = sprite.unwrap_or_else(|| panic!("{} vanished at {tick}", pet.name()));
            assert!(!painted.is_empty() && painted.iter().all(|r| r.y < 3));
            let bottom = u32::from(sprite.cell.1) * 19
                + u32::from(sprite.offset.1)
                + u32::from(sprite.size.1);
            assert_eq!(bottom, 3 * 19, "{} stands on the floor", pet.name());
            assert!(u32::from(sprite.size.0) <= 16 * 8);
            columns.insert(sprite.cell.0);
            if poses.insert(sprite.key) {
                assert_eq!(
                    m.pixels(&sprite).len(),
                    usize::from(sprite.size.0) * usize::from(sprite.size.1) * 4
                );
            }
        }
        assert!(columns.len() > 20, "{} must travel its lane", pet.name());
        assert!(poses.len() > 20, "{} animates", pet.name());
    }
}

#[test]
fn clawd_pictures_walk_with_only_their_legs_moving() {
    let mut m = Mascot::with_images(Pet::Clawd, (8, 19));
    let mut terminal = Terminal::new(TestBackend::new(70, 3)).unwrap();
    // The first walk heads right for 40 ticks.
    let mut uppers = std::collections::BTreeSet::new();
    let mut legs = std::collections::BTreeSet::new();
    for tick in 0..40 {
        let mut sprite = None;
        terminal
            .draw(|f| {
                m.draw(f, f.area(), f64::from(tick) / 12.0, &[]);
                sprite = m.sprite(f.buffer_mut());
            })
            .unwrap();
        let sprite = sprite.unwrap();
        let pixels = m.pixels(&sprite);
        let (width, height) = (usize::from(sprite.size.0), usize::from(sprite.size.1));
        // The legs are the bottom four of the 23 source rows.
        let split = (19 * height).div_ceil(23) * width * 4;
        uppers.insert(pixels[..split].to_vec());
        legs.insert(pixels[split..].to_vec());
    }
    assert_eq!(uppers.len(), 1, "head, arms and body stay put");
    assert!(legs.len() > 2, "the legs step");
}

#[test]
fn blocks_display_draws_glyphs_even_where_pictures_show() {
    use saddle::mascot::Display;
    let frame = |m: &mut Mascot| {
        let mut terminal = Terminal::new(TestBackend::new(70, 3)).unwrap();
        let mut sprite = None;
        let mut glyphs = false;
        terminal
            .draw(|f| {
                m.draw(f, f.area(), 0.0, &[]);
                sprite = m.sprite(f.buffer_mut());
                glyphs = f.buffer_mut().content().iter().any(|c| c.symbol() != " ");
            })
            .unwrap();
        (sprite.is_some(), glyphs)
    };
    for pet in Pet::ALL {
        let cell = Some((8, 19));
        assert_eq!(
            frame(&mut Mascot::for_display(pet, Display::Auto, cell, true)),
            (true, false),
            "Auto shows a picture where the terminal can"
        );
        assert_eq!(
            frame(&mut Mascot::for_display(pet, Display::Auto, None, true)),
            (false, true),
            "Auto falls back to blocks"
        );
        assert_eq!(
            frame(&mut Mascot::for_display(pet, Display::Blocks, cell, true)),
            (false, true),
            "Blocks never shows a picture"
        );
    }
}
