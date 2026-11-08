use crate::{
    audio::{AudioCommand, AudioHandle, Encoded},
    config::Config,
    coordination::Coordinator,
    model::{
        Chat, ControlMessage, Controls, LinkStats, Peer, Session, SharedState, Snapshot, ViewState,
    },
    transport::{self, Identity, Mapping, SequenceTracker},
};
use anyhow::{bail, Result};
use bytes::Bytes;
use quinn::{Connection, Endpoint};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, RwLock,
    },
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

pub enum Command {
    Enter {
        config: Config,
        invitation: Option<String>,
    },
    Leave,
    Chat(String),
    ControlChanged,
    Test(Config),
    StopTest,
    Shutdown,
}
pub struct Controller {
    pub state: SharedState,
    pub controls: Arc<Controls>,
    tx: mpsc::Sender<Command>,
    cancel: Arc<AtomicBool>,
    _runtime: tokio::runtime::Runtime,
}
impl Controller {
    pub fn new(repaint: eframe::egui::Context) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("hush-network")
            .build()?;
        let state = Arc::new(Mutex::new(ViewState {
            coordination_online: true,
            ..Default::default()
        }));
        let controls = Arc::new(Controls::default());
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel(32);
        runtime.spawn(worker(
            rx,
            state.clone(),
            controls.clone(),
            cancel.clone(),
            repaint,
        ));
        Ok(Self {
            state,
            controls,
            tx,
            cancel,
            _runtime: runtime,
        })
    }
    pub fn send(&self, command: Command) {
        if matches!(
            command,
            Command::Leave | Command::Shutdown | Command::StopTest
        ) {
            self.cancel.store(true, Ordering::Release);
        }
        if self.tx.try_send(command).is_err() {
            self.state.lock().unwrap().error =
                "The voice engine is busy. Try again shortly.".into();
        }
    }
    pub fn ptt(&self, pressed: bool) {
        self.controls.pressed.store(pressed, Ordering::Relaxed);
        self.send(Command::ControlChanged);
    }
    pub fn configure(&self, config: &Config) {
        self.controls
            .ptt
            .store(config.push_to_talk, Ordering::Relaxed);
        self.controls
            .gain
            .store(config.input_gain.to_bits(), Ordering::Relaxed);
        self.controls
            .volume
            .store(config.output_volume.to_bits(), Ordering::Relaxed);
        self.send(Command::ControlChanged);
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        let _ = self.tx.try_send(Command::Shutdown);
    }
}
async fn worker(
    mut commands: mpsc::Receiver<Command>,
    state: SharedState,
    controls: Arc<Controls>,
    cancel: Arc<AtomicBool>,
    repaint: eframe::egui::Context,
) {
    while let Some(command) = commands.recv().await {
        match command {
            Command::Enter { config, invitation } => {
                cancel.store(false, Ordering::Release);
                {
                    let mut view = state.lock().unwrap();
                    view.busy = true;
                    view.error.clear();
                    view.notice.clear();
                    view.network = "Opening native audio and UDP…".into();
                }
                repaint.request_repaint();
                let result = run_session(
                    config,
                    invitation,
                    &mut commands,
                    state.clone(),
                    controls.clone(),
                    cancel.clone(),
                    repaint.clone(),
                )
                .await;
                let mut view = state.lock().unwrap();
                view.session = None;
                view.peers.clear();
                view.links.clear();
                view.joined = None;
                view.busy = false;
                view.local_address.clear();
                view.published_addresses.clear();
                view.audio.clear();
                view.network.clear();
                view.chat.clear();
                if let Err(error) = result {
                    if !cancel.load(Ordering::Acquire) {
                        view.error = format!("{error:#}");
                    }
                }
                controls.pressed.store(false, Ordering::Relaxed);
                drop(view);
                repaint.request_repaint();
            }
            Command::Test(config) => {
                cancel.store(false, Ordering::Release);
                let (tx, _rx) = mpsc::channel(4);
                let st = state.clone();
                let ct = controls.clone();
                let rp = repaint.clone();
                {
                    let mut view = state.lock().unwrap();
                    view.busy = true;
                    view.error.clear();
                }
                repaint.request_repaint();
                match tokio::task::spawn_blocking(move || {
                    AudioHandle::start(config, ct, st, rp, tx)
                })
                .await
                {
                    Ok(Ok(audio)) => {
                        {
                            let mut view = state.lock().unwrap();
                            view.testing = true;
                            view.busy = false;
                        }
                        repaint.request_repaint();
                        loop {
                            match commands.recv().await {
                                Some(Command::StopTest | Command::Leave | Command::Shutdown)
                                | None => break,
                                _ => {}
                            }
                        }
                        let _ = tokio::task::spawn_blocking(move || drop(audio)).await;
                        let mut view = state.lock().unwrap();
                        view.testing = false;
                        view.audio.clear();
                        controls.level.store(0, Ordering::Relaxed);
                    }
                    result => {
                        let mut view = state.lock().unwrap();
                        view.busy = false;
                        view.error = match result {
                            Ok(Err(error)) => format!("{error:#}"),
                            Err(error) => error.to_string(),
                            _ => String::new(),
                        };
                    }
                }
                repaint.request_repaint();
            }
            Command::Shutdown => break,
            _ => {}
        }
    }
}
struct Link {
    connection: Connection,
    preferred: bool,
    tracker: SequenceTracker,
    sent: u64,
    audio_task: tokio::task::JoinHandle<()>,
    control_task: tokio::task::JoinHandle<()>,
}
impl Drop for Link {
    fn drop(&mut self) {
        self.connection.close(0u32.into(), b"peer left");
        self.audio_task.abort();
        self.control_task.abort();
    }
}
enum NetEvent {
    Snapshot(Snapshot),
    CoordinationError(String, bool),
    Connected(Peer, Connection, bool),
    Failed(String, String),
    Packet(String, usize, Bytes),
    Control(String, usize, ControlMessage),
    Closed(String, usize),
}
struct Live {
    endpoint: Endpoint,
    links: HashMap<String, Link>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
    audio: Option<AudioHandle>,
    mapping: Option<Arc<Mapping>>,
}
impl Drop for Live {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
        self.links.clear();
        self.endpoint.close(0u32.into(), b"room closed");
        if let Some(audio) = self.audio.take() {
            let _ = std::thread::Builder::new()
                .name("hush-audio-close".into())
                .spawn(move || drop(audio));
        }
        if let Some(mapping) = self.mapping.take() {
            let _ = std::thread::Builder::new()
                .name("hush-router-close".into())
                .spawn(move || drop(mapping));
        }
    }
}
async fn run_session(
    config: Config,
    invitation: Option<String>,
    commands: &mut mpsc::Receiver<Command>,
    state: SharedState,
    controls: Arc<Controls>,
    cancel: Arc<AtomicBool>,
    repaint: eframe::egui::Context,
) -> Result<()> {
    config.validate()?;
    let coordinator = Coordinator::new(&config.server)?;
    let identity = Arc::new(Identity::new()?);
    let endpoint = transport::endpoint(config.port)?;
    let port = endpoint.local_addr()?.port();
    let mut live = Live {
        endpoint: endpoint.clone(),
        links: HashMap::new(),
        tasks: vec![],
        audio: None,
        mapping: None,
    };
    let mut endpoints = transport::local_endpoints(port);
    let mut network = "LAN or manual UDP forwarding. No relay server is used.".to_owned();
    if config.auto_map && config.public_address.trim().is_empty() {
        state.lock().unwrap().network = "Checking your router for direct UDP access…".into();
        repaint.request_repaint();
        match tokio::task::spawn_blocking(move||transport::map_router(port)).await?{Ok(mapping)=>{endpoints.insert(0,mapping.public_address.clone());network=format!("Router mapped {}. Direct peer connections only.",mapping.public_address);live.mapping=Some(Arc::new(mapping));},Err(error)=>network=format!("Automatic router mapping unavailable: {error}. LAN works; for internet calls set your public UDP address and forward the port.")}
    }
    if !config.public_address.trim().is_empty() {
        endpoints.insert(0, config.public_address.trim().to_owned());
        network = "Using your manually configured public UDP address.".into();
    }
    if cancel.load(Ordering::Acquire) {
        return Ok(());
    }
    let (encoded_tx, mut encoded_rx) = mpsc::channel(8);
    let audio_config = config.clone();
    let audio_controls = controls.clone();
    let audio_state = state.clone();
    let audio_repaint = repaint.clone();
    live.audio = Some(
        tokio::task::spawn_blocking(move || {
            AudioHandle::start(
                audio_config,
                audio_controls,
                audio_state,
                audio_repaint,
                encoded_tx,
            )
        })
        .await??,
    );
    if cancel.load(Ordering::Acquire) {
        return Ok(());
    }
    let session = coordinator
        .enter(
            &config,
            invitation.as_deref(),
            &identity.published(),
            &endpoints,
        )
        .await?;
    if cancel.load(Ordering::Acquire) {
        let _ = coordinator.leave(&session).await;
        return Ok(());
    }
    let session = Arc::new(session);
    controls.muted.store(false, Ordering::Relaxed);
    controls.deafened.store(false, Ordering::Relaxed);
    controls.pressed.store(false, Ordering::Relaxed);
    {
        let mut view = state.lock().unwrap();
        view.session = Some((*session).clone());
        view.busy = false;
        view.peers.clear();
        view.network = network;
        view.local_address = format!("UDP {port}");
        view.published_addresses = endpoints.clone();
        view.joined = Some(Instant::now());
        view.chat.clear();
    }
    repaint.request_repaint();
    let roster = Arc::new(RwLock::new(Vec::<Peer>::new()));
    let (tx, mut events) = mpsc::channel(128);
    live.tasks.push(spawn_coordination(
        coordinator.clone(),
        session.clone(),
        endpoints,
        controls.clone(),
        tx.clone(),
    ));
    let accept_endpoint = endpoint.clone();
    let accept_roster = roster.clone();
    let accept_tx = tx.clone();
    let local_id = session.peer.clone();
    live.tasks.push(tokio::spawn(async move {
        let slots = Arc::new(tokio::sync::Semaphore::new(8));
        while let Some(incoming) = accept_endpoint.accept().await {
            let Ok(slot) = slots.clone().try_acquire_owned() else {
                incoming.refuse();
                continue;
            };
            let peers = accept_roster.clone();
            let tx = accept_tx.clone();
            let local = local_id.clone();
            tokio::spawn(async move {
                let _slot = slot;
                if let Ok(Ok(connection)) =
                    tokio::time::timeout(Duration::from_secs(5), incoming).await
                {
                    let peer = transport::identify(&connection, &peers.read().unwrap());
                    if let Some(peer) = peer {
                        let preferred = peer.id < local;
                        let _ = tx
                            .send(NetEvent::Connected(peer, connection, preferred))
                            .await;
                    } else {
                        connection.close(1u32.into(), b"not in roster");
                    }
                }
            });
        }
    }));
    if let Some(mapping) = live.mapping.clone() {
        live.tasks.push(tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1800)).await;
                let mapping = mapping.clone();
                let _ = tokio::task::spawn_blocking(move || mapping.renew()).await;
            }
        }));
    }
    let mut pending = HashSet::<String>::new();
    let mut stats_tick = tokio::time::interval(Duration::from_secs(2));
    let mut result = Ok(());
    enum Work {
        Command(Option<Command>),
        Encoded(Option<Encoded>),
        Event(Option<NetEvent>),
        Tick,
    }
    loop {
        let work = tokio::select! {
            command = commands.recv() => Work::Command(command),
            encoded = encoded_rx.recv() => Work::Encoded(encoded),
            event = events.recv() => Work::Event(event),
            _ = stats_tick.tick() => Work::Tick,
        };
        match work {
            Work::Command(command) => match command {
                Some(Command::Leave | Command::Shutdown) | None => break,
                Some(Command::Chat(text)) => {
                    let text = text.trim().chars().take(500).collect::<String>();
                    if !text.is_empty() {
                        if live.links.is_empty() && !roster.read().unwrap().is_empty() {
                            state.lock().unwrap().notice =
                                "Message not sent: wait for a peer connection.".into();
                            repaint.request_repaint();
                            continue;
                        }
                        broadcast_control(
                            &live.links,
                            &ControlMessage::Chat { text: text.clone() },
                        );
                        let mut view = state.lock().unwrap();
                        view.chat.push(Chat {
                            name: config.name.clone(),
                            text,
                        });
                        if view.chat.len() > 100 {
                            view.chat.remove(0);
                        }
                        drop(view);
                        repaint.request_repaint();
                    }
                }
                Some(Command::ControlChanged) => broadcast_control(
                    &live.links,
                    &ControlMessage::State {
                        muted: !controls.transmitting(),
                        deafened: controls.deafened.load(Ordering::Relaxed),
                    },
                ),
                _ => {}
            },
            Work::Encoded(encoded) => {
                if let Some(Encoded(bytes)) = encoded {
                    for link in live.links.values_mut() {
                        if link.connection.send_datagram(bytes.clone()).is_ok() {
                            link.sent += 1;
                        }
                    }
                } else {
                    result = Err(anyhow::anyhow!("The audio device stopped. Check the device connection, then rejoin the room."));
                    break;
                }
            }
            Work::Event(event) => match event {
                Some(NetEvent::Snapshot(snapshot)) => {
                    let peers: Vec<_> = snapshot
                        .peers
                        .into_iter()
                        .filter(|peer| peer.id != session.peer)
                        .collect();
                    let config_result = identity.server_config(&peers);
                    match config_result {
                        Ok(config) => endpoint.set_server_config(config),
                        Err(error) => {
                            result =
                                Err(error.context("Peer identities could not be authenticated"));
                            break;
                        }
                    }
                    *roster.write().unwrap() = peers.clone();
                    let removed: Vec<_> = live
                        .links
                        .keys()
                        .filter(|id| !peers.iter().any(|peer| &peer.id == *id))
                        .cloned()
                        .collect();
                    for id in removed {
                        live.links.remove(&id);
                        let _ = live
                            .audio
                            .as_ref()
                            .unwrap()
                            .incoming
                            .try_send(AudioCommand::Remove(id));
                    }
                    {
                        let mut view = state.lock().unwrap();
                        if let Some(current) = view.session.as_mut() {
                            current.room_name = snapshot.room_name;
                            current.max_members = snapshot.max_members;
                        }
                        view.peers = peers.clone();
                        if !view.coordination_online {
                            view.notice.clear();
                        }
                        view.coordination_online = true;
                    }
                    for peer in peers {
                        if !live.links.contains_key(&peer.id) && pending.insert(peer.id.clone()) {
                            state
                                .lock()
                                .unwrap()
                                .links
                                .entry(peer.id.clone())
                                .or_default()
                                .state = "Connecting".into();
                            let endpoint = endpoint.clone();
                            let identity = identity.clone();
                            let tx = tx.clone();
                            let local = session.peer.clone();
                            live.tasks.push(tokio::spawn(async move {
                                let id = peer.id.clone();
                                match connect_peer(endpoint, identity, &peer).await {
                                    Ok(connection) => {
                                        let preferred = local < id;
                                        let _ = tx
                                            .send(NetEvent::Connected(peer, connection, preferred))
                                            .await;
                                    }
                                    Err(error) => {
                                        let _ = tx
                                            .send(NetEvent::Failed(id, format!("{error:#}")))
                                            .await;
                                    }
                                }
                            }));
                        }
                    }
                    repaint.request_repaint();
                }
                Some(NetEvent::CoordinationError(message, fatal)) => {
                    if fatal {
                        result = Err(anyhow::anyhow!(message));
                        break;
                    }
                    let mut view = state.lock().unwrap();
                    view.coordination_online = false;
                    view.notice = format!(
                        "Room coordination is reconnecting. Existing voice can continue. {message}"
                    );
                    drop(view);
                    repaint.request_repaint();
                }
                Some(NetEvent::Connected(peer, connection, preferred)) => {
                    pending.remove(&peer.id);
                    if !roster.read().unwrap().iter().any(|member| {
                        member.id == peer.id && member.certificate == peer.certificate
                    }) {
                        connection.close(1u32.into(), b"membership changed");
                        continue;
                    }
                    if live
                        .links
                        .get(&peer.id)
                        .is_some_and(|link| link.preferred || !preferred)
                    {
                        connection.close(0u32.into(), b"duplicate connection");
                        continue;
                    }
                    let id = peer.id.clone();
                    let connection_id = connection.stable_id();
                    let audio_connection = connection.clone();
                    let audio_id = id.clone();
                    let audio_tx = tx.clone();
                    let audio_task = tokio::spawn(async move {
                        while let Ok(packet) = audio_connection.read_datagram().await {
                            if packet.len() >= 6
                                && packet.len() <= 1105
                                && packet[0] == 1
                                && audio_tx
                                    .send(NetEvent::Packet(audio_id.clone(), connection_id, packet))
                                    .await
                                    .is_err()
                            {
                                break;
                            }
                        }
                        let _ = audio_tx
                            .send(NetEvent::Closed(audio_id, connection_id))
                            .await;
                    });
                    let control_connection = connection.clone();
                    let control_id = id.clone();
                    let control_tx = tx.clone();
                    let control_task = tokio::spawn(async move {
                        while let Ok(mut stream) = control_connection.accept_uni().await {
                            if let Ok(Ok(bytes)) = tokio::time::timeout(
                                Duration::from_secs(3),
                                stream.read_to_end(4096),
                            )
                            .await
                            {
                                if let Ok(message) =
                                    serde_json::from_slice::<ControlMessage>(&bytes)
                                {
                                    let valid = match &message {
                                        ControlMessage::Chat { text } => {
                                            !text.trim().is_empty() && text.chars().count() <= 500
                                        }
                                        _ => true,
                                    };
                                    if valid
                                        && control_tx
                                            .send(NetEvent::Control(
                                                control_id.clone(),
                                                connection_id,
                                                message,
                                            ))
                                            .await
                                            .is_err()
                                    {
                                        break;
                                    }
                                }
                            }
                        }
                    });
                    let link = Link {
                        connection,
                        preferred,
                        tracker: SequenceTracker::default(),
                        sent: 0,
                        audio_task,
                        control_task,
                    };
                    send_control(
                        link.connection.clone(),
                        ControlMessage::State {
                            muted: !controls.transmitting(),
                            deafened: controls.deafened.load(Ordering::Relaxed),
                        },
                    );
                    live.links.insert(id.clone(), link);
                    state.lock().unwrap().links.entry(id).or_default().state = "Connected".into();
                    repaint.request_repaint();
                }
                Some(NetEvent::Failed(id, error)) => {
                    pending.remove(&id);
                    if !live.links.contains_key(&id) {
                        let mut view = state.lock().unwrap();
                        let stats = view.links.entry(id).or_default();
                        stats.state = "Blocked / retrying".into();
                        stats.last_error = error;
                        drop(view);
                        repaint.request_repaint();
                    }
                }
                Some(NetEvent::Packet(id, connection_id, packet)) => {
                    if let Some(link) = live
                        .links
                        .get_mut(&id)
                        .filter(|link| link.connection.stable_id() == connection_id)
                    {
                        let sequence = u32::from_be_bytes(packet[1..5].try_into().unwrap());
                        if link.tracker.record(sequence) {
                            let _ = live.audio.as_ref().unwrap().incoming.try_send(
                                AudioCommand::Packet {
                                    peer: id,
                                    sequence,
                                    bytes: packet.slice(5..),
                                },
                            );
                        }
                    }
                }
                Some(NetEvent::Control(id, connection_id, message)) => {
                    if live
                        .links
                        .get(&id)
                        .is_some_and(|link| link.connection.stable_id() == connection_id)
                    {
                        let mut view = state.lock().unwrap();
                        match message {
                            ControlMessage::Chat { text } => {
                                let name = view
                                    .peers
                                    .iter()
                                    .find(|peer| peer.id == id)
                                    .map_or("Squadmate".into(), |peer| peer.name.clone());
                                view.chat.push(Chat { name, text });
                                if view.chat.len() > 100 {
                                    view.chat.remove(0);
                                }
                            }
                            ControlMessage::State { muted, deafened } => {
                                if let Some(peer) = view.peers.iter_mut().find(|peer| peer.id == id)
                                {
                                    peer.muted = muted;
                                    peer.deafened = deafened;
                                }
                            }
                        }
                        drop(view);
                        repaint.request_repaint();
                    }
                }
                Some(NetEvent::Closed(id, connection_id)) => {
                    if live
                        .links
                        .get(&id)
                        .is_some_and(|link| link.connection.stable_id() == connection_id)
                    {
                        live.links.remove(&id);
                        let _ = live
                            .audio
                            .as_ref()
                            .unwrap()
                            .incoming
                            .try_send(AudioCommand::Remove(id.clone()));
                        state.lock().unwrap().links.entry(id).or_default().state =
                            "Reconnecting".into();
                        repaint.request_repaint();
                    }
                }
                None => break,
            },
            Work::Tick => {
                let mut view = state.lock().unwrap();
                let ids: HashSet<_> = view.peers.iter().map(|peer| peer.id.clone()).collect();
                view.links.retain(|id, _| ids.contains(id));
                for (id, link) in &live.links {
                    view.links.insert(
                        id.clone(),
                        LinkStats {
                            state: "Connected".into(),
                            rtt_ms: link.connection.rtt().as_secs_f64() * 1000.,
                            sent: link.sent,
                            received: link.tracker.received,
                            lost: link.tracker.lost(),
                            address: link.connection.remote_address().to_string(),
                            last_error: String::new(),
                        },
                    );
                }
                drop(view);
                repaint.request_repaint();
            }
        }
        controls
            .connections
            .store(live.links.len() as u32, Ordering::Relaxed);
        if cancel.load(Ordering::Acquire) {
            break;
        }
        live.tasks.retain(|task| !task.is_finished());
    }
    // Media and devices are closed before waiting for an optional server leave.
    controls.connections.store(0, Ordering::Relaxed);
    drop(live);
    let _ = tokio::time::timeout(Duration::from_secs(2), coordinator.leave(&session)).await;
    result
}
fn spawn_coordination(
    coordinator: Coordinator,
    session: Arc<Session>,
    endpoints: Vec<String>,
    controls: Arc<Controls>,
    events: mpsc::Sender<NetEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let start = Instant::now();
        loop {
            let result = coordinator
                .sync(
                    &session,
                    &endpoints,
                    !controls.transmitting(),
                    controls.deafened.load(Ordering::Relaxed),
                )
                .await;
            let event = match result {
                Ok(snapshot) => NetEvent::Snapshot(snapshot),
                Err(error) => {
                    let message = format!("{error:#}");
                    let fatal = message.contains("session has ended")
                        || message.contains("Room not found")
                        || message.contains("room has expired");
                    NetEvent::CoordinationError(message, fatal)
                }
            };
            let fatal = matches!(&event, NetEvent::CoordinationError(_, true));
            if events.send(event).await.is_err() || fatal {
                break;
            }
            tokio::time::sleep(if start.elapsed() < Duration::from_secs(20) {
                Duration::from_secs(1)
            } else {
                Duration::from_secs(5)
            })
            .await;
        }
    })
}
async fn connect_peer(
    endpoint: Endpoint,
    identity: Arc<Identity>,
    peer: &Peer,
) -> Result<Connection> {
    let config = identity.client_config(&peer.certificate)?;
    let mut last = "No reachable IPv4 address was advertised".to_owned();
    for address in &peer.endpoints {
        let Ok(address) = address.parse() else {
            continue;
        };
        match endpoint.connect_with(config.clone(), address, "hush.local") {
            Ok(connecting) => {
                match tokio::time::timeout(Duration::from_secs(2), connecting).await {
                    Ok(Ok(connection)) => return Ok(connection),
                    Ok(Err(error)) => last = error.to_string(),
                    Err(_) => last = "Direct UDP handshake timed out".into(),
                }
            }
            Err(error) => last = error.to_string(),
        }
    }
    bail!("{last}. Check the UDP port mapping, Windows Firewall, or carrier-grade NAT. Vercel does not relay voice.")
}
fn send_control(connection: Connection, message: ControlMessage) {
    tokio::spawn(async move {
        if let Ok(bytes) = serde_json::to_vec(&message) {
            let _ = tokio::time::timeout(Duration::from_secs(3), async {
                let mut stream = connection.open_uni().await?;
                stream.write_all(&bytes).await?;
                stream.finish()?;
                Ok::<_, anyhow::Error>(())
            })
            .await;
        }
    });
}
fn broadcast_control(links: &HashMap<String, Link>, message: &ControlMessage) {
    for link in links.values() {
        send_control(link.connection.clone(), message.clone());
    }
}
