//! The host Telemetry page reads the Store and shows what it returns, nothing more.
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use saddle::telemetry::{
    BindingFilter, EventInput, Observation, OperationInput, SettingInput, Store,
};
use saddle::telemetry_view::{Outcome, Page};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

fn press(page: &mut Page, code: KeyCode) -> Outcome {
    page.event(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)))
}
fn typed(page: &mut Page, text: &str) {
    for c in text.chars() {
        press(page, KeyCode::Char(c));
    }
}
/// Waits for the page's background reads to land.
fn settle(page: &mut Page) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        page.poll();
        if !page.loading() || Instant::now() > deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!page.loading(), "reads did not finish");
}
fn screen(page: &mut Page, width: u16, height: u16) -> String {
    use ratatui::{Terminal, backend::TestBackend};
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| page.draw(&saddle::theme::Theme::default(), frame, frame.area()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            let mut line = String::new();
            let mut skip = 0;
            for x in 0..width {
                // Wide characters occupy a second, blank-symbol cell.
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                let symbol = buffer[(x, y)].symbol();
                skip = unicode_width::UnicodeWidthStr::width(symbol).saturating_sub(1);
                line.push_str(symbol);
            }
            line + "\n"
        })
        .collect()
}
fn shown(page: &mut Page) -> String {
    settle(page);
    screen(page, 100, 40)
}
fn open(root: &Path) -> Page {
    let mut page = Page::open(Ok(Store::new(root.to_path_buf())), None, None);
    settle(&mut page);
    page
}

