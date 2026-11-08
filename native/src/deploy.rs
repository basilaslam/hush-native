//! Official Cloudflare login/deployment runs only while requested, off the audio/UI threads.
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{ChildStdin, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
#[derive(Clone, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
}
#[derive(Clone, Default)]
pub struct State {
    pub running: bool,
    pub message: String,
    pub accounts: Vec<Account>,
    pub selected: usize,
    pub server: String,
    pub error: bool,
    pending: Option<String>,
}
#[derive(Deserialize)]
#[serde(tag = "event", rename_all = "lowercase")]
enum Event {
    Status { message: String },
    Accounts { accounts: Vec<Account> },
    Ready { server: String },
    Error { message: String },
    Cancelled { message: String },
}
pub struct Deployment {
    state: Arc<Mutex<State>>,
    input: Arc<Mutex<Option<ChildStdin>>>,
    cancelled: Arc<AtomicBool>,
}
impl Default for Deployment {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(State::default())),
            input: Arc::new(Mutex::new(None)),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
}
fn runtime_directory() -> Result<PathBuf> {
    let parent = std::env::current_exe()?
        .parent()
        .context("Could not locate Hush")?
        .to_owned();
    let installed = parent.join("deployment");
    if installed.join("node.exe").is_file() && installed.join("helper.mjs").is_file() {
        return Ok(installed);
    }
    #[cfg(debug_assertions)]
    {
        let development = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../deployment");
        if development.join("node.exe").is_file() {
            return Ok(development);
        }
    }
    bail!("The Cloudflare deployment tools are missing. Reinstall Hush with the full Windows installer.")
}
pub fn validate_server(server: &str) -> Result<String> {
    let url = url::Url::parse(server).context("Cloudflare returned an invalid server address")?;
    if url.scheme() != "https"
        || !url
            .host_str()
            .is_some_and(|host| host.ends_with(".workers.dev"))
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        bail!("Cloudflare returned an unexpected server address");
    }
    Ok(url.origin().ascii_serialization())
}
impl Deployment {
    pub fn snapshot(&self) -> State {
        self.state.lock().unwrap().clone()
    }
    pub fn take_ready(&self) -> Option<String> {
        self.state.lock().unwrap().pending.take()
    }
    pub fn start(&self, repaint: eframe::egui::Context) {
        {
            let mut state = self.state.lock().unwrap();
            if state.running {
                return;
            }
            *state = State {
                running: true,
                message: "Preparing Cloudflare sign-in…".into(),
                ..Default::default()
            };
        }
        self.cancelled.store(false, Ordering::Release);
        let cancelled = self.cancelled.clone();
        let state = self.state.clone();
        let input = self.input.clone();
        let thread_state = state.clone();
        let result = std::thread::Builder::new()
            .name("hush-deployment".into())
            .spawn(move || {
                let result = run(&state, &input, &cancelled, &repaint);
                *input.lock().unwrap() = None;
                let mut view = state.lock().unwrap();
                if let Err(error) = result {
                    view.error = true;
                    view.message = format!("{error:#}");
                }
                view.running = false;
                view.accounts.clear();
                drop(view);
                repaint.request_repaint();
            });
        if let Err(error) = result {
            let mut view = thread_state.lock().unwrap();
            view.running = false;
            view.error = true;
            view.message = format!("Could not start deployment: {error}");
        }
    }
    pub fn select(&self, index: usize) {
        self.state.lock().unwrap().selected = index;
    }
    pub fn use_account(&self) {
        let mut state = self.state.lock().unwrap();
        let Some(account) = state.accounts.get(state.selected) else {
            return;
        };
        if let Some(input) = self.input.lock().unwrap().as_mut() {
            if writeln!(input, "{}", json!({"op":"account", "id":account.id})).is_ok() {
                let _ = input.flush();
                state.accounts.clear();
                state.message = "Preparing your server…".into();
            }
        }
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        if let Some(input) = self.input.lock().unwrap().as_mut() {
            let _ = writeln!(input, "{}", json!({"op":"cancel"}));
            let _ = input.flush();
        }
    }
}
impl Drop for Deployment {
    fn drop(&mut self) {
        self.cancel();
        self.input.lock().unwrap().take();
    }
}
fn run(
    state: &Arc<Mutex<State>>,
    input: &Arc<Mutex<Option<ChildStdin>>>,
    cancelled: &Arc<AtomicBool>,
    repaint: &eframe::egui::Context,
) -> Result<()> {
    let root = runtime_directory()?;
    let data = directories::ProjectDirs::from("app", "Hush", "Hush")
        .context("Could not locate Hush preferences")?
        .data_local_dir()
        .join("cloudflare");
    std::fs::create_dir_all(&data)?;
    let mut command = Command::new(root.join("node.exe"));
    command
        .arg(root.join("helper.mjs"))
        .arg("--state-dir")
        .arg(data)
        .current_dir(&root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .context("Could not start the bundled Cloudflare tools. Reinstall Hush and try again")?;
    *input.lock().unwrap() = child.stdin.take();
    if cancelled.load(Ordering::Acquire) {
        if let Some(writer) = input.lock().unwrap().as_mut() {
            let _ = writeln!(writer, "{}", json!({"op":"cancel"}));
            let _ = writer.flush();
        }
    }
    let stdout = child
        .stdout
        .take()
        .context("Could not read deployment progress")?;
    let mut terminal_event = false;
    for line in BufReader::new(stdout).lines() {
        let line = line?;
        if line.len() > 16384 {
            continue;
        }
        let Ok(event) = serde_json::from_str::<Event>(&line) else {
            continue;
        };
        let mut view = state.lock().unwrap();
        match event {
            Event::Status { message } => view.message = message,
            Event::Accounts { accounts } => {
                view.accounts = accounts;
                view.selected = 0;
                view.message = "Choose the Cloudflare account for your server.".into();
            }
            Event::Ready { server } => {
                let server = validate_server(&server)?;
                view.server = server.clone();
                view.pending = Some(server);
                view.message = "Server ready. Its address has been set up for Hush.".into();
                view.error = false;
                terminal_event = true;
            }
            Event::Error { message } => {
                view.message = message;
                view.error = true;
                terminal_event = true;
            }
            Event::Cancelled { message } => {
                view.message = message;
                view.error = false;
                terminal_event = true;
            }
        }
        drop(view);
        repaint.request_repaint();
    }
    let status = child.wait()?;
    if !terminal_event {
        bail!(
            "Cloudflare setup stopped unexpectedly ({}). Try Deploy server again.",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deployment_accepts_only_clean_cloudflare_https_origins() {
        assert_eq!(
            validate_server("https://hush-squad-abcdef123456.example.workers.dev").unwrap(),
            "https://hush-squad-abcdef123456.example.workers.dev"
        );
        for bad in [
            "http://example.workers.dev",
            "https://workers.dev.evil.test",
            "https://example.workers.dev/path",
            "https://user:pass@example.workers.dev",
            "https://example.workers.dev?token=x",
            "https://example.workers.dev:8443",
        ] {
            assert!(validate_server(bad).is_err(), "{bad}");
        }
    }
}
