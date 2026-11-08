use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddrV4, path::PathBuf};
pub const MAX_ROOM_MEMBERS: usize = 32;
pub fn default_room_capacity() -> usize {
    6
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: String,
    pub name: String,
    pub room_name: String,
    pub max_members: usize,
    pub check_updates: bool,
    pub preview_updates: bool,
    pub input: String,
    pub output: String,
    pub bitrate: i32,
    pub input_gain: f32,
    pub output_volume: f32,
    pub push_to_talk: bool,
    pub ptt_shortcut: String,
    pub quiet: bool,
    pub port: u16,
    pub public_address: String,
    pub auto_map: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            server: option_env!("HUSH_SERVER_URL").unwrap_or("").to_owned(),
            name: String::new(),
            room_name: "The hangout".into(),
            max_members: default_room_capacity(),
            check_updates: true,
            preview_updates: true,
            input: String::new(),
            output: String::new(),
            bitrate: 32_000,
            input_gain: 1.,
            output_volume: 1.,
            push_to_talk: false,
            ptt_shortcut: "F8".into(),
            quiet: true,
            port: 41642,
            public_address: String::new(),
            auto_map: true,
        }
    }
}
impl Config {
    pub fn path() -> Option<PathBuf> {
        directories::ProjectDirs::from("app", "Hush", "Hush")
            .map(|dirs| dirs.config_dir().join("preferences.json"))
    }
    pub fn load() -> Self {
        Self::path()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default()
    }
    pub fn save(&self) -> Result<()> {
        let path = Self::path().context("Could not locate your preferences folder")?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        let temporary = path.with_extension("tmp");
        std::fs::write(&temporary, serde_json::to_vec_pretty(self)?)?;
        // Windows rename does not replace a destination. No credentials are persisted here.
        std::fs::copy(&temporary, &path)?;
        std::fs::remove_file(temporary)?;
        Ok(())
    }
    pub fn validate(&self) -> Result<()> {
        crate::shortcut::parse(&self.ptt_shortcut)?;
        if !(2..=MAX_ROOM_MEMBERS).contains(&self.max_members) {
            bail!("Choose a room size from 2 to {MAX_ROOM_MEMBERS}");
        }
        let url = url::Url::parse(self.server.trim())
            .context("Run terminal server setup or set its HTTPS address in Settings first")?;
        let local = url
            .host_str()
            .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "[::1]"));
        if url.scheme() != "https" && !(local && url.scheme() == "http") {
            bail!("The coordination address must use HTTPS (localhost may use HTTP)");
        }
        if url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            bail!("Use only the server origin, without a path, password, or query");
        }
        if self.name.trim().is_empty() || self.name.trim().chars().count() > 24 {
            bail!("Choose a display name of 1–24 characters");
        }
        if self.room_name.trim().is_empty() || self.room_name.trim().chars().count() > 48 {
            bail!("Choose a room name of 1–48 characters");
        }
        if ![24_000, 32_000, 64_000].contains(&self.bitrate) {
            bail!("Choose a supported voice quality");
        }
        if !self.public_address.trim().is_empty() {
            self.public_address
                .trim()
                .parse::<SocketAddrV4>()
                .context("Public UDP address must look like 203.0.113.1:41642")?;
        }
        Ok(())
    }
}
pub fn parse_invite(value: &str) -> Result<(String, String)> {
    let value = value.trim();
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3
        || parts[0] != "hush1"
        || parts[1].len() != 12
        || parts[2].len() != 32
        || parts[1..].iter().any(|part| {
            !part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        })
    {
        bail!("Paste a native Hush invitation beginning with hush1.");
    }
    Ok((parts[1].into(), parts[2].into()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn older_preferences_get_capacity_and_update_defaults() {
        let mut config: Config =
            serde_json::from_str(r#"{"name":"Player","server":"https://room.example"}"#).unwrap();
        assert_eq!(config.max_members, 6);
        assert!(config.check_updates && config.preview_updates);
        assert_eq!(config.ptt_shortcut, "F8");
        config.ptt_shortcut = "Ctrl+Shift+V".into();
        config.push_to_talk = true;
        let saved = serde_json::to_vec(&config).unwrap();
        let restored: Config = serde_json::from_slice(&saved).unwrap();
        assert!(restored.push_to_talk);
        assert_eq!(restored.ptt_shortcut, "Ctrl+Shift+V");
        for size in [2, 32] {
            config.max_members = size;
            assert!(config.validate().is_ok());
        }
        for size in [0, 1, 33, usize::MAX] {
            config.max_members = size;
            assert!(config.validate().is_err());
        }
    }
    #[test]
    fn native_invites_are_distinct_and_validated() {
        assert!(parse_invite("hush1.abcdefghijkl.01234567890123456789012345678901").is_ok());
        assert!(parse_invite("abcdefghijkl.01234567890123456789012345678901").is_err());
        assert!(parse_invite("hush1.abcdefghijkl.0123456789012345678901234567890/").is_err());
    }
    #[test]
    fn server_urls_cannot_redirect_credentials_or_send_over_cleartext_internet() {
        let mut c = Config {
            name: "Basil".into(),
            server: "http://example.com".into(),
            ..Default::default()
        };
        assert!(c.validate().is_err());
        c.server = "https://user:pass@example.com".into();
        assert!(c.validate().is_err());
        c.server = "https://hush.example.com".into();
        assert!(c.validate().is_ok());
        c.server = "http://127.0.0.1:5173".into();
        assert!(c.validate().is_ok());
    }
}