fn store(root: &Path) -> Store {
    let store = Store::new(root.to_path_buf());
    store
        .set_recording(
            None,
            SettingInput {
                schema_version: 1,
                enabled: true,
                actor: "synthetic".into(),
            },
        )
        .unwrap();
    store
}
fn task(store: &Store, id: &str, key: &str, label: &str) {
    store
        .create_trace(
            serde_json::from_value(json!({"schema_version":1,"trace_id":id,"origin":"task","label":label,
                "binding":{"kind":"drover.task","scope":"/synthetic/proj","key":key,"run":format!("run-{id}")}}))
            .unwrap(),
        )
        .unwrap();
}
fn dispatch(store: &Store, trace: &str, id: &str, kind: &str, parent: Option<&str>) {
    store
        .create_dispatch(
            serde_json::from_value(json!({"schema_version":1,"trace_id":trace,"dispatch_id":id,
                "kind":kind,"parent_dispatch_id":parent}))
            .unwrap(),
        )
        .unwrap();
}
fn file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}
fn statement(store: &Store, dir: &Path, trace: &str, id: &str, kind: &str, body: &[u8]) {
    let path = file(dir, id, body);
    store
        .append(EventInput {
            schema_version: 1,
            event_id: id.into(),
            trace_id: trace.into(),
            dispatch_id: None,
            operation_id: None,
            kind: kind.into(),
            observed_at: json!("2020-01-01T00:00:00Z"),
            producer: "synthetic controller".into(),
            evidence_kind: "controller_statement".into(),
            payload: match kind {
                "controller.note" => json!({"note_kind":"other"}),
                "proposal.recorded" => {
                    json!({"declared_speaker":"user (declared)","acquisition":"controller_transcription"})
                }
                "authorization.recorded" => {
                    json!({"declared_speaker":"user (declared)","acquisition":"controller_transcription","context_complete":false})
                }
                _ => {
                    json!({"declared_speaker":"user (declared)","acquisition":"controller_transcription","declared_at":null})
                }
            },
            links: vec![],
            bodies: vec![serde_json::from_value(json!({"role":"text","path":path})).unwrap()],
        })
        .unwrap();
}
fn transition(store: &Store, trace: &str, id: &str, key: &str) {
    store
        .append(EventInput {
            schema_version: 1,
            event_id: id.into(),
            trace_id: trace.into(),
            dispatch_id: None,
            operation_id: None,
            kind: "task.transition".into(),
            observed_at: Value::Null,
            producer: "synthetic plugin".into(),
            evidence_kind: "plugin_statement".into(),
            payload: json!({"binding":{"kind":"drover.task","scope":"/synthetic/proj","key":key,"run":format!("run-{trace}")},
                "from":"pending","to":"running","business_committed_at":null}),
            links: vec![],
            bodies: vec![],
        })
        .unwrap();
}
/// A send whose begin was saved with a brief snapshot and message, but no end.
fn send_without_end(store: &Store, dir: &Path, trace: &str, dispatch: &str) -> String {
    let brief = file(dir, "brief.md", "# 任务书\nbrief body\n".as_bytes());
    let message = file(dir, "message.txt", b"actual message sent");
    let capture = store
        .prepare_operation(
            OperationInput {
                schema_version: 1,
                trace_id: trace.into(),
                dispatch_id: dispatch.into(),
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
            Some(brief),
        )
        .unwrap();
    store.record_begin(&capture).unwrap();
    capture.operation_id().to_owned()
}
/// The row the selection marker is on, inside the page border.
fn marked(text: &str) -> &str {
    text.lines()
        .find(|l| {
            l.trim_start_matches(['│', '┃'])
                .trim_start()
                .starts_with('▸')
        })
        .unwrap_or_else(|| panic!("no selected row:\n{text}"))
}
fn blob(root: &Path, store: &Store, event: &str) -> PathBuf {
    let hash = store.show(event).unwrap()["record"]["bodies"][0]["sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    root.join("blobs/sha256").join(hash)
}
/// Moves the event selection to the row showing `kind`.
fn select_event(page: &mut Page, kind: &str) {
    for _ in 0..200 {
        let text = screen(page, 100, 40);
        if marked(&text).contains(kind) {
            return;
        }
        press(page, KeyCode::Down);
    }
    panic!("no event row {kind}");
}

#[test]
fn missing_database_shows_not_initialized_and_creates_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("telemetry");
    let mut page = open(&root);
    let text = shown(&mut page);
    assert!(
        text.contains(
            "Telemetry is not initialized — nothing recorded yet. No files were created."
        ),
        "{text}"
    );
    assert!(text.contains("Recording: Off"), "{text}");
    // Refresh reads again; nothing is created either way.
    press(&mut page, KeyCode::Char('r'));
    shown(&mut page);
    assert!(!root.exists(), "opening the page must not create storage");
    assert!(matches!(press(&mut page, KeyCode::Esc), Outcome::Close));
}

#[test]
fn unavailable_storage_is_an_error_not_an_empty_list() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("telemetry");
    std::fs::create_dir_all(&root).unwrap();
    // A database from an unsupported version.
    let conn = rusqlite::Connection::open(root.join("telemetry.sqlite3")).unwrap();
    conn.pragma_update(None, "user_version", 9).unwrap();
    drop(conn);
    let mut page = open(&root);
    let text = shown(&mut page);
    assert!(text.contains("storage_unavailable"), "{text}");
    assert!(text.contains("unsupported database version"), "{text}");
    assert!(!text.contains("No traces"), "{text}");
    assert!(!text.contains("0 traces"), "{text}");
    assert!(text.contains("Recording: Unknown"), "{text}");

    let mut page = Page::open(
        Err(saddle::telemetry::Error {
            status: "unavailable",
            code: "storage_unavailable",
            message: "HOME is unavailable".into(),
        }),
        None,
        None,
    );
    let text = shown(&mut page);
    assert!(text.contains("HOME is unavailable"), "{text}");
}

