//! Telemetry reading layers drawn from a synthetic store: list, detail and body keep their
//! layering, and labels, metadata and text stay readable after presentation tidying.
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{buffer::Buffer, style::Modifier};
use saddle::telemetry::{EventInput, Observation, OperationInput, SettingInput, Store};
use saddle::telemetry_view::Page;
use saddle::theme::Theme;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

fn press(page: &mut Page, code: KeyCode) {
    page.event(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)));
}
fn settle(page: &mut Page) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while page.loading() && Instant::now() < deadline {
        page.poll();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!page.loading(), "reads did not finish");
}
fn draw(page: &mut Page, width: u16, height: u16) -> Buffer {
    use ratatui::{Terminal, backend::TestBackend};
    settle(page);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| page.draw(&Theme::default(), frame, frame.area()))
        .unwrap();
    terminal.backend().buffer().clone()
}
/// Where `text` starts on screen, by column and row.
fn find(buffer: &Buffer, text: &str) -> (u16, u16) {
    let area = buffer.area;
    for y in 0..area.height {
        let row: Vec<&str> = (0..area.width).map(|x| buffer[(x, y)].symbol()).collect();
        for x in 0..area.width as usize {
            if row[x..].concat().starts_with(text) {
                return (x as u16, y);
            }
        }
    }
    panic!("{text} not on screen");
}
fn file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}
fn synthetic(root: &Path, dir: &Path) {
    let store = Store::new(root.to_path_buf());
    let on = |enabled| SettingInput {
        schema_version: 1,
        enabled,
        actor: "synthetic".into(),
    };
    store.set_recording(None, on(true)).unwrap();
    for (id, key, label) in [("b", "T56", "older trace"), ("a", "T57", "Fix queue focus")] {
        store
            .create_trace(
                serde_json::from_value(json!({"schema_version":1,"trace_id":id,"origin":"task",
                    "label":label,"binding":{"kind":"drover.task","scope":"/synthetic/proj",
                    "key":key,"run":format!("run-{id}")}}))
                .unwrap(),
            )
            .unwrap();
    }
    store
        .create_dispatch(
            serde_json::from_value(json!({"schema_version":1,"trace_id":"a",
                "dispatch_id":"impl-77c0","kind":"implementation","parent_dispatch_id":null}))
            .unwrap(),
        )
        .unwrap();
    let body = file(dir, "req", "需求 first line\nsecond line\n".as_bytes());
    store
        .append(EventInput {
            schema_version: 1,
            event_id: "req".into(),
            trace_id: "a".into(),
            dispatch_id: None,
            operation_id: None,
            kind: "requirement.recorded".into(),
            observed_at: json!("2020-01-01T00:00:00Z"),
            producer: "synthetic controller".into(),
            evidence_kind: "controller_statement".into(),
            payload: json!({"declared_speaker":"user (declared)",
                "acquisition":"controller_transcription","declared_at":null}),
            links: vec![],
            bodies: vec![serde_json::from_value(json!({"role":"text","path":body})).unwrap()],
        })
        .unwrap();
    let message = file(dir, "message.txt", b"actual message sent");
    let capture = store
        .prepare_operation(
            OperationInput {
                schema_version: 1,
                trace_id: "a".into(),
                dispatch_id: "impl-77c0".into(),
                kind: "agent.send".into(),
                producer: "saddle.synthetic".into(),
                basis_event_ids: vec![],
                decision_event_id: None,
                previous_brief_event_id: None,
            },
            Observation {
                observed_at: Value::Null,
                payload: json!({"target_name":"synthetic/agent","send_kind":"initial","gaps":[]}),
                bodies: vec![
                    serde_json::from_value(json!({"role":"message","path":message})).unwrap(),
                ],
            },
            Some(file(dir, "brief.md", "# 任务书\nbrief body\n".as_bytes())),
        )
        .unwrap();
    store.record_begin(&capture).unwrap();
    store.set_recording(None, on(false)).unwrap();
}

#[test]
fn columns_align_and_selection_reads_by_weight_and_background() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("telemetry");
    synthetic(&root, dir.path());
    let t = Theme::default();
    let mut page = Page::open(Ok(Store::new(root)), None, None);

    // The selected trace has the selection background and weight; filters are not emphasized.
    let list = draw(&mut page, 100, 40);
    let label = list[find(&list, "Fix queue focus")].style();
    assert_eq!(label.bg, Some(t.agent_selected));
    assert!(label.add_modifier.contains(Modifier::BOLD));
    let other = list[find(&list, "older trace")].style();
    assert_ne!(other.bg, Some(t.agent_selected));
    assert!(
        !list[find(&list, "‹Project")]
            .style()
            .add_modifier
            .contains(Modifier::BOLD)
    );
    find(&list, "Coverage  from ");

    // Event columns line up under their header; only the panel's main action has weight.
    press(&mut page, KeyCode::Enter);
    let detail = draw(&mut page, 100, 40);
    let (header, _) = find(&detail, "Bodies / gaps");
    let (cell, row) = find(&detail, "Bodies 1 · Late");
    assert_eq!(header, cell);
    assert_eq!(detail[(cell, row)].style().bg, Some(t.agent_selected));
    let read = detail[find(&detail, "‹Read full body")].style();
    assert!(read.add_modifier.contains(Modifier::BOLD));
    let technical = detail[find(&detail, "‹Technical details")].style();
    assert!(!technical.add_modifier.contains(Modifier::BOLD));
    let recorded = detail[find(&detail, "declared 2020-01-01")].style();
    assert_eq!(recorded.fg, Some(t.muted));
}
