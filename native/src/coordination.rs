use crate::{
    config::{parse_invite, Config},
    model::{Session, Snapshot},
};
use anyhow::{bail, Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
#[derive(Clone)]
pub struct Coordinator {
    http: reqwest::Client,
    endpoint: String,
}
impl Coordinator {
    pub fn new(origin: &str) -> Result<Self> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            endpoint: format!("{}/api/native-room", origin.trim().trim_end_matches('/')),
        })
    }
    async fn request<T: DeserializeOwned>(&self, body: &impl Serialize) -> Result<T> {
        let response = self
            .http
            .post(&self.endpoint)
            .json(body)
            .send()
            .await
            .context("Could not reach Vercel. Check your server address and network")?;
        let status = response.status();
        let length = response.content_length().unwrap_or(0);
        if length > 64_000 {
            bail!("Coordination response exceeds the size limit");
        }
        let data: Value = response
            .json()
            .await
            .context("The server did not return native room data. Check its deployment")?;
        if !status.is_success() {
            bail!(
                "{}",
                data.get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("Room coordination failed")
            );
        }
        serde_json::from_value(data).context("The coordination protocol is incompatible")
    }
    pub async fn enter(
        &self,
        config: &Config,
        invitation: Option<&str>,
        certificate: &str,
        endpoints: &[String],
    ) -> Result<Session> {
        let mut data = json!({ "op": "create", "name": config.name.trim(), "roomName": config.room_name.trim(), "certificate": certificate, "endpoints": endpoints });
        if let Some(invite) = invitation {
            let (room, secret) = parse_invite(invite)?;
            data["op"] = json!("join");
            data["room"] = json!(room);
            data["invite"] = json!(secret);
        }
        self.request(&data).await
    }
    pub async fn sync(
        &self,
        session: &Session,
        endpoints: &[String],
        muted: bool,
        deafened: bool,
    ) -> Result<Snapshot> {
        let snapshot: Snapshot = self.request(&json!({"op":"sync", "room":session.room, "peer":session.peer, "token":session.token, "endpoints": endpoints, "muted": muted, "deafened":deafened})).await?;
        if snapshot.version != 1 {
            bail!("This server uses a different native protocol version");
        }
        Ok(snapshot)
    }
    pub async fn leave(&self, session: &Session) -> Result<()> {
        let _: Value = self.request(&json!({"op":"leave", "room":session.room,"peer":session.peer,"token":session.token})).await?;
        Ok(())
    }
}