#[test]
fn list_detail_and_reader_show_store_results_and_return_layer_by_layer() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("telemetry");
    let store = store(&root);
    task(&store, "trace-b", "T56", "older trace");
    task(&store, "trace-a", "T57", "T57 Fix queue focus");
    dispatch(&store, "trace-a", "impl-77c0", "implementation", None);
    dispatch(
        &store,
        "trace-a",
        "review-90de",
        "review",
        Some("impl-77c0"),
    );
    statement(
        &store,
        dir.path(),
        "trace-a",
        "req",
        "requirement.recorded",
        "需求 \x1b[31mred\x07\n".as_bytes(),
    );
    send_without_end(&store, dir.path(), "trace-a", "impl-77c0");
    transition(&store, "trace-a", "move", "T57");
    // Recording turned off globally after the trace: closed now, history still readable.
    store
        .set_recording(
            None,
            SettingInput {
                schema_version: 1,
                enabled: false,
                actor: "synthetic".into(),
            },
        )
        .unwrap();

    let mut page = open(&root);
    let list = shown(&mut page);
    assert!(list.contains("Recording: Off (global)"), "{list}");
    assert!(list.contains("Filter: all traces"), "{list}");
    assert!(list.contains("2 traces"), "{list}");
    assert!(list.contains("登记声明未核验"), "{list}");
    assert!(list.contains("未见运行登记声明"), "{list}");
    assert!(list.contains("drover.task · T57"), "{list}");
    assert!(page.status().contains("Esc Close"));
    // The newest trace is selected first; its details are below the list.
    press(&mut page, KeyCode::Down);
    press(&mut page, KeyCode::Up);
    let list = shown(&mut page);
    assert!(list.contains("T57 Fix queue focus"), "{list}");
    assert!(list.contains("/synthetic/proj"), "{list}");
    assert!(marked(&list).contains("T57"), "{list}");

    assert!(matches!(press(&mut page, KeyCode::Enter), Outcome::Stay));
    let detail = shown(&mut page);
    assert!(page.status().contains("Esc Back"));
    assert!(detail.contains("Now (queried"), "{detail}");
    assert!(detail.contains("Events seq ≤"), "{detail}");
    assert!(detail.contains("登记声明未核验"), "{detail}");
    // Both dispatches are listed, though only one has events.
    assert!(detail.contains("Dispatch: All"), "{detail}");
    assert!(detail.contains("implementation impl"), "{detail}");
    assert!(detail.contains("review revi"), "{detail}");
    assert!(detail.contains("no end 1"), "{detail}");
    for kind in [
        "requirement.recorded",
        "brief.snapshot",
        "agent.send.begin",
        "task.transition",
    ] {
        assert!(detail.contains(kind), "{kind}: {detail}");
    }
    // Missing ends are not inserted into the event list as rows.
    assert!(!detail.contains("agent.send.end"), "{detail}");

    select_event(&mut page, "requirement.recorded");
    let detail = shown(&mut page);
    assert!(detail.contains("需求原文（声明来源）"), "{detail}");
    assert!(
        detail.contains("declared by synthetic controller (unverified)"),
        "{detail}"
    );
    assert!(
        detail.contains("声明的说话者/来源（未经独立核验）"),
        "{detail}"
    );
    assert!(
        detail.contains("事后提交/声明时间在未采集区间或早于当前记录时段"),
        "{detail}"
    );
    assert!(detail.contains("late"), "{detail}");

    // Reader: full text, control characters made visible.
    press(&mut page, KeyCode::Enter);
    let reader = shown(&mut page);
    assert!(reader.contains("hash verified · UTF-8"), "{reader}");
    assert!(reader.contains("需求 \\x1b[31mred\\x07"), "{reader}");
    assert!(reader.contains("需求原文（声明来源）"), "{reader}");
    assert!(press(&mut page, KeyCode::Esc).stay());
    let back = shown(&mut page);
    assert!(back.contains("需求原文（声明来源）"), "{back}");
    assert!(back.contains("Events seq ≤"), "{back}");

    // The brief snapshot and the message actually sent are labelled separately.
    select_event(&mut page, "brief.snapshot");
    let detail = shown(&mut page);
    assert!(detail.contains("任务书快照"), "{detail}");
    assert!(detail.contains("observed by Saddle"), "{detail}");
    assert!(detail.contains("brief.md"), "{detail}");
    assert!(!detail.contains("实际发送内容"), "{detail}");
    select_event(&mut page, "agent.send.begin");
    let detail = shown(&mut page);
    assert!(detail.contains("实际发送内容"), "{detail}");
    assert!(!detail.contains("任务书快照"), "{detail}");
    // The current operation state, marked as read now, not as part of the list.
    assert!(detail.contains("now: no end"), "{detail}");
    press(&mut page, KeyCode::Enter);
    let reader = shown(&mut page);
    assert!(reader.contains("actual message sent"), "{reader}");
    press(&mut page, KeyCode::Esc);

    // Operations and recording intervals, each from the same current read.
    press(&mut page, KeyCode::Char('o'));
    let ops = shown(&mut page);
    assert!(ops.contains("Operations now (queried"), "{ops}");
    assert!(ops.contains("执行或交付结果未知"), "{ops}");
    assert!(ops.contains("no end"), "{ops}");
    press(&mut page, KeyCode::Char('i'));
    let intervals = shown(&mut page);
    assert!(
        intervals.contains("Recording intervals now (queried"),
        "{intervals}"
    );
    assert!(
        intervals
            .contains("仅描述已记录材料；未记录操作不可见，缺口原因未知，不代表任务完整历史。"),
        "{intervals}"
    );
    assert!(intervals.contains("off"), "{intervals}");
    press(&mut page, KeyCode::Char('i'));

    // A dispatch filters the events.
    press(&mut page, KeyCode::Tab);
    let filtered = shown(&mut page);
    assert!(filtered.contains("agent.send.begin"), "{filtered}");
    assert!(!filtered.contains("requirement.recorded"), "{filtered}");
    press(&mut page, KeyCode::Tab);
    let empty = shown(&mut page);
    assert!(!empty.contains("agent.send.begin"), "{empty}");
    assert!(empty.contains("parent impl-77c0"), "{empty}");

    assert!(press(&mut page, KeyCode::Esc).stay());
    let list = shown(&mut page);
    assert!(list.contains("Filter: all traces"), "{list}");
    assert!(marked(&list).contains("T57"), "{list}");
    assert!(matches!(press(&mut page, KeyCode::Esc), Outcome::Close));
}

