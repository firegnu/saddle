use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::Value;
use std::{
    io::Read,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Agent {
    pub name: String,
    pub kind: Option<String>,
    pub instance: Option<String>,
    pub cwd: Option<String>,
    pub starting: bool,
    pub incompatible: bool,
    pub proto: Option<u64>,
    pub state: Option<String>,
    pub last_tool: Option<String>,
    pub turn_started: Option<f64>,
    pub last_output: Option<f64>,
    pub idle_for: Option<f64>,
    pub attached: usize,
    pub last_input_source: Option<String>,
    pub title: Option<String>,
    pub started: Option<f64>,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct Client {
    pub program: String,
}
impl Client {
    pub fn json(&self, args: &[&str], timeout: Duration, cancel: &AtomicBool) -> Result<Value> {
        let mut child = Command::new(&self.program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("starting {} {}", self.program, args.join(" ")))?;
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let (tx, rx) = mpsc::channel();
        for (is_stdout, mut pipe) in [
            (true, Box::new(stdout) as Box<dyn Read + Send>),
            (false, Box::new(stderr)),
        ] {
            let tx = tx.clone();
            thread::spawn(move || {
                let mut bytes = Vec::new();
                let result = pipe.read_to_end(&mut bytes).map(|_| bytes);
                let _ = tx.send((is_stdout, result));
            });
        }
        drop(tx);
        let deadline = Instant::now() + timeout;
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                bail!("{} cancelled or timed out", args.join(" "));
            }
            thread::sleep(Duration::from_millis(10));
        };
        let mut output = Vec::new();
        let mut errors = Vec::new();
        for _ in 0..2 {
            let (is_stdout, bytes) = rx
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .context("command output timed out")?;
            if is_stdout {
                output = bytes?;
            } else {
                errors = bytes?;
            }
        }
        let value: Value = serde_json::from_slice(&output).with_context(|| {
            format!(
                "{}: invalid JSON ({})",
                args.join(" "),
                String::from_utf8_lossy(&errors).trim()
            )
        })?;
        if !status.success() || value.get("ok") == Some(&Value::Bool(false)) {
            bail!(
                "{}: {}",
                args.join(" "),
                value
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("command failed")
            );
        }
        Ok(value)
    }

    pub fn collect(&self) -> Result<Vec<Agent>> {
        self.collect_until(&AtomicBool::new(false))
    }

    pub fn collect_until(&self, cancel: &AtomicBool) -> Result<Vec<Agent>> {
        #[derive(Deserialize)]
        struct Listing {
            agents: Vec<Agent>,
        }
        let listing: Listing =
            serde_json::from_value(self.json(&["ls"], Duration::from_secs(15), cancel)?)?;
        let mut agents = listing.agents;
        for agent in &mut agents {
            if cancel.load(Ordering::Relaxed) {
                bail!("cancelled");
            }
            if agent.starting || agent.incompatible {
                continue;
            }
            match self
                .json(&["status", &agent.name], Duration::from_secs(15), cancel)
                .and_then(|v| Ok(serde_json::from_value::<Agent>(v)?))
            {
                Ok(mut status) => {
                    status.name.clone_from(&agent.name);
                    status.cwd = agent.cwd.take();
                    if status.instance.is_none() {
                        status.instance = agent.instance.take();
                    }
                    *agent = status;
                }
                Err(error) => agent.error = Some(error.to_string()),
            }
        }
        agents.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(agents)
    }
}

pub struct Poller {
    pub updates: mpsc::Receiver<Result<Vec<Agent>>>,
    cancel: Arc<AtomicBool>,
    wake: mpsc::Sender<()>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Poller {
    pub fn start(client: Client, every: Duration) -> Self {
        let (tx, updates) = mpsc::channel();
        let (wake, wait) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let quitting = cancel.clone();
        let worker = thread::spawn(move || {
            while !quitting.load(Ordering::Relaxed) {
                if tx.send(client.collect_until(&quitting)).is_err() {
                    break;
                }
                let _ = wait.recv_timeout(every);
            }
        });
        Self {
            updates,
            cancel,
            wake,
            worker: Some(worker),
        }
    }
    pub fn refresh(&self) {
        let _ = self.wake.send(());
    }
}
impl Drop for Poller {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.refresh();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
