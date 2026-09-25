mod common;
use saddle::drover::Client;
#[test]
fn queue_reads_public_json_in_the_configured_project_without_launching_a_board() {
    let temp = tempfile::tempdir().unwrap();
    let program = common::script(
        temp.path(),
        "drover",
        r#"#!/bin/sh
[ "$1" = list ] && [ "$2" = --json ] || exit 99
[ -f project-marker ] || exit 98
printf '%s\n' '{"mode":{"loop":true,"gate":true},"paused":true,"current":{"id":"T1","title":"Doing","body":"current body"},"awaiting":null,"pending":[{"id":null,"title":"中文任务","body":"task detail"}],"history":[{"id":"T0","title":"Past","status":"dropped","reason":"test"}]}'
"#,
    );
    std::fs::write(temp.path().join("project-marker"), "").unwrap();
    let snapshot = Client {
        program,
        cwd: temp.path().into(),
    }
    .snapshot()
    .unwrap();
    assert!(snapshot.paused && snapshot.mode.r#loop && snapshot.mode.gate);
    assert_eq!(snapshot.current.unwrap().id.as_deref(), Some("T1"));
    assert_eq!(snapshot.pending[0].title, "中文任务");
    assert_eq!(snapshot.pending[0].body, "task detail");
    assert_eq!(snapshot.history[0].reason.as_deref(), Some("test"));
}

#[test]
fn operations_use_literal_public_arguments_and_surface_failure_feedback() {
    use saddle::drover::Operation;
    use std::sync::atomic::AtomicBool;
    let temp = tempfile::tempdir().unwrap();
    let program = common::script(
        temp.path(),
        "drover",
        r#"#!/bin/sh
case "$1" in
 add) printf '%s\n%s' "$2" "$3" ;;
 pause|resume) printf '%s' "$1" ;;
 loop) printf 'loop %s' "$2" ;;
 go) echo 'criteria not met'; exit 9 ;;
 *) exit 99 ;;
esac
"#,
    );
    let client = Client {
        program,
        cwd: temp.path().into(),
    };
    let cancel = AtomicBool::new(false);
    let title = "Literal $(touch bad) 中文";
    let body = "first\nsecond";
    assert_eq!(
        client
            .execute(
                &Operation::Add {
                    title: title.into(),
                    body: body.into()
                },
                &cancel
            )
            .unwrap(),
        format!("{title}\n{body}")
    );
    assert!(!temp.path().join("bad").exists());
    for (op, expected) in [
        (Operation::Pause(true), "pause"),
        (Operation::Pause(false), "resume"),
        (Operation::Loop(true), "loop on"),
        (Operation::Loop(false), "loop off"),
    ] {
        assert_eq!(client.execute(&op, &cancel).unwrap(), expected);
    }
    assert!(
        client
            .execute(&Operation::Go, &cancel)
            .unwrap_err()
            .to_string()
            .contains("criteria not met")
    );
}
