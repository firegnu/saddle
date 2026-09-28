use super::*;
use crate::links::AgentRequest;

#[derive(Clone)]
pub(super) struct LinkAttach {
    request: AgentRequest,
    ticket: Ticket,
    confirming: bool,
    deadline: Instant,
}
impl App {
    pub(super) fn task_link_current(&self, request: &AgentRequest) -> bool {
        self.focus == Focus::Queue
            && self.queue.view == queue::View::Links
            && matches!(self.queue.page, queue::Page::List)
            && self.queue.read_error.is_none()
            && self
                .queue
                .links_key()
                .is_some_and(|key| key.same_task(&request.key))
            && self.queue.links.checking_agent.as_ref() == Some(request)
    }
    pub(super) fn task_link_error(&mut self, message: String) {
        self.queue.links.checking_agent = None;
        self.queue.links.agent_request = None;
        self.queue.links.message = message;
    }
    pub(super) fn task_agent_result(
        &mut self,
        request: AgentRequest,
        result: Result<serde_json::Value>,
    ) {
        if !self.task_link_current(&request) {
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
            crate::links::check_agent(&status, &request, local)?;
            if local {
                self.viewer.activate_existing(&request.name)?;
                self.focus = Focus::Viewer;
                self.queue.links.checking_agent = None;
                self.queue.links.message.clear();
                return Ok(());
            }
            let id = existing.unwrap_or(self.viewer.active_pane().id);
            if self.viewer.get(id).unwrap().viewer.shell_live() {
                self.request_replace(id, Replacement::TaskLink(request.clone()))?;
                return Ok(());
            }
            let ticket = self
                .viewer
                .reserve_at(id, Place::Current, Some(request.name.clone()));
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
            self.queue.links.message = format!("Attaching {}…", request.name);
            Ok(())
        });
        if let Err(error) = result {
            self.task_link_error(format!("{error:#}"));
        }
    }
    pub(super) fn task_agent_ready(
        &mut self,
        request: AgentRequest,
        ticket: Ticket,
        result: Result<serde_json::Value>,
    ) {
        if !self.task_link_current(&request)
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
            crate::links::check_agent(&status, &request, attached)?;
            Ok(attached)
        });
        match checked {
            Ok(true) if self.viewer.get(ticket.pane).unwrap().viewer.state() == "running" => {
                self.viewer.focus(ticket.pane);
                self.focus = Focus::Viewer;
                self.link_attach = None;
                self.queue.links.checking_agent = None;
                self.queue.links.message.clear();
            }
            Ok(_) => self.link_attach.as_mut().unwrap().confirming = false,
            Err(error) => {
                let _ = self.viewer.get_mut(ticket.pane).unwrap().viewer.close();
                self.task_link_error(format!("{error:#}"));
                self.link_attach = None;
            }
        }
    }
    pub(super) fn task_links_tick(&mut self) -> Result<()> {
        if self.focus == Focus::Queue {
            self.queue.tick_links();
        }
        if let Some(LinkAttach {
            request,
            ticket,
            confirming,
            deadline,
        }) = self.link_attach.clone()
        {
            if !self.task_link_current(&request) || !self.viewer.valid(ticket) {
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
                        self.task_link_error("Agent attachment timed out".into());
                        self.viewer.get_mut(ticket.pane).unwrap().viewer.close()?;
                        self.link_attach = None;
                    } else if !confirming {
                        self.link_attach.as_mut().unwrap().confirming = true;
                        self.actions.start(Action::TaskAgentReady(request, ticket));
                    }
                } else if !pane.replacing() {
                    let note = pane.viewer.note.clone();
                    self.task_link_error(format!("Agent unavailable: {note}"));
                    self.link_attach = None;
                }
            }
        }
        if let Some(request) = self.queue.links.checking_agent.clone()
            && !self.task_link_current(&request)
        {
            self.task_link_error(String::new());
        }
        if let Some(request) = self.queue.links.agent_request.take()
            && self.task_link_current(&request)
        {
            self.actions.start(Action::TaskAgent(request));
        }
        Ok(())
    }
}
