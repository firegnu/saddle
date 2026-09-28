use super::*;
use crate::control::{self, Caller, CloseTarget, Content, Message, Operation};
use anyhow::{Context, ensure};
use serde_json::{Value, json};

pub(super) struct Record {
    message: Message,
    ticket: Option<Ticket>,
    pub value: Value,
}
#[derive(Clone)]
pub(super) struct Closing {
    pub target: CloseTarget,
    pub snapshot: Value,
    pub replacement: Option<Replacement>,
    pub scroll: u16,
}
#[derive(Clone)]
pub(super) enum Replacement {
    Attach(String),
    TaskLink(crate::links::AgentRequest),
    Start(Vec<String>),
}
impl App {
    fn caller_pane(&self, caller: &Caller) -> Result<u64> {
        if caller.name.is_some() || caller.corral_instance.is_some() {
            let name = caller.name.as_deref().context("missing CORRAL_NAME")?;
            let instance = caller
                .corral_instance
                .as_deref()
                .context("missing CORRAL_INSTANCE")?;
            ensure!(
                self.panel.agents.iter().any(|a| a.name == name
                    && a.instance.as_deref() == Some(instance)
                    && a.error.is_none()),
                "corral identity not present in the public listing"
            );
            let panes: Vec<_> = self
                .viewer
                .tabs
                .iter()
                .flat_map(|t| &t.panes)
                .filter(|p| {
                    p.viewer.showing.as_deref() == Some(name)
                        && p.viewer.metadata.instance.as_deref() == Some(instance)
                        && !p.replacing()
                        && p.input_session().is_some()
                })
                .collect();
            ensure!(
                panes.len() == 1,
                "caller pane missing, replacing or ambiguous"
            );
            return Ok(panes[0].id);
        }
        ensure!(
            caller.saddle_instance.as_deref() == Some(&self.control.id),
            "caller has no identity in this instance"
        );
        let pane = caller
            .pane
            .and_then(|p| self.viewer.get(p))
            .context("caller pane missing")?;
        ensure!(
            caller.revision == Some(pane.ticket().revision)
                && pane.viewer.shell_live()
                && !pane.replacing(),
            "stale shell pane identity"
        );
        Ok(pane.id)
    }
    fn inspect(&self, caller: &Caller) -> Value {
        let caller = match self.caller_pane(caller) {
            Ok(pane) => {
                json!({"pane":pane,"revision":self.viewer.get(pane).unwrap().ticket().revision})
            }
            Err(e) => control::error("caller_unresolved", e),
        };
        let tabs: Vec<_> = self.viewer.tabs.iter().map(|tab| {
            let panes: Vec<_> = tab.panes.iter().map(|p| {
                let shell = p.viewer.shell.as_ref();
                let kind = if shell.is_some() || matches!(p.viewer.remembered, crate::layout_state::Content::Shell { .. }) { "shell" } else if p.viewer.target().is_some() || p.requested().is_some() || p.viewer.remembered.name().is_some() { "agent" } else { "empty" };
                json!({"id":p.id,"revision":p.ticket().revision,"kind":kind,
                    "agent":p.requested().or(p.viewer.target()).or(p.viewer.remembered.name()),"corral_instance":p.viewer.metadata.instance,
                    "cwd":p.source_cwd().unwrap_or(&self.queue.project),"cwd_source":if p.source_cwd().is_some() {"pane"} else {"tasks_project"},
                    "state":if p.starting {"starting"} else if p.requested().is_some() {"accepted"} else {p.viewer.state()},
                    "shell":shell.map(|s| json!({"program":s.program,"exit_code":s.exit_code})),"exit_code":p.viewer.exit_code,"note":p.viewer.note})
            }).collect();
            json!({"id":tab.id,"active_pane":tab.active,"layout":tab.layout(),"panes":panes})
        }).collect();
        json!({"ok":true,"instance":self.control.id,"active_tab":self.viewer.active,
            "active_pane":self.viewer.active_pane().id,"focus":format!("{:?}",self.focus).to_lowercase(),"caller":caller,"tabs":tabs})
    }
    pub(super) fn control_tick(&mut self) {
        for record in &mut self.records {
            if !matches!(
                record.value["state"].as_str(),
                Some("accepted" | "starting" | "attaching" | "complete")
            ) {
                continue;
            }
            let Some(ticket) = record.ticket else {
                continue;
            };
            if !self.viewer.valid(ticket) {
                if record.value["state"] != "starting" {
                    record.value["state"] = json!("target_invalid");
                    record.value["ok"] = json!(false);
                    record.value["error"] = json!({"code":"target_invalid","message":"display target closed or replaced"});
                }
                record.value["pty"] = json!("target_invalid");
            } else if record.value["state"] != "starting" {
                let pane = self.viewer.get(ticket.pane).unwrap();
                if pane.requested().is_some() {
                    continue;
                }
                record.value["exit_code"] = json!(
                    pane.viewer
                        .shell
                        .as_ref()
                        .and_then(|s| s.exit_code)
                        .or(pane.viewer.exit_code)
                );
                match pane.viewer.state() {
                    "running" | "exited" => {
                        record.value["state"] = json!("complete");
                        record.value["pty"] = json!(pane.viewer.state());
                    }
                    "failed" | "disconnected" => {
                        record.value["state"] = json!("failed");
                        record.value["pty"] = json!("failed");
                        record.value["ok"] = json!(false);
                        record.value["error"] =
                            json!({"code":"pty_failed","message":pane.viewer.note});
                    }
                    _ => {}
                }
            }
        }
        let messages: Vec<_> = self.control.incoming.try_iter().take(32).collect();
        for incoming in messages {
            let value = self.control_message(incoming.message);
            let _ = incoming.reply.try_send(value);
        }
    }
    fn control_message(&mut self, message: Message) -> Value {
        if message.instance != self.control.id {
            return control::error("instance_unavailable", "instance has exited or restarted");
        }
        match &message.operation {
            Operation::Inspect => return self.inspect(&message.caller),
            Operation::Request { request } => {
                return self
                    .records
                    .iter()
                    .find(|r| r.message.request_id.as_ref() == Some(request))
                    .map(|r| r.value.clone())
                    .unwrap_or_else(|| {
                        control::error(
                            "request_unavailable",
                            "request not recorded in this instance",
                        )
                    });
            }
            _ => {}
        }
        let Some(id) = message
            .request_id
            .as_ref()
            .filter(|s| !s.is_empty() && s.len() <= 128)
        else {
            return control::error("invalid_request", "request_id must contain 1–128 bytes");
        };
        if let Some(record) = self
            .records
            .iter()
            .find(|r| r.message.request_id.as_ref() == Some(id))
        {
            return if record.message == message {
                record.value.clone()
            } else {
                control::error(
                    "request_conflict",
                    "request ID already used with different parameters or caller",
                )
            };
        }
        if self.placement.is_some()
            || self.search.is_some()
            || self.settings.is_some()
            || self.new_agent.as_ref().is_some_and(|f| f.visible)
            || self.closing.is_some()
            || self.panel.confirm.is_some()
        {
            return control::error("busy", "finish the current layout/form/confirmation first");
        }
        if self.records.len() >= control::RECORD_LIMIT {
            return control::error(
                "request_limit",
                "instance request capacity reached; existing requests remain queryable",
            );
        }
        let mut value = json!({"ok":true,"instance":self.control.id,"request_id":id,"accepted":true,"state":"accepted","agent_created":null,"pty":"not_started"});
        let result: Result<Option<Ticket>> = (|| match &message.operation {
            Operation::Open {
                relative_to,
                place,
                content,
                focus,
            } => {
                ensure!(*place != Place::Current, "open does not replace panes");
                let anchor = match relative_to.as_str() {
                    "self" => self.caller_pane(&message.caller)?,
                    "active" => self.viewer.active_pane().id,
                    id => id
                        .parse()
                        .context("relative-to must be self, active or a pane ID")?,
                };
                let source = self.viewer.get(anchor).context("target pane disappeared")?;
                value["relative_to"] = json!({"pane":anchor,"revision":source.ticket().revision});
                let (ticket, cwd_source, moved) =
                    self.open_content(anchor, *place, content.clone(), *focus)?;
                value["pane"] = json!(ticket.pane);
                value["revision"] = json!(ticket.revision);
                value["cwd"] = json!(self.viewer.get(ticket.pane).and_then(|p| p.source_cwd()));
                value["cwd_source"] = json!(cwd_source);
                value["state"] = json!(if moved
                    && self.viewer.get(ticket.pane).unwrap().viewer.state() == "running"
                {
                    "complete"
                } else if matches!(content, Content::NewAgent { .. }) {
                    "starting"
                } else {
                    "attaching"
                });
                value["pty"] = json!(if moved {
                    self.viewer.get(ticket.pane).unwrap().viewer.state()
                } else {
                    "pending"
                });
                Ok(Some(ticket))
            }
            Operation::Close {
                target,
                confirmation,
                confirm_shells,
            } => {
                ensure!(
                    *target != CloseTarget::All,
                    "ctl close requires a pane or tab"
                );
                let snapshot = self.close_snapshot(*target)?;
                if *confirm_shells || confirmation.is_some() {
                    ensure!(*confirm_shells, "confirmation requires --confirm-shells");
                    let token = confirmation
                        .as_ref()
                        .context("--confirm-shells needs --confirmation")?;
                    let previous = self
                        .records
                        .iter()
                        .find(|r| r.value["confirmation"].as_str() == Some(token))
                        .context("confirmation not found")?;
                    ensure!(
                        previous.value["target"] == json!(target)
                            && previous.value["targets"] == snapshot,
                        "confirmation target changed"
                    );
                    self.close_target(*target)?;
                    value["state"] = json!("complete");
                } else if snapshot
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["running_shell"] == true)
                {
                    value["state"] = json!("confirmation_required");
                    value["confirmation"] = json!(control::random_id()?);
                    value["target"] = json!(target);
                    value["targets"] = snapshot;
                } else {
                    self.close_target(*target)?;
                    value["state"] = json!("complete");
                }
                Ok(None)
            }
            _ => unreachable!(),
        })();
        match result {
            Ok(ticket) => self.records.push(Record {
                message,
                ticket,
                value: value.clone(),
            }),
            Err(e) => {
                value["ok"] = json!(false);
                value["accepted"] = json!(false);
                value["state"] = json!("failed");
                value["error"] = json!({"code":"invalid_target","message":format!("{e:#}")});
                self.records.push(Record {
                    message,
                    ticket: None,
                    value: value.clone(),
                });
            }
        }
        value
    }
    pub(super) fn open_content(
        &mut self,
        anchor: u64,
        place: Place,
        content: Content,
        focus: bool,
    ) -> Result<(Ticket, &'static str, bool)> {
        let source = self.viewer.get(anchor).context("target pane disappeared")?;
        let agent_content = matches!(&content, Content::Agent { .. });
        let explicit = match &content {
            Content::Shell { cwd } | Content::NewAgent { cwd, .. } => cwd.clone(),
            _ => None,
        };
        let cwd_source = if explicit.is_some() {
            "explicit"
        } else if source.source_cwd().is_some() {
            "source_pane"
        } else {
            "tasks_project"
        };
        let cwd = explicit
            .or_else(|| source.source_cwd().map(str::to_owned))
            .unwrap_or_else(|| self.queue.project.clone());
        if !matches!(content, Content::Agent { .. }) {
            ensure!(
                std::path::Path::new(&cwd).is_absolute() && std::path::Path::new(&cwd).is_dir(),
                "cwd must be an existing absolute directory"
            );
        }
        if let Content::NewAgent {
            name, role, argv, ..
        } = &content
        {
            ensure!(
                !name.is_empty()
                    && !name.starts_with('-')
                    && !name.chars().any(char::is_whitespace),
                "invalid full agent name"
            );
            ensure!(
                ["regular", "controller", "implementer", "reviewer"].contains(&role.as_str()),
                "invalid role"
            );
            ensure!(
                argv.first().is_some_and(|a| !a.is_empty()),
                "program required"
            );
        }
        let saved = self.viewer.focus_snapshot();
        let mut moved = false;
        let ticket = match content {
            Content::Agent { name } => {
                ensure!(
                    !name.is_empty() && !name.starts_with('-'),
                    "invalid agent name"
                );
                let agent = self
                    .panel
                    .agents
                    .iter()
                    .find(|a| a.name == name)
                    .context("agent not in public listing")?;
                let identity = agent.instance.clone().context("agent instance unknown")?;
                let cwd = agent.cwd.clone();
                if let Some(id) = self.viewer.find(&name) {
                    let pane = self.viewer.get(id).unwrap();
                    let metadata = if pane.viewer.showing.as_deref() == Some(&name) {
                        &pane.viewer.metadata
                    } else if pane.requested() == Some(&name) {
                        &pane.pending_agent
                    } else {
                        pane.viewer.target_metadata()
                    };
                    ensure!(
                        metadata
                            .instance
                            .as_ref()
                            .is_none_or(|old| old == &identity),
                        "agent identity changed"
                    );
                }
                match self.viewer.place(anchor, place, &name)? {
                    Some(ticket) => {
                        let pane = self.viewer.get_mut(ticket.pane).unwrap();
                        pane.pending_agent.cwd = cwd;
                        pane.pending_agent.instance = Some(identity);
                        self.actions.start(Action::Attach(name, ticket, None));
                        ticket
                    }
                    None => {
                        moved = true;
                        self.viewer
                            .get(self.viewer.find(&name).unwrap())
                            .unwrap()
                            .ticket()
                    }
                }
            }
            Content::Shell { .. } => {
                let ticket = self.viewer.reserve_at(anchor, place, None);
                self.viewer.complete(ticket, None)?;
                let pane = self.viewer.get_mut(ticket.pane).unwrap();
                pane.viewer.start_shell(crate::viewer::Shell {
                    program: std::env::var("SHELL")
                        .ok()
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| "/bin/sh".into()),
                    cwd,
                    state: "starting",
                    exit_code: None,
                    env: vec![
                        ("SADDLE_INSTANCE".into(), self.control.id.clone()),
                        ("SADDLE_PANE".into(), ticket.pane.to_string()),
                        ("SADDLE_REVISION".into(), ticket.revision.to_string()),
                    ],
                });
                ticket
            }
            Content::NewAgent {
                name,
                role,
                prompt,
                argv,
                ..
            } => {
                let mut args = vec![
                    "start".into(),
                    name,
                    "--cwd".into(),
                    cwd.clone(),
                    "--label".into(),
                    format!("role={role}"),
                ];
                if let Some(prompt) = prompt {
                    args.extend(["--prompt".into(), prompt]);
                }
                args.push("--".into());
                args.extend(argv);
                self.begin_start(anchor, place, args, None)?
            }
        };
        if focus {
            self.viewer.focus(ticket.pane);
            self.focus = Focus::Viewer;
        } else {
            self.viewer.restore_focus(&saved);
        }
        Ok((
            ticket,
            if agent_content { "agent" } else { cwd_source },
            moved,
        ))
    }
    pub(super) fn begin_start(
        &mut self,
        anchor: u64,
        place: Place,
        args: Vec<String>,
        focus_intent: Option<u64>,
    ) -> Result<Ticket> {
        ensure!(self.viewer.get(anchor).is_some(), "target pane disappeared");
        let cwd = args
            .windows(2)
            .find(|a| a[0] == "--cwd")
            .map(|a| a[1].clone());
        let ticket = self.viewer.reserve_at(anchor, place, args.get(1).cloned());
        let pane = self.viewer.get_mut(ticket.pane).unwrap();
        pane.starting = true;
        pane.pending_agent.cwd = cwd;
        self.actions
            .start(Action::Start(args, ticket, focus_intent));
        Ok(ticket)
    }
    pub(super) fn operation_update(
        &mut self,
        ticket: Ticket,
        state: &str,
        created: Option<&str>,
        error: Option<&str>,
    ) {
        if let Some(record) = self.records.iter_mut().find(|r| r.ticket == Some(ticket)) {
            record.value["state"] = json!(state);
            if let Some(name) = created {
                record.value["agent_created"] = json!({"name":name});
            }
            if state == "target_invalid" {
                record.value["pty"] = json!("target_invalid");
                record.value["ok"] = json!(false);
                record.value["error"] = json!({"code":"target_invalid","message":"display target closed or replaced; created agent keeps running"});
            }
            if let Some(error) = error {
                record.value["ok"] = json!(false);
                record.value["error"] = json!({"code":state,"message":error});
            }
        }
    }
    pub(super) fn close_snapshot(&self, target: CloseTarget) -> Result<Value> {
        let panes: Vec<_> = self.viewer.tabs.iter().flat_map(|tab| tab.panes.iter().filter(move |p| match target { CloseTarget::Pane(id) => p.id == id, CloseTarget::Tab(id) => tab.id == id, CloseTarget::All => true }))
            .map(|p| json!({"pane":p.id,"revision":p.ticket().revision,"running_shell":p.viewer.shell_live(),"cwd":p.viewer.metadata.cwd,"shell":p.viewer.shell.as_ref().map(|s| &s.program),"agent":p.viewer.target(),"corral_instance":p.viewer.metadata.instance})).collect();
        ensure!(!panes.is_empty(), "close target disappeared");
        Ok(json!(panes))
    }
    pub(super) fn close_target(&mut self, target: CloseTarget) -> Result<()> {
        match target {
            CloseTarget::Pane(id) => self.viewer.close_pane(id)?,
            CloseTarget::Tab(id) => self.viewer.close_tab(id)?,
            CloseTarget::All => {
                self.quit = true;
            }
        }
        Ok(())
    }
    pub(super) fn request_close(&mut self, target: CloseTarget) -> Result<()> {
        let snapshot = self.close_snapshot(target)?;
        if snapshot
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["running_shell"] == true)
        {
            self.closing = Some(Closing {
                target,
                snapshot,
                replacement: None,
                scroll: 0,
            });
        } else {
            self.close_target(target)?;
        }
        Ok(())
    }
    pub(super) fn request_replace(&mut self, pane: u64, replacement: Replacement) -> Result<()> {
        self.request_close(CloseTarget::Pane(pane))?;
        if let Some(closing) = &mut self.closing {
            closing.replacement = Some(replacement);
        }
        Ok(())
    }
    pub(super) fn confirm_close(&mut self) -> Result<()> {
        if let Some(closing) = self.closing.take() {
            if self.close_snapshot(closing.target).ok().as_ref() != Some(&closing.snapshot) {
                self.panel.message = "Close target changed; review and close again.".into();
                if let Some(Replacement::TaskLink(request)) = closing.replacement
                    && self.task_link_current(&request)
                {
                    self.task_link_error("Close target changed; retry the link.".into());
                }
                return Ok(());
            }
            if let Some(replacement) = closing.replacement {
                let CloseTarget::Pane(id) = closing.target else {
                    unreachable!()
                };
                let pane = self.viewer.get_mut(id).unwrap();
                pane.viewer.close()?;
                pane.viewer.shell = None;
                pane.viewer.metadata.instance = None;
                match replacement {
                    Replacement::Attach(name) => self.attach_at(id, name),
                    Replacement::TaskLink(request) => {
                        self.actions.start(Action::TaskAgent(request))
                    }
                    Replacement::Start(args) => {
                        let ticket =
                            self.begin_start(id, Place::Current, args, Some(self.input_revision))?;
                        if let Some(form) = &mut self.new_agent {
                            form.busy = Some(ticket);
                            form.error.clear();
                        }
                    }
                }
            } else {
                self.close_target(closing.target)?;
            }
        }
        Ok(())
    }
}

