use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex,
    },
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
    pub id: String,
    pub name: String,
    pub certificate: String,
    pub endpoints: Vec<String>,
    pub muted: bool,
    pub deafened: bool,
    pub seen: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub room: String,
    pub peer: String,
    pub token: String,
    pub invite: String,
    pub room_name: String,
    #[serde(default = "crate::config::default_room_capacity")]
    pub max_members: usize,
}
impl Session {
    pub fn invitation(&self) -> String {
        format!("hush1.{}.{}", self.room, self.invite)
    }
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub version: u32,
    pub room_name: String,
    #[serde(default = "crate::config::default_room_capacity")]
    pub max_members: usize,
    pub peers: Vec<Peer>,
}
#[derive(Clone, Default, Debug)]
pub struct LinkStats {
    pub state: String,
    pub rtt_ms: f64,
    pub sent: u64,
    pub received: u64,
    pub lost: u64,
    pub address: String,
    pub last_error: String,
}
#[derive(Clone)]
pub struct Chat {
    pub name: String,
    pub text: String,
}
#[derive(Default, Clone)]
pub struct ViewState {
    pub session: Option<Session>,
    pub peers: Vec<Peer>,
    pub links: BTreeMap<String, LinkStats>,
    pub chat: Vec<Chat>,
    pub busy: bool,
    pub testing: bool,
    pub error: String,
    pub notice: String,
    pub network: String,
    pub local_address: String,
    pub published_addresses: Vec<String>,
    pub audio: String,
    pub coordination_online: bool,
    pub joined: Option<std::time::Instant>,
}
pub type SharedState = Arc<Mutex<ViewState>>;
pub struct Controls {
    pub connections: AtomicU32,
    pub muted: AtomicBool,
    pub deafened: AtomicBool,
    pub ptt: AtomicBool,
    pub pressed: AtomicBool,
    pub pointer_pressed: AtomicBool,
    pub level: AtomicU32,
    pub gain: AtomicU32,
    pub volume: AtomicU32,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            connections: AtomicU32::new(0),
            muted: AtomicBool::new(false),
            deafened: AtomicBool::new(false),
            ptt: AtomicBool::new(false),
            pressed: AtomicBool::new(false),
            pointer_pressed: AtomicBool::new(false),
            level: AtomicU32::new(0),
            gain: AtomicU32::new(1f32.to_bits()),
            volume: AtomicU32::new(1f32.to_bits()),
        }
    }
}
impl Controls {
    pub fn transmitting(&self) -> bool {
        !self.muted.load(Ordering::Relaxed)
            && !self.deafened.load(Ordering::Relaxed)
            && (!self.ptt.load(Ordering::Relaxed)
                || self.pressed.load(Ordering::Relaxed)
                || self.pointer_pressed.load(Ordering::Relaxed))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ControlMessage {
    Chat { text: String },
    State { muted: bool, deafened: bool },
}