#[test]
fn body_states_are_distinct_and_never_shown_as_empty_text() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("telemetry");
    let store = store(&root);
    task(&store, "t", "T1", "bodies");
    statement(
        &store,
        dir.path(),
        "t",
        "empty",
        "requirement.recorded",
        b"",
    );
    statement(
        &store,
        dir.path(),
        "t",
        "binary",
        "proposal.recorded",
        b"ab\xff\xfecd",
    );
    statement(
        &store,
        dir.path(),
        "t",
        "gone",
        "authorization.recorded",
        b"missing",
    );
    statement(
        &store,
        dir.path(),
        "t",
        "bad",
        "controller.note",
        b"to be corrupted",
    );
    std::fs::remove_file(blob(&root, &store, "gone")).unwrap();
    let corrupt = blob(&root, &store, "bad");
    let mut permissions = std::fs::metadata(&corrupt).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o600);
    std::fs::set_permissions(&corrupt, permissions).unwrap();
    std::fs::write(&corrupt, b"to be CORRUPTED").unwrap();

    let mut page = open(&root);
    press(&mut page, KeyCode::Enter);
    settle(&mut page);
    select_event(&mut page, "requirement.recorded");
    press(&mut page, KeyCode::Enter);
    let text = shown(&mut page);
    assert!(
        text.contains("Empty body (0 bytes, hash verified)"),
        "{text}"
    );
    press(&mut page, KeyCode::Esc);

    select_event(&mut page, "proposal.recorded");
    press(&mut page, KeyCode::Enter);
    let text = shown(&mut page);
    assert!(text.contains("not UTF-8"), "{text}");
    assert!(text.contains("61 62 ff fe 63 64"), "{text}");
    press(&mut page, KeyCode::Char('x'));
    let text = shown(&mut page);
    assert!(text.contains("lossy"), "{text}");
    assert!(text.contains("ab��cd"), "{text}");
    press(&mut page, KeyCode::Esc);

    for (kind, code) in [
        ("authorization.recorded", "body_missing"),
        ("controller.note", "body_corrupt"),
    ] {
        select_event(&mut page, kind);
        press(&mut page, KeyCode::Enter);
        let text = shown(&mut page);
        assert!(text.contains(code), "{text}");
        assert!(text.contains("not an empty body"), "{text}");
        press(&mut page, KeyCode::Esc);
    }
}