impl App {
    pub(super) fn draw_closing(&mut self, frame: &mut ratatui::Frame) {
        let Some(closing) = &self.closing else {
            return;
        };
        let t = &self.config.colors;
        let area = crate::theme::centered(frame.area(), 72, 18);
        frame.render_widget(ratatui::widgets::Clear, area);
        frame.render_widget(
            t.block(" Close terminals ", true)
                .style(t.base().bg(t.overlay)),
            area,
        );
        let (body, hits) = crate::buttons::draw_compact(
            t,
            frame,
            ui::inner(area),
            &[
                crate::buttons::Button::new("Cancel Esc", KeyCode::Esc, true),
                crate::buttons::Button::new("End shells y", KeyCode::Char('y'), true).danger(),
            ],
        );
        let mut text = String::from(
            "End these running terminals and their foreground tasks?\nScroll: ↑/↓ or mouse wheel\n\n",
        );
        for pane in closing
            .snapshot
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["running_shell"] == true)
        {
            text.push_str(&format!(
                "Pane {} · {}\n  {}\n",
                pane["pane"],
                pane["shell"].as_str().unwrap_or("shell"),
                pane["cwd"].as_str().unwrap_or("")
            ));
        }
        text.push_str("\nAgent displays will only detach. Corral agents keep running.");
        frame.render_widget(
            ratatui::widgets::Paragraph::new(text)
                .wrap(Default::default())
                .scroll((closing.scroll, 0)),
            body,
        );
        self.hits.buttons = hits;
        self.hits.agents.clear();
        self.hits.terminal.clear();
        self.hits.queue_rows.clear();
        self.queue.buttons.clear();
        self.queue.fields.clear();
        self.queue.project_rows.clear();
    }
}
