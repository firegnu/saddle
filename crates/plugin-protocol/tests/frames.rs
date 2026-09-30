use saddle_plugin_protocol::{Color, Frame, Span};
fn frame(text: &str, cols: u16) -> Frame {
    Frame {
        panel: "main".into(),
        size_revision: 1,
        frame_id: 1,
        cols,
        rows_count: 1,
        rows: vec![vec![Span {
            text: text.into(),
            fg: Color::default(),
            bg: Color::default(),
            modifiers: vec![],
        }]],
    }
}
#[test]
fn double_width_cannot_cross_the_right_edge() {
    assert!(frame("中", 1).validate().is_err());
    assert!(frame("中A   ", 6).validate().is_ok());
}
#[test]
fn unicode_and_style_boundaries_preserve_graphemes() {
    assert!(frame("中文e\u{301}👩‍💻", 7).validate().is_ok());
    assert!(frame("\u{301}", 1).validate().is_err());
    assert!(frame("\x1b[0m", 5).validate().is_err());
    let mut f = frame("e", 1);
    let mut mark = f.rows[0][0].clone();
    mark.text = "\u{301}".into();
    f.rows[0].push(mark);
    assert!(
        f.validate().is_err(),
        "a style boundary must not split a grapheme"
    );
}
#[test]
fn malformed_and_oversized_lines_are_rejected_before_business_dispatch() {
    use saddle_plugin_protocol::{MAX_LINE, decode, read};
    assert!(decode(b"{\"kind\":\"response\",\"id\":1,\"result\":{},\"error\":{\"code\":\"x\",\"message\":\"x\"}}\n").is_err());
    let mut huge = std::io::Cursor::new(vec![b'x'; MAX_LINE + 1]);
    assert!(read(&mut huge).is_err());
    let deep = format!("{}0{}\n", "[".repeat(33), "]".repeat(33));
    assert!(decode(deep.as_bytes()).is_err());
}
#[test]
fn null_result_is_still_a_present_success_field() {
    use saddle_plugin_protocol::{Message, decode, encode};
    let bytes = encode(&Message::response(1, serde_json::Value::Null)).unwrap();
    assert!(matches!(
        decode(&bytes).unwrap(),
        Message::Response {
            result: Some(serde_json::Value::Null),
            error: None,
            ..
        }
    ));
}

#[test]
fn attention_budgets_bound_count_payload_and_depth() {
    use saddle_plugin_protocol::{AttentionItem, AttentionSnapshot};
    let item = AttentionItem {
        id: "one".into(),
        title: "One".into(),
        note: String::new(),
        action: "open".into(),
        target: serde_json::json!({"id":1}),
    };
    assert!(
        AttentionSnapshot {
            items: vec![item.clone()]
        }
        .validate()
        .is_ok()
    );
    assert!(
        AttentionSnapshot {
            items: (0..65)
                .map(|n| AttentionItem {
                    id: n.to_string(),
                    ..item.clone()
                })
                .collect()
        }
        .validate()
        .is_err()
    );
    assert!(
        AttentionSnapshot {
            items: (0..64)
                .map(|n| AttentionItem {
                    id: n.to_string(),
                    target: serde_json::json!("x".repeat(2000)),
                    ..item.clone()
                })
                .collect()
        }
        .validate()
        .is_err()
    );
    let mut deep = serde_json::Value::Null;
    for _ in 0..17 {
        deep = serde_json::json!([deep]);
    }
    assert!(
        AttentionSnapshot {
            items: vec![AttentionItem {
                target: deep,
                ..item
            }]
        }
        .validate()
        .is_err()
    );
}
