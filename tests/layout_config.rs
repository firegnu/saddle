use ratatui::layout::Rect;
use saddle::{config::Config, layout::Panes};

#[test]
fn responsive_management_panes_reserve_a_status_row() {
    let config = Config::parse("left_width = 40\nleft_split = 0.6").unwrap();
    let panes = Panes::new(Rect::new(0, 0, 120, 40), &config);
    assert_eq!(panes.agents, Rect::new(0, 0, 40, 23));
    assert_eq!(panes.queue, Rect::new(0, 23, 40, 16));
    assert_eq!(panes.viewer, Rect::new(40, 0, 80, 39));
    let small = Panes::new(Rect::new(3, 2, 80, 24), &Config::default());
    assert_eq!(small.agents, Rect::new(3, 3, 34, 22));
    assert!(small.queue.is_empty());
    assert_eq!(small.viewer, Rect::new(37, 2, 46, 23));
}

#[test]
fn defaults_and_invalid_configuration_are_explicit() {
    let default = Config::parse("").unwrap();
    assert_eq!(default.queue.drover, "drover");
    assert_eq!(default.left_width, 52);
    for invalid in [
        "left_split = 0.0",
        "left_split = 1.0",
        "left_split = nan",
        "left_width = 0",
        "refresh_ms = 0",
        "corral = ''",
        "[queue]\ndrover = ''",
    ] {
        assert!(Config::parse(invalid).is_err(), "accepted {invalid}");
    }
}

#[test]
fn external_queue_ui_commands_are_rejected() {
    assert!(Config::parse("[queue]\ncommand = ['drover', 'board']").is_err());
}

#[test]
fn tabs_and_status_cover_tiny_and_normal_windows_without_overlap() {
    for (w, h) in
        (0..8)
            .flat_map(|w| (0..8).map(move |h| (w, h)))
            .chain([(80, 24), (120, 36), (160, 48)])
    {
        let area = Rect::new(3, 2, w, h);
        for queue in [false, true] {
            let p = Panes::with_queue(area, &Config::default(), queue);
            let rects = [p.agents, p.queue, p.viewer, p.status, p.tabs];
            assert_eq!(rects.iter().map(|r| r.area()).sum::<u32>(), area.area());
            for (i, r) in rects.iter().enumerate().filter(|(_, r)| !r.is_empty()) {
                assert_eq!(r.intersection(area), *r);
                for other in &rects[i + 1..] {
                    assert!(r.intersection(*other).is_empty());
                }
            }
        }
    }
}