#[test]
fn event_pages_keep_their_upper_bound_until_refresh() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("telemetry");
    let store = store(&root);
    task(&store, "t", "T1", "many");
    for i in 0..101 {
        transition(&store, "t", &format!("e{i:03}"), "T1");
    }
    let mut page = open(&root);
    press(&mut page, KeyCode::Enter);
    let first = shown(&mut page);
    assert!(first.contains("n: next page (100 per page)"), "{first}");
    let upper = first
        .lines()
        .find_map(|l| l.split("Events seq ≤ ").nth(1))
        .and_then(|s| s.split_whitespace().next())
        .unwrap()
        .to_owned();
    // A later event is not mixed into the fixed set.
    statement(
        &store,
        dir.path(),
        "t",
        "later",
        "requirement.recorded",
        b"later",
    );
    press(&mut page, KeyCode::Char('n'));
    let more = shown(&mut page);
    assert!(more.contains(&format!("Events seq ≤ {upper} ")), "{more}");
    assert!(!more.contains("n: next page"), "{more}");
    for _ in 0..101 {
        press(&mut page, KeyCode::Down);
    }
    let end = shown(&mut page);
    assert!(!end.contains("requirement.recorded"), "{end}");
    // Refresh rereads both: a new upper bound includes it.
    press(&mut page, KeyCode::Char('r'));
    shown(&mut page);
    press(&mut page, KeyCode::Char('n'));
    shown(&mut page);
    for _ in 0..101 {
        press(&mut page, KeyCode::Down);
    }
    let refreshed = shown(&mut page);
    assert!(
        !refreshed.contains(&format!("Events seq ≤ {upper} ")),
        "{refreshed}"
    );
    assert!(refreshed.contains("requirement.recorded"), "{refreshed}");
}

#[test]
fn filter_needs_kind_scope_and_key_and_clears_back_to_all() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("telemetry");
    let store = store(&root);
    task(&store, "t1", "T1", "first task");
    task(&store, "t2", "T2", "second task");
    store
        .create_trace(
            serde_json::from_value(
                json!({"schema_version":1,"trace_id":"adhoc","origin":"ad_hoc","label":"manual review"}),
            )
            .unwrap(),
        )
        .unwrap();
    let mut page = open(&root);
    assert!(shown(&mut page).contains("3 traces"));
    press(&mut page, KeyCode::Char('f'));
    typed(&mut page, "drover.task");
    press(&mut page, KeyCode::Tab);
    typed(&mut page, "/synthetic/proj");
    press(&mut page, KeyCode::Enter);
    let text = shown(&mut page);
    assert!(text.contains("kind, scope and key are required"), "{text}");
    assert!(text.contains("3 traces"), "{text}");
    press(&mut page, KeyCode::Tab);
    typed(&mut page, "T2");
    press(&mut page, KeyCode::Enter);
    let text = shown(&mut page);
    assert!(text.contains("1 trace"), "{text}");
    assert!(text.contains("key=T2"), "{text}");
    assert!(!text.contains("first task"), "{text}");
    press(&mut page, KeyCode::Char('F'));
    let text = shown(&mut page);
    assert!(text.contains("Filter: all traces"), "{text}");
    assert!(text.contains("3 traces"), "{text}");

    // The entry a plugin jump will use: an opaque binding and where it came from.
    let mut page = Page::open(
        Ok(Store::new(root.clone())),
        Some(BindingFilter {
            kind: "drover.task".into(),
            scope: "/synthetic/proj".into(),
            key: "T1".into(),
            run: None,
        }),
        Some("Drover".into()),
    );
    let text = shown(&mut page);
    assert!(text.contains("(from Drover)"), "{text}");
    assert!(text.contains("1 trace"), "{text}");
    assert!(text.contains("first task"), "{text}");
}

