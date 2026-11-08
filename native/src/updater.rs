//! GitHub is the update trust origin. No room service or GitHub token is involved.
use anyhow::{bail, Context, Result};
use eframe::egui;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const REPOSITORY: &str = "https://github.com/basilaslam/hush-native";
const API: &str = "https://api.github.com/repos/basilaslam/hush-native/releases?per_page=30";
const MAX_INSTALLER: u64 = 350 * 1024 * 1024;
#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}
#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}
#[derive(Clone)]
pub struct Release {
    pub version: Version,
    pub page: String,
    installer_url: String,
    checksums_url: String,
    size: u64,
    digest: String,
}
#[derive(Clone, Default)]
pub struct State {
    pub busy: bool,
    pub error: bool,
    pub message: String,
    pub progress: f32,
    pub release: Option<Release>,
    pub installer: Option<PathBuf>,
}
#[derive(Default)]
pub struct Updater {
    state: Arc<Mutex<State>>,
}
fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(concat!("Hush/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(600))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let url = attempt.url();
            if attempt.previous().len() < 5
                && url.scheme() == "https"
                && url.username().is_empty()
                && url.password().is_none()
                && matches!(
                    url.host_str(),
                    Some(
                        "github.com"
                            | "api.github.com"
                            | "release-assets.githubusercontent.com"
                            | "objects.githubusercontent.com"
                    )
                )
            {
                attempt.follow()
            } else {
                attempt.error("Update redirect left GitHub")
            }
        }))
        .build()?)
}
async fn limited(mut response: reqwest::Response, limit: usize) -> Result<Vec<u8>> {
    response.error_for_status_ref()?;
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
        bail!("Update metadata exceeds its size limit");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > limit {
            bail!("Update metadata exceeds its size limit");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
fn select_release(bytes: &[u8], installed: &Version, previews: bool) -> Result<Option<Release>> {
    let releases: Vec<GithubRelease> = serde_json::from_slice(bytes)?;
    let newest = releases
        .into_iter()
        .filter_map(|release| {
            let version = Version::parse(release.tag_name.strip_prefix('v')?).ok()?;
            (!release.draft
                && (previews || (!release.prerelease && version.pre.is_empty()))
                && version > *installed)
                .then_some((version, release))
        })
        .max_by(|a, b| a.0.cmp(&b.0));
    let Some((version, release)) = newest else {
        return Ok(None);
    };
    let filename = format!("Hush-{version}-Windows-Setup.exe");
    let installer = release
        .assets
        .iter()
        .find(|asset| asset.name == filename)
        .context("The newest release has no Windows installer yet. Check again shortly")?;
    let sums = release
        .assets
        .iter()
        .find(|asset| asset.name == "SHA256SUMS.txt")
        .context("The newest release has no checksums yet. Check again shortly")?;
    let prefix = format!("{REPOSITORY}/releases/download/{}/", release.tag_name);
    if installer.browser_download_url != format!("{prefix}{filename}")
        || sums.browser_download_url != format!("{prefix}SHA256SUMS.txt")
        || installer.size == 0
        || installer.size > MAX_INSTALLER
        || sums.size > 64_000
    {
        bail!("The release has an unexpected asset address or size");
    }
    let digest = installer
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .context("GitHub did not provide a SHA-256 digest for this installer")?
        .to_ascii_lowercase();
    Ok(Some(Release {
        version,
        page: format!("{REPOSITORY}/releases/tag/{}", release.tag_name),
        installer_url: installer.browser_download_url.clone(),
        checksums_url: sums.browser_download_url.clone(),
        size: installer.size,
        digest,
    }))
}
async fn available(previews: bool) -> Result<Option<Release>> {
    let response = client()?
        .get(API)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2026-03-10")
        .timeout(Duration::from_secs(25))
        .send()
        .await?;
    let bytes = limited(response, 1024 * 1024).await?;
    select_release(
        &bytes,
        &Version::parse(env!("CARGO_PKG_VERSION"))?,
        previews,
    )
}
fn file_digest(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut bytes = [0u8; 64 * 1024];
    loop {
        let size = file.read(&mut bytes)?;
        if size == 0 {
            break;
        }
        digest.update(&bytes[..size]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
fn checksum_matches(bytes: &[u8], filename: &str, digest: &str) -> Result<()> {
    let text = std::str::from_utf8(bytes)?;
    let matches: Vec<_> = text
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let name = parts.next()?.trim_start_matches('*');
            (name == filename && parts.next().is_none()).then_some(hash)
        })
        .collect();
    if matches.len() != 1 || !matches[0].eq_ignore_ascii_case(digest) {
        bail!("The release checksum does not match GitHub's installer digest");
    }
    Ok(())
}
fn cache() -> Result<PathBuf> {
    Ok(directories::ProjectDirs::from("app", "Hush", "Hush")
        .context("Could not locate the update cache")?
        .cache_dir()
        .join("updates"))
}
pub fn previous_install_error() -> Option<String> {
    for folder in fs::read_dir(cache().ok()?).ok()?.flatten() {
        if !folder.file_type().ok()?.is_dir()
            || Version::parse(&folder.file_name().to_string_lossy()).is_err()
        {
            continue;
        }
        for entry in fs::read_dir(folder.path()).ok()?.flatten() {
            if !entry
                .file_name()
                .to_string_lossy()
                .ends_with("Windows-Setup.exe.log")
            {
                continue;
            }
            let mut detail = String::new();
            let _ = File::open(entry.path())
                .ok()?
                .take(4096)
                .read_to_string(&mut detail);
            let _ = fs::remove_file(entry.path());
            return Some(format!("The previous Windows update did not finish. Open App updates in Settings and retry. {}", detail.chars().take(500).collect::<String>()));
        }
    }
    None
}
async fn download(
    release: &Release,
    state: &Arc<Mutex<State>>,
    repaint: &egui::Context,
) -> Result<PathBuf> {
    let http = client()?;
    let filename = format!("Hush-{}-Windows-Setup.exe", release.version);
    let sums = limited(
        http.get(&release.checksums_url)
            .timeout(Duration::from_secs(25))
            .send()
            .await?,
        64_000,
    )
    .await?;
    checksum_matches(&sums, &filename, &release.digest)?;
    let folder = cache()?.join(release.version.to_string());
    fs::create_dir_all(&folder)?;
    for entry in fs::read_dir(cache()?)?.flatten() {
        if entry.file_type().is_ok_and(|kind| kind.is_dir())
            && entry.path() != folder
            && Version::parse(&entry.file_name().to_string_lossy()).is_ok()
        {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
    let target = folder.join(filename);
    if target.is_file()
        && target.metadata()?.len() == release.size
        && file_digest(&target)? == release.digest
    {
        return Ok(target);
    }
    let partial = folder.join("installer.part");
    let result = async {
        let mut response = http
            .get(&release.installer_url)
            .send()
            .await?
            .error_for_status()?;
        if response
            .content_length()
            .is_some_and(|size| size != release.size)
        {
            bail!("Installer size differs from GitHub metadata");
        }
        let mut file = File::create(&partial)?;
        let mut size = 0u64;
        let mut digest = Sha256::new();
        let mut last = Instant::now();
        while let Some(chunk) = response.chunk().await? {
            size += chunk.len() as u64;
            if size > release.size || size > MAX_INSTALLER {
                bail!("Installer download exceeds its size limit");
            }
            file.write_all(&chunk)?;
            digest.update(&chunk);
            if last.elapsed() >= Duration::from_millis(250) {
                state.lock().unwrap().progress = size as f32 / release.size as f32;
                repaint.request_repaint();
                last = Instant::now();
            }
        }
        file.sync_all()?;
        drop(file);
        if size != release.size || format!("{:x}", digest.finalize()) != release.digest {
            bail!("Installer verification failed. Nothing was installed");
        }
        if target.exists() {
            fs::remove_file(&target)?;
        }
        fs::rename(&partial, &target)?;
        Ok(target)
    }
    .await;
    if result.is_err() {
        let _ = fs::remove_file(partial);
    }
    result
}
impl Updater {
    pub fn snapshot(&self) -> State {
        self.state.lock().unwrap().clone()
    }
    fn job(
        &self,
        repaint: egui::Context,
        job: impl FnOnce(&Arc<Mutex<State>>, &egui::Context) -> Result<()> + Send + 'static,
    ) {
        let state = self.state.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("hush-updates".into())
            .spawn(move || {
                let result = job(&state, &repaint);
                let mut view = state.lock().unwrap();
                view.busy = false;
                if let Err(error) = result {
                    view.error = true;
                    view.message =
                        format!("Update failed: {error:#}. Retry or open GitHub Releases.");
                }
                drop(view);
                repaint.request_repaint();
            })
        {
            let mut view = self.state.lock().unwrap();
            view.busy = false;
            view.error = true;
            view.message = error.to_string();
        }
    }
    pub fn check(&self, repaint: egui::Context, previews: bool) {
        {
            let mut view = self.state.lock().unwrap();
            if view.busy {
                return;
            }
            *view = State {
                busy: true,
                message: "Checking GitHub Releases…".into(),
                ..Default::default()
            };
        }
        self.job(repaint, move |state, _| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            let release = runtime.block_on(available(previews))?;
            let mut view = state.lock().unwrap();
            view.message = release.as_ref().map_or_else(
                || "You're up to date on this update channel.".into(),
                |r| format!("Hush {} is available.", r.version),
            );
            view.release = release;
            Ok(())
        });
    }
    pub fn download(&self, repaint: egui::Context) {
        let release = {
            let mut view = self.state.lock().unwrap();
            if view.busy {
                return;
            }
            let Some(release) = view.release.clone() else {
                return;
            };
            view.busy = true;
            view.error = false;
            view.progress = 0.;
            view.installer = None;
            view.message = "Downloading and verifying the Windows update…".into();
            release
        };
        self.job(repaint, move |state, repaint| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            let path = runtime.block_on(download(&release, state, repaint))?;
            let mut view = state.lock().unwrap();
            view.installer = Some(path);
            view.progress = 1.;
            view.message = "Verified. Ready to install and restart.".into();
            Ok(())
        });
    }
    // The Windows handoff intentionally outlives this app, which closes immediately.
    #[allow(clippy::zombie_processes)]
    pub fn install(&self) -> Result<()> {
        #[cfg(not(target_os = "windows"))]
        bail!("Automatic installation currently supports Windows. Use the source release on this platform");
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            let state = self.snapshot();
            if state.busy {
                bail!("Wait for the update download to finish");
            }
            let installer = state
                .installer
                .context("Download and verify an update first")?;
            let release = state.release.context("Check for updates first")?;
            if file_digest(&installer)? != release.digest {
                bail!("Installer changed after download. Download it again");
            }
            let executable = std::env::current_exe()?;
            let directory = executable.parent().context("Could not locate Hush")?;
            let probe = directory.join(format!(".hush-update-{}", std::process::id()));
            drop(
                fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&probe)
                    .context(
                        "This folder is not writable. Download the installer from GitHub Releases",
                    )?,
            );
            fs::remove_file(probe)?;
            let script = installer.with_extension("ps1");
            let content =
                install_script(&installer, &executable, &release.digest, std::process::id());
            fs::write(&script, format!("\u{feff}{content}"))?;
            let windows = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
            std::process::Command::new(
                PathBuf::from(windows).join("System32/WindowsPowerShell/v1.0/powershell.exe"),
            )
            .env_remove("PSModulePath")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(script)
            .creation_flags(0x08000000)
            .spawn()
            .context("Could not start the Windows updater")?;
            Ok(())
        }
    }
}
#[cfg(any(test, target_os = "windows"))]
fn install_script(installer: &Path, executable: &Path, digest: &str, pid: u32) -> String {
    let quote = |path: &Path| format!("'{}'", path.to_string_lossy().replace('\'', "''"));
    format!(
        r#"$ErrorActionPreference = 'Stop'
$installer = {installer}
$exe = {exe}
$log = $installer + '.log'
try {{
  $deadline = (Get-Date).AddSeconds(90)
  while (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{
    if ((Get-Date) -gt $deadline) {{ throw 'Hush did not close. Update cancelled.' }}
    Start-Sleep -Milliseconds 300
  }}
  $stream = [System.IO.File]::OpenRead($installer)
  $sha = [System.Security.Cryptography.SHA256]::Create()
  try {{ $hash = [System.BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '').ToLowerInvariant() }}
  finally {{ $stream.Dispose(); $sha.Dispose() }}
  if ($hash -ne '{digest}') {{ throw 'Installer checksum changed. Update cancelled.' }}
  $destination = Split-Path -Parent $exe
  $result = Start-Process -FilePath $installer -ArgumentList @('/S', ('/D=' + $destination)) -Wait -PassThru
  if ($result.ExitCode -ne 0) {{ throw ('Installer failed: ' + $result.ExitCode) }}
  Remove-Item -LiteralPath $installer -ErrorAction SilentlyContinue
  Start-Process -FilePath $exe
}} catch {{
  $_ | Out-File -LiteralPath $log -Encoding utf8
  if (-not (Get-Process -Id {pid} -ErrorAction SilentlyContinue)) {{ Start-Process -FilePath $exe }}
}}
"#,
        installer = quote(installer),
        exe = quote(executable)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    fn release(version: &str, preview: bool) -> Value {
        let filename = format!("Hush-{version}-Windows-Setup.exe");
        let base = format!("{REPOSITORY}/releases/download/v{version}/");
        json!({"tag_name":format!("v{version}"),"draft":false,"prerelease":preview,"assets":[
            {"name":filename,"browser_download_url":format!("{base}{filename}"),"size":100,"digest":format!("sha256:{}","a".repeat(64))},
            {"name":"SHA256SUMS.txt","browser_download_url":format!("{base}SHA256SUMS.txt"),"size":100,"digest":null}]})
    }
    #[test]
    fn updates_use_versions_channels_and_never_downgrade() {
        let installed = Version::parse("0.4.0").unwrap();
        let data = serde_json::to_vec(&vec![
            release("0.5.0", true),
            release("0.10.0", true),
            release("0.4.1", false),
        ])
        .unwrap();
        assert_eq!(
            select_release(&data, &installed, true)
                .unwrap()
                .unwrap()
                .version
                .to_string(),
            "0.10.0"
        );
        assert_eq!(
            select_release(&data, &installed, false)
                .unwrap()
                .unwrap()
                .version
                .to_string(),
            "0.4.1"
        );
        assert!(select_release(
            &serde_json::to_vec(&vec![release("0.3.0", true), release("0.4.0", true)]).unwrap(),
            &installed,
            true
        )
        .unwrap()
        .is_none());
        let mut draft = release("9.0.0", false);
        draft["draft"] = json!(true);
        assert!(
            select_release(&serde_json::to_vec(&vec![draft]).unwrap(), &installed, true)
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn updates_reject_untrusted_urls_missing_digests_and_oversized_installers() {
        for mutation in 0..3 {
            let mut item = release("1.0.0", false);
            match mutation {
                0 => {
                    item["assets"][0]["browser_download_url"] =
                        json!("https://evil.example/installer.exe")
                }
                1 => item["assets"][0]["digest"] = Value::Null,
                _ => item["assets"][0]["size"] = json!(MAX_INSTALLER + 1),
            }
            assert!(select_release(
                &serde_json::to_vec(&vec![item]).unwrap(),
                &Version::new(0, 4, 0),
                true
            )
            .is_err());
        }
        let digest = "a".repeat(64);
        assert!(checksum_matches(
            format!("{digest}  Hush.exe\r\n").as_bytes(),
            "Hush.exe",
            &digest
        )
        .is_ok());
        assert!(checksum_matches(
            format!("{}  Hush.exe\n", "b".repeat(64)).as_bytes(),
            "Hush.exe",
            &digest
        )
        .is_err());
        assert!(checksum_matches(
            format!("{digest}  ../Hush.exe\n").as_bytes(),
            "Hush.exe",
            &digest
        )
        .is_err());
    }
    #[test]
    fn installer_handoff_waits_verifies_and_quotes_local_paths() {
        let script = install_script(
            Path::new("C:/Users/Player's PC/update.exe"),
            Path::new("C:/Games/Hush/hush-native.exe"),
            &"a".repeat(64),
            1234,
        );
        assert!(script.contains("Player''s PC"));
        assert!(
            script.find("while (Get-Process -Id 1234").unwrap()
                < script.find("ComputeHash").unwrap()
        );
        assert!(
            script.find("ComputeHash").unwrap() < script.find("$result = Start-Process").unwrap()
        );
        assert!(script.contains("('/D=' + $destination)"));
    }
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_handoff_executes_after_exit_and_restarts_in_a_path_with_spaces() {
        use std::process::Command;
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path().join("Player's test folder");
        fs::create_dir_all(&folder).unwrap();
        let source = folder.join("stub.rs");
        fs::write(&source, r#"fn main() {
            let args = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
            if args == "--wait" { std::thread::sleep(std::time::Duration::from_secs(120)); }
            else if let Some((_, directory)) = args.split_once("/D=") { std::fs::write(std::path::Path::new(directory).join("installed"), "ok").unwrap(); }
            else { std::fs::write(std::env::current_exe().unwrap().with_extension("started"), "ok").unwrap(); }
        }"#).unwrap();
        let exe = folder.join("Hush.exe");
        assert!(Command::new("rustc")
            .arg(source)
            .arg("-o")
            .arg(&exe)
            .status()
            .unwrap()
            .success());
        let installer = folder.join("Setup.exe");
        fs::copy(&exe, &installer).unwrap();
        let digest = file_digest(&installer).unwrap();
        let mut parent = Command::new(&exe).arg("--wait").spawn().unwrap();
        let script = folder.join("update.ps1");
        fs::write(
            &script,
            format!(
                "\u{feff}{}",
                install_script(&installer, &exe, &digest, parent.id())
            ),
        )
        .unwrap();
        let mut handoff = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(script)
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_millis(1200));
        assert!(!folder.join("installed").exists());
        parent.kill().unwrap();
        parent.wait().unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        while !exe.with_extension("started").exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(100));
        }
        let status = handoff.try_wait().unwrap();
        if status.is_none() {
            let _ = handoff.kill();
        }
        let _ = handoff.wait();
        assert!(
            folder.join("installed").exists(),
            "{}",
            fs::read_to_string(installer.with_extension("exe.log")).unwrap_or_default()
        );
        assert!(exe.with_extension("started").exists());
        fs::remove_file(folder.join("installed")).unwrap();
        fs::remove_file(exe.with_extension("started")).unwrap();
        fs::copy(&exe, &installer).unwrap();
        fs::OpenOptions::new()
            .append(true)
            .open(&installer)
            .unwrap()
            .write_all(b"tampered")
            .unwrap();
        let script = folder.join("tamper.ps1");
        fs::write(
            &script,
            format!(
                "\u{feff}{}",
                install_script(&installer, &exe, &digest, parent.id())
            ),
        )
        .unwrap();
        let mut blocked = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(script)
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        while !exe.with_extension("started").exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(100));
        }
        if blocked.try_wait().unwrap().is_none() {
            let _ = blocked.kill();
        }
        let _ = blocked.wait();
        assert!(!folder.join("installed").exists());
        assert!(installer.with_extension("exe.log").exists());
        assert!(exe.with_extension("started").exists());
    }
    #[tokio::test]
    async fn chunked_metadata_is_bounded_without_content_length() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 1024];
            let _ = socket.read(&mut request).await;
            socket.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n5\r\nabcde\r\n0\r\n\r\n").await.unwrap();
        });
        let response = reqwest::get(format!("http://{address}")).await.unwrap();
        assert!(limited(response, 4).await.is_err());
        server.await.unwrap();
    }
}
