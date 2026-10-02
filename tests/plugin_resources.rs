//! Built-in plugin resources through the management layer, page, palette and status.
//! Only synthetic plugins, temporary HOME/XDG directories and fake resources are used.
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent};
use saddle::plugins::{
    Manager,
    core::{Catalog, State},
    resources::{Class, Outcome, Resources, Trigger},
};
use saddle_core_plugin::{
    Call, Completion, CorePlugin, Manifest, Resource, ResourceFile, ResourceKind, SetupFile,
};
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

struct Fake(&'static Manifest);
impl CorePlugin for Fake {
    fn manifest(&self) -> &'static Manifest {
        self.0
    }
    fn run(&self, _: Call<'_>) -> Completion {
        Completion {
            exit_code: 0,
            stdout: vec![],
            end: None,
        }
    }
}
const NOTE: &str = "Synthetic setup note line one, provided by the plugin.\nSecond line of the synthetic setup note, kept verbatim by the host.\nThird line that only matters when there is room.";
const SETUP: &[SetupFile] = &[SetupFile {
    label: "Template",
    resource: "skill-a",
    path: "TEMPLATE.md",
}];
static OLD: Manifest = Manifest {
    id: "test.res",
    name: "Synthetic resources",
    version: "1",
    commands: &[],
    resources: &[Resource {
        kind: ResourceKind::AgentSkill,
        name: "skill-a",
        revision: 1,
        files: &[
            ResourceFile {
                path: "SKILL.md",
                bytes: b"skill v1",
            },
            ResourceFile {
                path: "TEMPLATE.md",
                bytes: b"template v1",
            },
        ],
    }],
    setup_note: NOTE,
    setup_files: SETUP,
};
static NEW: Manifest = Manifest {
    id: "test.res",
    name: "Synthetic resources",
    version: "1",
    commands: &[],
    resources: &[Resource {
        kind: ResourceKind::AgentSkill,
        name: "skill-a",
        revision: 2,
        files: &[
            ResourceFile {
                path: "SKILL.md",
                bytes: b"skill v2",
            },
            ResourceFile {
                path: "TEMPLATE.md",
                bytes: b"template v2",
            },
        ],
    }],
    setup_note: NOTE,
    setup_files: SETUP,
};
static OLD_PLUGIN: Fake = Fake(&OLD);
static NEW_PLUGIN: Fake = Fake(&NEW);
static OLD_CATALOG: Catalog = &[&OLD_PLUGIN];
static CATALOG: Catalog = &[&NEW_PLUGIN];

