//! An owned worker, stopped with the plugin. No launchd service and no retries.
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};
pub struct Worker {
    send: mpsc::SyncSender<String>,
    cancel: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Worker {
    pub fn start() -> Self {
        let (send, receive) = mpsc::sync_channel::<String>(8);
        let cancel = Arc::new(AtomicBool::new(false));
        let stop = cancel.clone();
        let thread = thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let text = match receive.recv_timeout(Duration::from_millis(50)) {
                    Ok(t) => t,
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(_) => break,
                };
                let _ = crate::command::run(
                    "osascript",
                    &[
                        "-e",
                        "on run argv\n display notification (item 1 of argv) with title \"Drover\"\nend run",
                        "--",
                        &text,
                    ],
                    None,
                    Duration::from_secs(10),
                    &stop,
                );
            }
        });
        Self {
            send,
            cancel,
            thread: Some(thread),
        }
    }
    pub fn send(&self, text: String) {
        let _ = self.send.try_send(text);
    }
}
impl Default for Worker {
    fn default() -> Self {
        Self::start()
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
