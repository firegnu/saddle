use super::*;
use crate::plugins::runtime::Navigation as AgentRequest;

#[derive(Clone)]
pub(super) struct LinkAttach {
    request: AgentRequest,
    ticket: Ticket,
    confirming: bool,
    deadline: Instant,
}
impl App {
    pub(super) fn navigation_current(&self, request: &AgentRequest) -> bool {
        self.navigation.as_ref() == Some(request)
            && self.navigation_revision == self.input_revision
            && self.plugins.navigation_current(request)
    }
    pub(super) fn navigation_error(&mut self, message: String) {
        if let Some(request) = self.navigation.take() {
            self.plugins.navigation_result(&request, "failed", &message);
        }
    }
    fn navigation_complete(&mut self, request: &AgentRequest) {
        self.plugins.navigation_result(request, "opened", "");
        self.navigation = None;
        self.plugin_overlay = None;
        self.focus = Focus::Viewer;
    }

    pub(super) fn navigation_result(
        &mut self,
        request: AgentRequest,
        result: Result<serde_json::Value>,
    ) {
        if !self.navigation_current(&request) {
            return;
        }
        let result = result.and_then(|status| {
            let existing = self.viewer.find(&request.name);
            let mut local = false;
            if let Some(id) = existing {
                let pane = self.viewer.get(id).unwrap();
                let metadata = if pane.viewer.showing.as_deref() == Some(&request.name) {
                    &pane.viewer.metadata
                } else if pane.requested() == Some(&request.name) {
                    &pane.pending_agent
                } else {
                    pane.viewer.target_metadata()
                };
                anyhow::ensure!(
                    metadata.instance.as_ref() == Some(&request.instance),
                    "open agent identity changed or unknown"
                );
                anyhow::ensure!(
                    !pane.replacing(),
                    "agent connection is already changing; retry when ready"
                );
                local = pane.input_session().is_some();
            }
            crate::plugins::navigation::check_agent(&status, &request, local)?;
            if local {
                self.viewer.activate_existing(&request.name)?;
                self.focus = Focus::Viewer;
                self.navigation_complete(&request);
                return Ok(());
            }
            let previous = self.viewer.active_pane().id;
            let id = existing.unwrap_or(previous);
            let ticket = self.viewer.reserve_at(
                id,
                if existing.is_some() {
                    Place::Current
                } else {
                    Place::Tab
                },
                Some(request.name.clone()),
            );
            // Keep the source view focused until the expected instance is attached and verified.
            self.viewer.focus(previous);
            let pane = self.viewer.get_mut(ticket.pane).unwrap();
            pane.viewer.shell = None;
            pane.pending_agent.instance = Some(request.instance.clone());
            pane.pending_agent.cwd = status["cwd"].as_str().map(str::to_owned).or_else(|| {
                self.panel
                    .agents
                    .iter()
                    .find(|a| a.name == request.name)
                    .and_then(|a| a.cwd.clone())
            });
            self.actions
                .start(Action::Attach(request.name.clone(), ticket, None));
            self.link_attach = Some(LinkAttach {
                request: request.clone(),
                ticket,
                confirming: false,
                deadline: Instant::now() + Duration::from_secs(15),
            });

            Ok(())
        });
        if let Err(error) = result {
            self.navigation_error(format!("{error:#}"));
        }
    }
    pub(super) fn navigation_ready(
        &mut self,
        request: AgentRequest,
        ticket: Ticket,
        result: Result<serde_json::Value>,
    ) {
        if !self.navigation_current(&request)
            || !self.viewer.valid(ticket)
            || !self
                .link_attach
                .as_ref()
                .is_some_and(|a| a.ticket == ticket && a.request == request)
        {
            return;
        }
        let checked = result.and_then(|status| {
            let attached = status["attached"].as_u64() == Some(1);
            crate::plugins::navigation::check_agent(&status, &request, attached)?;
            Ok(attached)
        });
        match checked {
            Ok(true) if self.viewer.get(ticket.pane).unwrap().viewer.state() == "running" => {
                self.viewer.focus(ticket.pane);
                self.focus = Focus::Viewer;
                self.link_attach = None;
                self.navigation_complete(&request);
            }
            Ok(_) => self.link_attach.as_mut().unwrap().confirming = false,
            Err(error) => {
                let _ = self.viewer.get_mut(ticket.pane).unwrap().viewer.close();
                self.navigation_error(format!("{error:#}"));
                self.link_attach = None;
            }
        }
    }
    pub(super) fn navigation_tick(&mut self) -> Result<()> {
        for request in self.plugins.take_navigation() {
            let source_open = self.plugin_palette.is_none()
                && self.plugin_page.is_none()
                && self.settings.is_none()
                && self.attention.is_none()
                && (self
                    .plugin_overlay
                    .as_ref()
                    .is_some_and(|o| o.id == request.plugin)
                    || (self.focus == Focus::Viewer
                        && self.viewer.active_pane().plugin_id() == Some(request.plugin.as_str())));
            if self.navigation.is_some()
                || !source_open
                || !self.plugins.navigation_current(&request)
            {
                self.plugins.navigation_result(
                    &request,
                    "cancelled",
                    "Source view changed or navigation already pending",
                );
                continue;
            }
            self.navigation_revision = self.input_revision;
            self.navigation = Some(request.clone());
            self.actions.start(Action::PluginAgent(request));
        }
        if let Some(LinkAttach {
            request,
            ticket,
            confirming,
            deadline,
        }) = self.link_attach.clone()
        {
            if !self.navigation_current(&request) || !self.viewer.valid(ticket) {
                // Invalidate only the connection opened by this request, never another pane.
                if self.viewer.valid(ticket) {
                    let cancelled = self.viewer.reserve_at(ticket.pane, Place::Current, None);
                    self.viewer.complete(cancelled, None)?;
                    let pane = self.viewer.get_mut(ticket.pane).unwrap();
                    pane.viewer.cancel_pending();
                    if pane.viewer.target() == Some(&request.name) {
                        pane.viewer.close()?;
                    }
                }
                self.link_attach = None;
            } else {
                let pane = self.viewer.get(ticket.pane).unwrap();
                if pane.viewer.state() == "running"
                    && pane.viewer.showing.as_deref() == Some(&request.name)
                    && pane.viewer.metadata.instance.as_ref() == Some(&request.instance)
                {
                    if Instant::now() >= deadline {
                        self.navigation_error("Agent attachment timed out".into());
                        self.viewer.get_mut(ticket.pane).unwrap().viewer.close()?;
                        self.link_attach = None;
                    } else if !confirming {
                        self.link_attach.as_mut().unwrap().confirming = true;
                        self.actions
                            .start(Action::PluginAgentReady(request, ticket));
                    }
                } else if !pane.replacing() {
                    let note = pane.viewer.note.clone();
                    self.navigation_error(format!("Agent unavailable: {note}"));
                    self.link_attach = None;
                }
            }
        }
        if self
            .navigation
            .as_ref()
            .is_some_and(|r| !self.navigation_current(r))
        {
            self.navigation_error("Source view changed".into());
        }
        Ok(())
    }
}
