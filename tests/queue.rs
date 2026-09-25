use crossterm::event::{KeyCode as K, KeyEvent, KeyModifiers as M};
use saddle::{
    drover::{Operation, Request, Snapshot},
    queue::{Page, Panel},
};
fn key(code: K) -> KeyEvent {
    KeyEvent::new(code, M::NONE)
}
#[test]
fn native_queue_selects_details_and_creates_tasks_without_an_external_editor() {
    let mut panel = Panel::default();
    panel.absorb(serde_json::from_str::<Snapshot>(r#"{"mode":{"loop":false,"gate":true},"paused":false,"current":null,"awaiting":null,"history":[],"pending":[{"id":"T1","title":"First","body":"first body"},{"id":"T2","title":"Second","body":"second body"}]}"#).unwrap());
    panel.key(key(K::Down));
    assert_eq!(panel.selected, 1);
    panel.key(key(K::Enter));
    assert!(matches!(panel.page, Page::Detail));
    panel.key(key(K::Esc));
    panel.key(key(K::Char('a')));
    for c in "中文新增".chars() {
        panel.key(key(K::Char(c)));
    }
    panel.key(key(K::Tab));
    for c in "正文".chars() {
        panel.key(key(K::Char(c)));
    }
    panel.key(key(K::Enter));
    panel.key(key(K::Char('b')));
    let request = panel.key(KeyEvent::new(K::Char('s'), M::CONTROL));
    assert!(
        matches!(request,Some(Request::Run(Operation::Add{title,body})) if title=="中文新增" && body=="正文\nb")
    );
    assert!(panel.busy);
    assert!(matches!(panel.page, Page::Add { .. }));
    panel.complete(
        &Operation::Add {
            title: "中文新增".into(),
            body: "正文\nb".into(),
        },
        Ok("added".into()),
    );
    assert!(matches!(panel.page, Page::List));
}

#[test]
fn failed_add_retains_the_native_draft_and_prevents_duplicate_submission() {
    let mut panel = Panel::default();
    panel.key(key(K::Char('a')));
    panel.paste("Task with 中文");
    panel.key(key(K::Tab));
    panel.paste("first\nsecond");
    let save = KeyEvent::new(K::Char('s'), M::CONTROL);
    let Some(Request::Run(op)) = panel.key(save) else {
        panic!("missing add request");
    };
    assert!(panel.key(save).is_none());
    panel.complete(&op, Err(anyhow::anyhow!("write failed")));
    assert!(panel.message.contains("write failed"));
    assert!(
        matches!(panel.key(save),Some(Request::Run(Operation::Add{title,body})) if title=="Task with 中文" && body=="first\nsecond")
    );
}