#[test]
fn foreign_text_is_escaped_everywhere_it_is_shown() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("telemetry");
    let store = store(&root);
    store
        .create_trace(
            serde_json::from_value(json!({"schema_version":1,"trace_id":"t","origin":"ad_hoc",
                "label":"evil\u{1b}]52;c;bad\u{7}label"}))
            .unwrap(),
        )
        .unwrap();
    let mut page = open(&root);
    let text = shown(&mut page);
    assert!(text.contains("evil\\x1b]52;c;bad\\x07label"), "{text}");
    press(&mut page, KeyCode::Enter);
    let text = shown(&mut page);
    assert!(text.contains("evil\\x1b]52;c;bad\\x07label"), "{text}");
}

#[test]
fn narrow_and_wide_windows_keep_the_same_layers() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("telemetry");
    let store = store(&root);
    task(&store, "t", "T1", "sized");
    transition(&store, "t", "move", "T1");
    let mut page = open(&root);
    settle(&mut page);
    let narrow = screen(&mut page, 80, 24);
    assert!(narrow.contains("Telemetry"), "{narrow}");
    assert!(narrow.contains("Esc Close"), "{narrow}");
    let wide = screen(&mut page, 180, 40);
    // Wide lists add the full creation date.
    let created = store.show("t").unwrap()["record"]["created_at"]
        .as_str()
        .unwrap()[..10]
        .to_owned();
    assert!(wide.contains(&created), "{wide}");
    assert!(!marked(&narrow).contains(&created), "{narrow}");
    press(&mut page, KeyCode::Enter);
    settle(&mut page);
    let narrow = screen(&mut page, 80, 24);
    assert!(narrow.contains("task.transition"), "{narrow}");
    assert!(narrow.contains("Esc Back"), "{narrow}");
    let wide = screen(&mut page, 180, 40);
    assert!(marked(&wide).contains("task.transition"), "{wide}");
    // The event detail sits beside the timeline, not below it.
    let beside = wide
        .lines()
        .find_map(|l| {
            l.find("declared by synthetic plugin (unverified)")
                .map(|i| l[..i].chars().count())
        })
        .unwrap_or_else(|| panic!("{wide}"));
    assert!(beside > 90, "{wide}");
}

trait Stay {
    fn stay(&self) -> bool;
}
impl Stay for Outcome {
    fn stay(&self) -> bool {
        matches!(self, Outcome::Stay)
    }
}

#[test]
fn refresh_keeps_the_selected_dispatch_and_its_events() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("telemetry");
    let store = store(&root);
    task(&store, "t", "T1", "dispatches");
    dispatch(&store, "t", "impl-77c0", "implementation", None);
    dispatch(&store, "t", "review-90de", "review", Some("impl-77c0"));
    send_without_end(&store, dir.path(), "t", "impl-77c0");
    let mut page = open(&root);
    press(&mut page, KeyCode::Enter);
    shown(&mut page);
    press(&mut page, KeyCode::Tab);
    press(&mut page, KeyCode::Tab);
    let review = shown(&mut page);
    assert!(review.contains("parent impl-77c0"), "{review}");
    assert!(!review.contains("agent.send.begin"), "{review}");
    // Refresh while the dispatch list itself is being read again.
    press(&mut page, KeyCode::Char('r'));
    let refreshed = shown(&mut page);
    assert!(refreshed.contains("parent impl-77c0"), "{refreshed}");
    assert!(!refreshed.contains("agent.send.begin"), "{refreshed}");
    assert!(!refreshed.contains("brief.snapshot"), "{refreshed}");
    assert!(
        refreshed.contains("No events recorded for this dispatch."),
        "{refreshed}"
    );
    // Back to the implementation dispatch: its own events only.
    press(&mut page, KeyCode::BackTab);
    let implementation = shown(&mut page);
    assert!(
        implementation.contains("agent.send.begin"),
        "{implementation}"
    );
    assert!(
        implementation.contains("dispatch impl-77c0"),
        "{implementation}"
    );
}

