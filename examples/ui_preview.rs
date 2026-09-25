//! Render synthetic Ratatui buffers for visual review without corral, drover or agents.
use ratatui::{
    Terminal,
    backend::TestBackend,
    layout::Rect,
    style::{Color, Modifier},
};
use saddle::{
    agents,
    buttons::Pointer,
    config::Config,
    corral::Agent,
    drover::{Snapshot, Task},
    input::Focus,
    layout::Panes,
    queue,
    terminal::{Screen, Size},
    ui::{self, View},
};
use std::{fmt::Write, path::PathBuf};
use unicode_width::UnicodeWidthStr;
fn main() -> anyhow::Result<()> {
    let dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "/tmp/saddle-ui-preview".into()),
    );
    std::fs::create_dir_all(&dir)?;
    for (name, w, h, focus, overlay) in [
        ("wide", 160, 48, Focus::Agents, ""),
        ("many-agents", 160, 100, Focus::Agents, "many"),
        ("medium", 120, 36, Focus::Queue, ""),
        ("narrow-agents", 80, 24, Focus::Agents, ""),
        ("narrow-queue", 80, 24, Focus::Queue, ""),
        ("projects", 120, 36, Focus::Queue, "projects"),
        ("add", 120, 36, Focus::Queue, "add"),
        ("stop", 120, 36, Focus::Agents, "stop"),
        ("reply", 120, 36, Focus::Agents, "reply"),
    ] {
        let mut a = agents::Panel {
            follow: true,
            ..Default::default()
        };
        a.absorb(
            [
                (
                    "corral/docs",
                    "claude",
                    "working",
                    "Edit docs/attach.md",
                    96.0,
                ),
                ("corral/main", "claude", "idle", "", 40.0),
                ("drover/main", "codex", "blocked", "等待确认项目设置", 86.0),
                ("drover/loop", "codex", "working", "cargo test", 0.0),
                ("saddle/main", "codex", "working", "界面重设计", 96.0),
            ]
            .into_iter()
            .map(|(name, kind, state, tool, last)| Agent {
                name: name.into(),
                kind: Some(kind.into()),
                state: Some(state.into()),
                instance: Some("abcdef123".into()),
                cwd: Some(format!("~/Developer/{}", name.split('/').next().unwrap())),
                title: Some("原生界面设计与交互验收".into()),
                last_tool: Some(tool.into()),
                last_input_source: Some("human".into()),
                last_output: Some(last),
                turn_started: Some(last),
                attached: if name == "saddle/main" { 1 } else { 0 },
                ..Default::default()
            })
            .collect(),
            None,
            150.0,
        );
        if overlay == "many" {
            for (name, kind) in [
                ("claude-demo-1", "claude"),
                ("omp-demo-1", "omp"),
                ("pi-demo-1", "pi"),
            ] {
                a.agents.push(Agent {
                    name: format!("saddle/{name}"),
                    kind: Some(kind.into()),
                    state: Some("idle".into()),
                    instance: Some("abcdef123".into()),
                    cwd: Some("~/Developer/saddle".into()),
                    title: Some(format!("{kind} demo is ready")),
                    last_output: Some(140.0),
                    ..Default::default()
                });
            }
        }
        a.select(Some("saddle/main".into()));
        a.unread.insert("corral/main".into());
        let mut q = queue::Panel::default();
        q.project = "~/Developer/drover".into();
        q.projects = vec![
            "~/Developer/drover".into(),
            "~/Developer/saddle".into(),
            "~/Developer/corral".into(),
        ];
        q.absorb(Snapshot {
            current: Some(Task {
                id: Some("T23".into()),
                title: "实现待办详情中的中文换行".into(),
                body: "按终端显示宽度换行，并保留任务状态。".into(),
                ..Default::default()
            }),
            awaiting: Some(Task {
                id: Some("T24".into()),
                title: "修复循环模式下的任务放行".into(),
                ..Default::default()
            }),
            pending: vec![Task {
                id: Some("T25".into()),
                title: "Queue 支持历史任务完整展示".into(),
                ..Default::default()
            }],
            history: (16..23)
                .map(|i| Task {
                    id: Some(format!("T{i}")),
                    title: "完善看板的交互与测试".into(),
                    status: Some("done".into()),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        });
        match overlay {
            "projects" => q.page = queue::Page::Projects,
            "add" => {
                q.page = queue::Page::Add {
                    title: "新增任务的中文标题".into(),
                    body: "第一行正文\n第二行正文".into(),
                    body_focus: true,
                }
            }
            "stop" => a.confirm = Some("saddle/main".into()),
            "reply" => a.show_reply = true,
            _ => {}
        }
        let panes = Panes::with_queue(
            Rect::new(0, 0, w, h),
            &Config::default(),
            focus == Focus::Queue,
        );
        let mut terminal = Terminal::new(TestBackend::new(w, h))?;
        terminal.draw(|frame| {
            ui::draw(frame,&mut a,View{panes,focus,showing:Some("saddle/main"),viewer:None,queue:&mut q,viewer_note:"",reply:"已完成布局与交互重设计。\n\n- 管理区域全部由 Rust + Ratatui 绘制\n- Viewer 保留原终端颜色与按键\n- 自动验证使用合成数据",now:150.0,pointer:&Pointer::default()});
            if overlay.is_empty() || overlay=="reply" {
                let area=ui::inner(panes.viewer);
                let mut screen=Screen::new(Size{rows:area.height,cols:area.width});
                screen.process("\x1b[1m› saddle 界面重设计\x1b[0m\r\n\r\n\x1b[32m●\x1b[0m Explored\r\n  Read src/ui.rs, src/queue.rs, src/buttons.rs\r\n\r\n\x1b[32m●\x1b[0m Ran cargo test\r\n  test result: ok. Synthetic checks passed\r\n\r\n\x1b[32m●\x1b[0m Edited src/ui.rs\r\n\x1b[48;2;25;50;32m  + 中文状态列保持对齐，按钮有悬停和按下反馈。\x1b[0m\r\n\r\n继续检查窄窗口和弹层输入边界。\r\n".as_bytes());
                screen.render(area,frame.buffer_mut());
            }
        })?;
        let buffer = terminal.backend().buffer();
        let mut svg = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='{}' height='{}'><rect width='100%' height='100%' fill='#141619'/><g font-family='Menlo, PingFang SC, monospace' font-size='14'>",
            w * 9,
            h * 19
        );
        let mut text = String::new();
        for y in 0..h {
            let mut x = 0;
            while x < w {
                let cell = &buffer[(x, y)];
                let symbol = cell.symbol();
                let width = symbol.width().max(1) as u16;
                let fg = color(cell.fg, "#d5d8dd");
                let bg = color(cell.bg, "#141619");
                write!(
                    svg,
                    "<rect x='{}' y='{}' width='{}' height='19' fill='{bg}'/>",
                    x * 9,
                    y * 19,
                    width * 9
                )?;
                if !symbol.trim().is_empty() {
                    write!(
                        svg,
                        "<text x='{}' y='{}' fill='{fg}' {}>{}</text>",
                        x * 9,
                        y * 19 + 15,
                        if cell.modifier.contains(Modifier::BOLD) {
                            "font-weight='bold'"
                        } else {
                            ""
                        },
                        symbol
                            .replace('&', "&amp;")
                            .replace('<', "&lt;")
                            .replace('>', "&gt;")
                    )?;
                }
                text.push_str(symbol);
                x += width;
            }
            text.push('\n');
        }
        svg.push_str("</g></svg>");
        std::fs::write(dir.join(format!("{name}.svg")), svg)?;
        std::fs::write(dir.join(format!("{name}.txt")), text)?;
    }
    println!("{}", dir.display());
    Ok(())
}
fn color(color: Color, fallback: &str) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        Color::Black => "#000000".into(),
        Color::Red => "#e26a6a".into(),
        Color::Green => "#86c07a".into(),
        Color::Yellow => "#e2b262".into(),
        Color::Blue => "#6fa6e3".into(),
        Color::Magenta => "#be96e6".into(),
        Color::Cyan => "#62c3c0".into(),
        Color::White => "#ffffff".into(),
        // Preview-only ANSI palette; the real terminal supplies its own colors.
        Color::Gray => "#c0c0c0".into(),
        Color::DarkGray => "#606060".into(),
        _ => fallback.into(),
    }
}
