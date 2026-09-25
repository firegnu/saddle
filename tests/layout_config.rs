use ratatui::layout::Rect;
use saddle::{config::Config, layout::Panes};

#[test]
fn configured_three_panes_fill_the_window_and_resize() {
    let config = Config::parse("left_width = 40\nleft_split = 0.6").unwrap();
    let panes = Panes::new(Rect::new(0, 0, 120, 40), &config);
    assert_eq!(panes.agents, Rect::new(0, 0, 40, 24));
    assert_eq!(panes.queue, Rect::new(0, 24, 40, 16));
    assert_eq!(panes.viewer, Rect::new(40, 0, 80, 40));
    let small = Panes::new(Rect::new(3, 2, 30, 10), &config);
    assert_eq!(small.agents, Rect::new(3, 2, 15, 6));
    assert_eq!(small.queue, Rect::new(3, 8, 15, 4));
    assert_eq!(small.viewer, Rect::new(18, 2, 15, 10));
    for w in 0..5 {
        for h in 0..5 {
            let p = Panes::new(Rect::new(0, 0, w, h), &config);
            assert_eq!(
                p.agents.area() + p.queue.area() + p.viewer.area(),
                u32::from(w * h)
            );
        }
    }
}

#[test]
fn defaults_and_invalid_configuration_are_explicit() {
    let default = Config::parse("").unwrap();
    assert_eq!(default.queue.command, ["drover", "board"]);
    assert_eq!(default.left_width, 52);
    for invalid in [
        "left_split = 0.0",
        "left_split = 1.0",
        "left_split = nan",
        "left_width = 0",
        "refresh_ms = 0",
        "corral = ''",
        "[queue]\ncommand = []",
    ] {
        assert!(Config::parse(invalid).is_err(), "accepted {invalid}");
    }
}