#[test]
fn event_detail_shows_recorded_fields_and_scrolls_to_the_end_when_narrow() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("telemetry");
    let store = store(&root);
    task(&store, "t", "T1", "fields");
    dispatch(&store, "t", "impl-77c0", "implementation", None);
    transition(&store, "t", "move", "T1");
    let reason = file(dir.path(), "reason.txt", b"why");
    store
        .append(EventInput {
            schema_version: 1,
            event_id: "decide".into(),
            trace_id: "t".into(),
            dispatch_id: Some("impl-77c0".into()),
            operation_id: None,
            kind: "controller.decision".into(),
            observed_at: Value::Null,
            producer: "synthetic controller".into(),
            evidence_kind: "controller_statement".into(),
            payload: json!({"planned_model":"model-x","planned_effort":"high","validation_budget":null}),
            links: vec![],
            bodies: vec![serde_json::from_value(json!({"role":"reason","path":reason})).unwrap()],
        })
        .unwrap();
    let mut page = open(&root);
    press(&mut page, KeyCode::Enter);
    settle(&mut page);
    select_event(&mut page, "task.transition");
    let detail = shown(&mut page);
    assert!(detail.contains("from: pending"), "{detail}");
    assert!(detail.contains("to: running"), "{detail}");
    assert!(detail.contains("binding.key: T1"), "{detail}");
    assert!(detail.contains("event move"), "{detail}");
    select_event(&mut page, "controller.decision");
    let detail = shown(&mut page);
    assert!(detail.contains("planned_model: model-x"), "{detail}");
    assert!(detail.contains("planned_effort: high"), "{detail}");
    assert!(detail.contains("validation_budget: null"), "{detail}");
    assert!(detail.contains("dispatch impl-77c0"), "{detail}");

    // In a small window the end of a long detail is reachable without moving the selection.
    press(&mut page, KeyCode::Home);
    select_event(&mut page, "task.transition");
    let mut seen = screen(&mut page, 80, 24);
    for _ in 0..30 {
        if seen.contains("to: running") {
            break;
        }
        press(&mut page, KeyCode::Char('J'));
        seen = screen(&mut page, 80, 24);
    }
    assert!(seen.contains("to: running"), "{seen}");
    assert!(marked(&seen).contains("task.transition"), "{seen}");
}