struct Env {
    dir: tempfile::TempDir,
}
impl Env {
    fn new() -> Self {
        let env = Self {
            dir: tempfile::tempdir().unwrap(),
        };
        fs::create_dir_all(env.home().join(".claude")).unwrap();
        fs::create_dir_all(env.home().join(".agents/skills")).unwrap();
        // Simulates the existing global link that must stay untouched.
        symlink("/synthetic/old-skill", env.codex()).unwrap();
        env
    }
    fn home(&self) -> PathBuf {
        self.dir.path().join("home")
    }
    fn claude(&self) -> PathBuf {
        self.home().join(".claude/skills/skill-a")
    }
    fn codex(&self) -> PathBuf {
        self.home().join(".agents/skills/skill-a")
    }
    fn registry(&self) -> PathBuf {
        self.dir.path().join("config/saddle/plugins.toml")
    }
    fn manager(&self, catalog: Catalog) -> Manager {
        Manager::with_resources(
            self.registry(),
            catalog,
            Resources::new(self.home(), self.dir.path().join("state/saddle")),
        )
    }
}
fn screen(
    page: &mut saddle::plugins::ui::Page,
    m: &Manager,
    settings: &saddle::settings::Settings,
    (w, h): (u16, u16),
) -> (String, ratatui::buffer::Buffer) {
    let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
    t.draw(|f| page.draw(&saddle::theme::Theme::default(), f, m, settings))
        .unwrap();
    let b = t.backend().buffer().clone();
    let text = (0..h)
        .map(|y| (0..w).map(|x| b[(x, y)].symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    (text, b)
}
/// Screen text with borders removed and wrapped lines joined by single spaces.
fn flat(text: &str) -> String {
    text.lines()
        .map(|l| l.replace(['┃', '│'], " ").trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
fn find(text: &str, needle: &str) -> Option<(u16, u16)> {
    text.lines().enumerate().find_map(|(y, line)| {
        line.find(needle)
            .map(|i| (line[..i].chars().count() as u16, y as u16))
    })
}
/// Buttons sit below the details, so the bottom-most match is the button.
fn find_last(text: &str, needle: &str) -> Option<(u16, u16)> {
    let lines: Vec<_> = text.lines().collect();
    lines.iter().enumerate().rev().find_map(|(y, line)| {
        line.find(needle)
            .map(|i| (line[..i].chars().count() as u16, y as u16))
    })
}
/// Click a visible button by its label; disabled buttons are inert.
fn click(
    page: &mut saddle::plugins::ui::Page,
    m: &mut Manager,
    settings: &saddle::settings::Settings,
    label: &str,
) {
    let (text, _) = screen(page, m, settings, (100, 48));
    let (x, y) = find_last(&text, label).unwrap_or_else(|| panic!("{label} not shown:\n{text}"));
    page.event(
        Event::Mouse(MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(MouseButton::Left),
            column: x + 1,
            row: y,
            modifiers: KeyModifiers::NONE,
        }),
        m,
    );
}
fn external_dir(env: &Env) -> (PathBuf, saddle::plugins::registry::Manifest) {
    let dir = env.dir.path().join("external");
    fs::create_dir(&dir).unwrap();
    symlink("/bin/echo", dir.join("entry")).unwrap();
    let manifest: saddle::plugins::registry::Manifest = serde_json::from_value(serde_json::json!({"manifest_version":1,"id":"test.external","name":"Synthetic external","version":"1","protocol_major":1,"executable":"entry","args":[],"required_capabilities":[]})).unwrap();
    fs::write(dir.join("plugin.toml"), toml::to_string(&manifest).unwrap()).unwrap();
    (dir, manifest)
}
fn external(env: &Env, m: &mut Manager) {
    let (dir, manifest) = external_dir(env);
    m.add(&dir, &manifest).unwrap();
}
fn target<'a>(
    receipt: &'a saddle::plugins::resources::Receipt,
    agent: &str,
) -> &'a saddle::plugins::resources::TargetReceipt {
    receipt.resources[0]
        .targets
        .iter()
        .find(|t| t.agent == agent)
        .unwrap()
}

#[test]
fn management_page_runs_builtin_resource_actions_and_keeps_external_rows() {
    let env = Env::new();
    let mut m = env.manager(CATALOG);
    external(&env, &mut m);
    let settings = saddle::settings::Settings::open(env.dir.path().join("config.toml"), true);
    let mut page = saddle::plugins::ui::Page::default();
    let (text, _) = screen(&mut page, &m, &settings, (100, 48));
    // The split list clips long names; the details pane titles the selected one in full.
    let builtin = find(&text, "› Synthetic resour").expect("built-in row");
    let ext = find(&text, "Synthetic extern").unwrap_or_else(|| panic!("external row:\n{text}"));
    assert!(builtin.1 < ext.1, "built-in rows come first:\n{text}");
    // Only the list column, left of the details pane.
    let row = text.lines().nth(builtin.1 as usize).unwrap();
    let row = row.split('│').next().unwrap();
    assert!(row.contains("No") && row.contains("Built-in"), "{row}");
    for shown in [
        " Synthetic resources ─",
        "Built-in · Disabled",
        "ID: test.res",
        "Skill: skill-a",
        "Not installed",
        "Conflict — existing link kept",
        "Second line of the synthetic setup note",
        "Template: not installed (see skill status above)",
        "Enable",
        "Sync resources",
        "Remove resources",
        "Refresh",
        "Back",
    ] {
        assert!(flat(&text).contains(shown), "missing {shown}:\n{text}");
    }
    for hidden in ["Open panel", "Restart"] {
        assert!(!text.contains(hidden), "{hidden} shown for a built-in row");
    }
    click(&mut page, &mut m, &settings, "Sync resources");
    assert!(m.core_receipt("test.res").is_none(), "Sync is disabled");

    click(&mut page, &mut m, &settings, "Enable");
    assert_eq!(m.core_state("test.res"), Some(State::Enabled));
    let receipt = m.core_receipt("test.res").expect("enable receipt");
    assert_eq!(receipt.trigger, Trigger::Enable);
    assert_eq!(target(receipt, "claude-code").result, Outcome::Installed);
    let codex = target(receipt, "codex");
    assert_eq!(
        (codex.before, codex.result),
        (Class::Foreign, Outcome::Conflict)
    );
    assert_eq!(
        fs::read_link(env.codex()).unwrap(),
        Path::new("/synthetic/old-skill")
    );
    let (text, _) = screen(&mut page, &m, &settings, (100, 48));
    let template = env.claude().join("TEMPLATE.md");
    for shown in [
        "Built-in · Enabled",
        "ID: test.res",
        "Installed",
        "Conflict — existing link kept",
        "Enabled. skill-a r2: Claude Code installed · Codex conflict (existing link kept).",
        "Disable",
        "Template: /",
        "TEMPLATE.md",
    ] {
        assert!(flat(&text).contains(shown), "missing {shown}:\n{text}");
    }
    assert!(!text.contains("not installed"));
    let resolved = m.core_resources("test.res").unwrap().as_ref().unwrap();
    assert_eq!(
        resolved.setup_files[0].paths,
        std::slice::from_ref(&template)
    );

    click(&mut page, &mut m, &settings, "Sync resources");
    assert_eq!(m.core_receipt("test.res").unwrap().trigger, Trigger::Sync);
    click(&mut page, &mut m, &settings, "Remove resources");
    assert!(env.claude().exists(), "Remove resources requires disabled");

    click(&mut page, &mut m, &settings, "Disable");
    assert_eq!(m.core_state("test.res"), Some(State::Disabled));
    assert_eq!(
        fs::read(&template).unwrap(),
        b"template v2",
        "Disable keeps skills"
    );
    let (text, _) = screen(&mut page, &m, &settings, (100, 48));
    assert!(text.contains("Installed resources are kept"), "{text}");

    click(&mut page, &mut m, &settings, "Remove resources");
    let receipt = m.core_receipt("test.res").unwrap();
    assert_eq!(receipt.trigger, Trigger::Remove);
    assert_eq!(target(receipt, "claude-code").result, Outcome::Removed);
    assert!(!env.claude().exists());
    assert!(fs::symlink_metadata(env.codex()).unwrap().is_symlink());

    // External rows keep their actions and details.
    page.event(
        Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        &mut m,
    );
    let (text, _) = screen(&mut page, &m, &settings, (100, 48));
    for shown in ["Open panel", "Restart", "Add local…", "ID: test.external"] {
        assert!(flat(&text).contains(shown), "missing {shown}:\n{text}");
    }
    assert!(!text.contains("Sync resources"));
}

#[test]
fn narrow_management_page_prioritizes_switch_and_resource_summary() {
    let env = Env::new();
    let mut m = env.manager(CATALOG);
    m.core_enabled("test.res", true).unwrap();
    let settings = saddle::settings::Settings::open(env.dir.path().join("config.toml"), true);
    let mut page = saddle::plugins::ui::Page::default();
    let (text, _) = screen(&mut page, &m, &settings, (60, 30));
    for shown in [
        "Disable",
        "Built-in · Enabled",
        "Codex",
        "Conflict",
        "Third line",
    ] {
        assert!(flat(&text).contains(shown), "missing {shown}:\n{text}");
    }
    // Long details scroll inside the page instead of hiding the note; the switch stays put.
    for _ in 0..4 {
        page.event(
            Event::Key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
            &mut m,
        );
    }
    let (text, _) = screen(&mut page, &m, &settings, (60, 30));
    for shown in ["Enabled. skill-a r2", "Disable"] {
        assert!(flat(&text).contains(shown), "missing {shown}:\n{text}");
    }
    for size in [(1, 1), (10, 3), (20, 5), (40, 12)] {
        screen(&mut page, &m, &settings, size);
    }
    // Add local… stays reachable from a built-in row and selects the new external row.
    let (dir, _) = external_dir(&env);
    click(&mut page, &mut m, &settings, "Add local…");
    page.event(Event::Paste(dir.display().to_string()), &mut m);
    click(&mut page, &mut m, &settings, "Read manifest");
    click(&mut page, &mut m, &settings, "Add disabled");
    let (text, _) = screen(&mut page, &m, &settings, (100, 48));
    assert!(text.contains("› Synthetic extern"), "{text}");
}

#[test]
fn plugins_palette_lists_builtin_and_enter_opens_its_management_row() {
    use saddle::plugins::palette::{Outcome as PaletteOutcome, Palette};
    let env = Env::new();
    let mut m = env.manager(CATALOG);
    external(&env, &mut m);
    let items = m.palette_items(&Default::default());
    assert_eq!(items[0].id, "test.res");
    assert!(items[0].builtin);
    assert_eq!(items[0].status(), "Disabled");
    assert!(items[0].action().is_none(), "no view to open");
    assert!(!items[0].explanation().is_empty());
    let mut palette = Palette::default();
    palette.update(items);
    let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
    t.draw(|f| palette.draw(f, &Default::default())).unwrap();
    let enter = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(palette.event(&enter), PaletteOutcome::Manage);
    assert_eq!(palette.selected_id(), Some("test.res"));

    let settings = saddle::settings::Settings::open(env.dir.path().join("config.toml"), true);
    let mut page = saddle::plugins::ui::Page::default();
    page.select_plugin("test.external", &m);
    let (text, _) = screen(&mut page, &m, &settings, (100, 48));
    assert!(text.contains("› Synthetic extern"), "{text}");
    assert!(text.contains(" Synthetic external ─"), "{text}");
    page.select_plugin("test.res", &m);
    let (text, _) = screen(&mut page, &m, &settings, (100, 48));
    assert!(text.contains("› Synthetic resour"), "{text}");
    assert!(text.contains(" Synthetic resources ─"), "{text}");
}

#[test]
fn startup_upgrades_only_enabled_builtin_owned_outdated_resources() {
    let env = Env::new();
    // Enabled without any recorded install: startup creates nothing.
    fs::create_dir_all(env.registry().parent().unwrap()).unwrap();
    fs::write(
        env.registry(),
        "version=1\nplugins=[]\n[core.'test.res']\nenabled=true\n",
    )
    .unwrap();
    let m = env.manager(CATALOG);
    assert!(m.core_receipt("test.res").is_none());
    assert!(!env.home().join(".claude/skills").exists());
    assert!(!env.dir.path().join("state").exists());
    drop(m);
    fs::remove_file(env.registry()).unwrap();

    let mut old = env.manager(OLD_CATALOG);
    old.core_enabled("test.res", true).unwrap();
    assert_eq!(
        fs::read(env.claude().join("SKILL.md")).unwrap(),
        b"skill v1"
    );
    old.core_enabled("test.res", false).unwrap();
    drop(old);
    let disabled = env.manager(CATALOG);
    assert!(disabled.core_receipt("test.res").is_none());
    assert_eq!(
        fs::read(env.claude().join("SKILL.md")).unwrap(),
        b"skill v1",
        "disabled plugins are not upgraded"
    );
    let mut disabled = disabled;
    disabled.core_enabled("test.res", true).unwrap();
    assert_eq!(
        target(disabled.core_receipt("test.res").unwrap(), "claude-code").result,
        Outcome::Updated,
        "explicit Enable upgrades"
    );
    drop(disabled);

    let mut old = env.manager(OLD_CATALOG);
    assert!(
        old.core_receipt("test.res").is_none(),
        "no downgrade at startup"
    );
    assert_eq!(
        fs::read(env.claude().join("SKILL.md")).unwrap(),
        b"skill v2"
    );
    old.core_sync("test.res").unwrap();
    assert_eq!(
        target(old.core_receipt("test.res").unwrap(), "claude-code").before,
        Class::Newer
    );
    // Simulate the previous build's install, then start the new build.
    fs::remove_dir_all(env.claude()).unwrap();
    fs::remove_dir_all(env.dir.path().join("state")).unwrap();
    old.core_sync("test.res").unwrap();
    drop(old);
    let m = env.manager(CATALOG);
    let receipt = m
        .core_receipt("test.res")
        .expect("startup upgrade reported");
    assert_eq!(receipt.trigger, Trigger::Startup);
    assert_eq!(target(receipt, "claude-code").result, Outcome::Updated);
    assert_eq!(
        fs::read(env.claude().join("SKILL.md")).unwrap(),
        b"skill v2"
    );
}

// Subprocess-only seam for the headless status entry; never installed in production.
#[test]
fn resource_status_driver() {
    let Some(args) = std::env::var_os("TEST_RESOURCE_ARGS") else {
        return;
    };
    let output = fs::File::create(std::env::var_os("TEST_OUTPUT").unwrap()).unwrap();
    use std::os::fd::AsRawFd;
    assert_eq!(
        unsafe { libc::dup2(output.as_raw_fd(), libc::STDOUT_FILENO) },
        libc::STDOUT_FILENO
    );
    let args: Vec<String> = serde_json::from_str(args.to_str().unwrap()).unwrap();
    std::process::exit(saddle::plugins::cli::run(
        &args.into_iter().map(Into::into).collect::<Vec<_>>(),
        CATALOG,
    ));
}
fn status(env: &Env) -> (Option<i32>, Value) {
    let out = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "resource_status_driver", "--nocapture"])
        .env("TEST_RESOURCE_ARGS", r#"["status","test.res"]"#)
        .env("TEST_OUTPUT", env.dir.path().join("output"))
        .env("HOME", env.home())
        .env("XDG_CONFIG_HOME", env.dir.path().join("config"))
        .env("XDG_STATE_HOME", env.dir.path().join("state"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    let value = serde_json::from_slice(&fs::read(env.dir.path().join("output")).unwrap())
        .unwrap_or(Value::Null);
    (out.code(), value)
}

#[test]
fn status_lists_resource_states_and_setup_files_read_only() {
    let env = Env::new();
    let (code, value) = status(&env);
    assert_eq!(code, Some(0));
    let core = &value["core"][0];
    assert!(core.get("installed").is_none());
    let resource = &core["resources"][0];
    assert_eq!(resource["kind"], "agent_skill");
    assert_eq!(resource["revision"], 2);
    assert!(
        resource["fingerprint"]
            .as_str()
            .unwrap()
            .starts_with("sha256:")
    );
    let targets = resource["targets"].as_array().unwrap();
    assert_eq!(targets[0]["agent"], "claude-code");
    assert_eq!(targets[0]["state"], "absent");
    assert_eq!(targets[1]["state"], "foreign");
    assert_eq!(targets[1]["reason"], "symlink");
    assert_eq!(targets[1]["detail"], "-> /synthetic/old-skill");
    assert_eq!(core["setup_note"], NOTE);
    assert_eq!(core["setup_files"][0]["label"], "Template");
    assert_eq!(core["setup_files"][0]["paths"], serde_json::json!([]));
    assert!(
        !env.dir.path().join("state").exists(),
        "status is read-only"
    );
    assert!(!env.home().join(".claude/skills").exists());

    env.manager(CATALOG).core_enabled("test.res", true).unwrap();
    let (code, value) = status(&env);
    assert_eq!(code, Some(0));
    let core = &value["core"][0];
    assert_eq!(core["state"], "enabled");
    assert_eq!(core["resources"][0]["targets"][0]["state"], "owned_current");
    assert_eq!(core["resources"][0]["targets"][0]["revision"], 2);
    assert_eq!(
        core["setup_files"][0]["paths"],
        serde_json::json!([env.claude().join("TEMPLATE.md")])
    );
    fs::write(
        env.dir.path().join("state/saddle/plugin-resources.json"),
        "{broken",
    )
    .unwrap();
    assert_eq!(status(&env).0, Some(1));
}