#[test]
fn carried_from_names_the_target_trace_or_says_why_it_cannot() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("telemetry");
    let store = store(&root);
    task(&store, "source-trace-1234", "T1", "source");
    statement(
        &store,
        dir.path(),
        "source-trace-1234",
        "original",
        "requirement.recorded",
        b"text",
    );
    task(&store, "t", "T2", "reuser");
    store
        .append(EventInput {
            schema_version: 1,
            event_id: "reuse".into(),
            trace_id: "t".into(),
            dispatch_id: None,
            operation_id: None,
            kind: "evidence.reused".into(),
            observed_at: Value::Null,
            producer: "synthetic plugin".into(),
            evidence_kind: "plugin_statement".into(),
            payload: json!({"reused_by":"synthetic plugin"}),
            links: vec![
                serde_json::from_value(
                    json!({"relation":"carried_from","target_event_id":"original"}),
                )
                .unwrap(),
            ],
            bodies: vec![],
        })
        .unwrap();
    let mut page = open(&root);
    // The newest trace (the reuser) is first.
    press(&mut page, KeyCode::Enter);
    settle(&mut page);
    select_event(&mut page, "evidence.reused");
    let detail = shown(&mut page);
    assert!(detail.contains("carried_from → trace sour"), "{detail}");
    assert!(detail.contains("requirement.recorded"), "{detail}");

    // A failed read is reported as such, never as a guessed trace: this target ID is also
    // a trace ID, which show() refuses as ambiguous.
    let dir2 = tempfile::tempdir().unwrap();
    let root2 = dir2.path().join("telemetry");
    let store2 = crate::store(&root2);
    task(&store2, "source-trace-1234", "T1", "source");
    statement(
        &store2,
        dir2.path(),
        "source-trace-1234",
        "dup",
        "requirement.recorded",
        b"text",
    );
    store2
        .create_trace(
            serde_json::from_value(
                json!({"schema_version":1,"trace_id":"dup","origin":"ad_hoc","label":"same id"}),
            )
            .unwrap(),
        )
        .unwrap();
    task(&store2, "t", "T2", "reuser");
    store2
        .append(EventInput {
            schema_version: 1,
            event_id: "reuse".into(),
            trace_id: "t".into(),
            dispatch_id: None,
            operation_id: None,
            kind: "evidence.reused".into(),
            observed_at: Value::Null,
            producer: "synthetic plugin".into(),
            evidence_kind: "plugin_statement".into(),
            payload: json!({"reused_by":"synthetic plugin"}),
            links: vec![
                serde_json::from_value(json!({"relation":"carried_from","target_event_id":"dup"}))
                    .unwrap(),
            ],
            bodies: vec![],
        })
        .unwrap();
    let mut page = open(&root2);
    press(&mut page, KeyCode::Enter);
    settle(&mut page);
    select_event(&mut page, "evidence.reused");
    let failed = shown(&mut page);
    assert!(failed.contains("carried_from → event dup"), "{failed}");
    assert!(failed.contains("target trace not read"), "{failed}");
    assert!(failed.contains("ambiguous"), "{failed}");
    assert!(!failed.contains("trace sour"), "{failed}");
}

#[test]
fn a_prefilled_binding_with_control_characters_is_shown_escaped_and_queried_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("telemetry");
    let store = store(&root);
    let odd = "T1\u{1b}[31m\nnext";
    for (id, key) in [("odd", odd), ("stripped", "T1[31mnext"), ("t2", "T2")] {
        task(&store, id, key, &format!("{id} task"));
    }
    let filter = |key: &str| BindingFilter {
        kind: "drover.task".into(),
        scope: "/synthetic/proj".into(),
        key: key.into(),
        run: None,
    };
    let mut page = Page::open(
        Ok(Store::new(root.clone())),
        Some(filter(odd)),
        Some("Drover".into()),
    );
    let text = shown(&mut page);
    assert!(
        text.contains("1 trace") && text.contains("odd task"),
        "{text}"
    );
    assert!(text.contains(r"key=T1\x1b[31m\nnext (from D"), "{text}");
    // The form shows the same visible escapes and keeps the exact value.
    press(&mut page, KeyCode::Char('f'));
    let text = shown(&mut page);
    let field = text
        .lines()
        .find(|l| l.contains("kind ") && !l.contains("Filter:"))
        .unwrap_or_default()
        .to_owned();
    assert!(field.contains(r"T1\x1b[31m\nnext"), "{text}");
    assert!(!text.contains('\u{1b}'), "{text}");
    press(&mut page, KeyCode::Enter);
    let text = shown(&mut page);
    assert!(
        text.contains("1 trace") && text.contains("odd task"),
        "{text}"
    );
    assert!(!text.contains("stripped task"), "{text}");
    // Typing into that field replaces the whole value; the others edit as before.
    press(&mut page, KeyCode::Char('f'));
    press(&mut page, KeyCode::Tab);
    press(&mut page, KeyCode::Tab);
    typed(&mut page, "T2");
    press(&mut page, KeyCode::Enter);
    let text = shown(&mut page);
    assert!(
        text.contains("1 trace") && text.contains("t2 task"),
        "{text}"
    );
    assert!(text.contains("key=T2"), "{text}");
}
