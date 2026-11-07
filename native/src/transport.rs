//! Native QUIC/TLS. Every peer presents a per-session certificate. The roster
//! obtained over authenticated HTTPS is the only trust root; there is no
//! accept-any-certificate verifier and no media path through Vercel.
use crate::model::Peer;
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use quinn::{
    crypto::rustls::{QuicClientConfig, QuicServerConfig},
    ClientConfig, Endpoint, ServerConfig, TransportConfig,
};
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
    RootCertStore,
};
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket},
    sync::Arc,
    time::Duration,
};

pub struct Identity {
    pub certificate: CertificateDer<'static>,
    key: Vec<u8>,
}
impl Identity {
    pub fn new() -> Result<Self> {
        let identity = rcgen::generate_simple_self_signed(vec!["hush.local".into()])?;
        Ok(Self {
            certificate: identity.cert.der().clone(),
            key: identity.key_pair.serialize_der(),
        })
    }
    pub fn published(&self) -> String {
        STANDARD.encode(self.certificate.as_ref())
    }
    fn private_key(&self) -> PrivateKeyDer<'static> {
        PrivatePkcs8KeyDer::from(self.key.clone()).into()
    }
    pub fn client_config(&self, certificate: &str) -> Result<ClientConfig> {
        let mut roots = RootCertStore::empty();
        roots.add(decode_certificate(certificate)?)?;
        let mut tls = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_client_auth_cert(vec![self.certificate.clone()], self.private_key())?;
        tls.alpn_protocols = vec![b"hush-voice/1".to_vec()];
        let mut config = ClientConfig::new(Arc::new(QuicClientConfig::try_from(tls)?));
        config.transport_config(transport_config());
        Ok(config)
    }
    pub fn server_config(&self, peers: &[Peer]) -> Result<Option<ServerConfig>> {
        if peers.is_empty() {
            return Ok(None);
        }
        let mut roots = RootCertStore::empty();
        for peer in peers {
            roots.add(decode_certificate(&peer.certificate)?)?;
        }
        let verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(roots)).build()?;
        let mut tls = rustls::ServerConfig::builder()
            .with_client_cert_verifier(verifier)
            .with_single_cert(vec![self.certificate.clone()], self.private_key())?;
        tls.alpn_protocols = vec![b"hush-voice/1".to_vec()];
        let mut config = ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(tls)?));
        config.transport_config(transport_config());
        Ok(Some(config))
    }
}
fn decode_certificate(value: &str) -> Result<CertificateDer<'static>> {
    let data = STANDARD
        .decode(value)
        .context("Peer certificate is invalid")?;
    if data.len() > 4096 {
        bail!("Peer identity is too large");
    }
    Ok(CertificateDer::from(data))
}
fn transport_config() -> Arc<TransportConfig> {
    let mut config = TransportConfig::default();
    config.keep_alive_interval(Some(Duration::from_secs(5)));
    config.max_idle_timeout(Some(Duration::from_secs(20).try_into().unwrap()));
    config.max_concurrent_bidi_streams(0u32.into());
    config.max_concurrent_uni_streams(8u32.into());
    config.datagram_receive_buffer_size(Some(64 * 1200));
    config.datagram_send_buffer_size(16 * 1200);
    config.receive_window(32_768u32.into());
    config.stream_receive_window(4096u32.into());
    Arc::new(config)
}
pub fn endpoint(port: u16) -> Result<Endpoint> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port)).with_context(|| {
        format!("UDP port {port} is in use or blocked. Choose a different port in Settings")
    })?;
    Endpoint::new(
        quinn::EndpointConfig::default(),
        None,
        socket,
        Arc::new(quinn::TokioRuntime),
    )
    .context("Could not open the native UDP transport")
}
pub fn identify(connection: &quinn::Connection, peers: &[Peer]) -> Option<Peer> {
    let identity = connection.peer_identity()?;
    let certificates = identity.downcast_ref::<Vec<CertificateDer<'static>>>()?;
    let certificate = certificates.first()?;
    peers
        .iter()
        .find(|peer| {
            STANDARD
                .decode(&peer.certificate)
                .is_ok_and(|der| der.as_slice() == certificate.as_ref())
        })
        .cloned()
}
pub fn local_endpoints(port: u16) -> Vec<String> {
    let mut endpoints: Vec<_> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|interface| match interface.ip() {
            std::net::IpAddr::V4(ip) if !ip.is_loopback() && !ip.is_link_local() => {
                Some(SocketAddrV4::new(ip, port).to_string())
            }
            _ => None,
        })
        .collect();
    endpoints.sort();
    endpoints.dedup();
    endpoints.truncate(6);
    endpoints.push(format!("127.0.0.1:{port}"));
    endpoints
}
pub struct Mapping {
    gateway: igd_next::Gateway,
    port: u16,
    pub public_address: String,
}
impl Mapping {
    pub fn renew(&self) -> Result<()> {
        let local = route_address(self.gateway.addr.ip(), self.port)?;
        self.gateway.add_port(
            igd_next::PortMappingProtocol::UDP,
            self.port,
            SocketAddr::V4(local),
            3600,
            "Hush native voice",
        )?;
        Ok(())
    }
}
impl Drop for Mapping {
    fn drop(&mut self) {
        let _ = self
            .gateway
            .remove_port(igd_next::PortMappingProtocol::UDP, self.port);
    }
}
fn route_address(gateway: std::net::IpAddr, port: u16) -> Result<SocketAddrV4> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    socket.connect(SocketAddr::new(gateway, 1900))?;
    match socket.local_addr()? {
        SocketAddr::V4(local) => Ok(SocketAddrV4::new(*local.ip(), port)),
        _ => bail!("The router did not expose an IPv4 route"),
    }
}
pub fn map_router(port: u16) -> Result<Mapping> {
    let gateway = igd_next::search_gateway(igd_next::SearchOptions {
        timeout: Some(Duration::from_secs(2)),
        ..Default::default()
    })
    .context("Router port mapping is unavailable")?;
    let ip = gateway.get_external_ip()?;
    let std::net::IpAddr::V4(ip) = ip else {
        bail!("The router did not provide a public IPv4 address");
    };
    if ip.is_private()
        || ip.is_loopback()
        || ip.is_unspecified()
        || (ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
    {
        bail!("Your router is behind another NAT. Use a reachable network or manual forwarding at the upstream router");
    }
    let local = route_address(gateway.addr.ip(), port)?;
    gateway.add_port(
        igd_next::PortMappingProtocol::UDP,
        port,
        SocketAddr::V4(local),
        3600,
        "Hush native voice",
    )?;
    Ok(Mapping {
        gateway,
        port,
        public_address: SocketAddrV4::new(ip, port).to_string(),
    })
}
#[derive(Default)]
pub struct SequenceTracker {
    highest: Option<u32>,
    first: u32,
    bitmap: u64,
    pub received: u64,
}
impl SequenceTracker {
    pub fn record(&mut self, sequence: u32) -> bool {
        if let Some(highest) = self.highest {
            if sequence > highest {
                let gap = sequence - highest;
                self.bitmap = if gap >= 64 {
                    1
                } else {
                    (self.bitmap << gap) | 1
                };
                self.highest = Some(sequence);
            } else {
                let behind = highest - sequence;
                if behind >= 64 || self.bitmap & (1u64 << behind) != 0 {
                    return false;
                }
                self.bitmap |= 1u64 << behind;
                self.first = self.first.min(sequence);
            }
        } else {
            self.highest = Some(sequence);
            self.first = sequence;
            self.bitmap = 1;
        }
        self.received += 1;
        true
    }
    pub fn lost(&self) -> u64 {
        self.highest
            .map(|highest| (highest - self.first) as u64 + 1)
            .unwrap_or(0)
            .saturating_sub(self.received)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn peer(id: &str, identity: &Identity) -> Peer {
        Peer {
            id: id.into(),
            name: id.into(),
            certificate: identity.published(),
            endpoints: vec![],
            muted: false,
            deafened: false,
            seen: 0,
        }
    }
    #[test]
    fn loss_counts_reordering_and_duplicates_correctly() {
        let mut sequence = SequenceTracker::default();
        assert!(sequence.record(10));
        assert!(sequence.record(12));
        assert_eq!(sequence.lost(), 1);
        assert!(sequence.record(11));
        assert_eq!(sequence.lost(), 0);
        assert!(!sequence.record(11));
        assert_eq!(sequence.received, 3);
        assert!(sequence.record(15));
        assert_eq!(sequence.lost(), 2);
    }
    #[tokio::test]
    async fn native_peers_authenticate_both_certificates_and_exchange_datagrams() {
        let a = Identity::new().unwrap();
        let b = Identity::new().unwrap();
        let stranger = Identity::new().unwrap();
        let left = endpoint(0).unwrap();
        let right = endpoint(0).unwrap();
        right.set_server_config(b.server_config(&[peer("left", &a)]).unwrap());
        let address = SocketAddr::from(([127, 0, 0, 1], right.local_addr().unwrap().port()));
        let outgoing = left
            .connect_with(
                a.client_config(&b.published()).unwrap(),
                address,
                "hush.local",
            )
            .unwrap();
        let incoming = right.accept().await.unwrap();
        let (client, server) = tokio::join!(outgoing, incoming);
        let client = client.unwrap();
        let server = server.unwrap();
        assert_eq!(identify(&server, &[peer("left", &a)]).unwrap().id, "left");
        client
            .send_datagram(bytes::Bytes::from_static(b"encrypted native audio"))
            .unwrap();
        assert_eq!(
            server.read_datagram().await.unwrap().as_ref(),
            b"encrypted native audio"
        );
        client.close(0u32.into(), b"done");
        server.close(0u32.into(), b"done");
        let rejected = left
            .connect_with(
                stranger.client_config(&b.published()).unwrap(),
                address,
                "hush.local",
            )
            .unwrap();
        let incoming = right.accept().await.unwrap();
        let (client, server) = tokio::join!(rejected, incoming);
        assert!(server.is_err());
        if let Ok(connection) = client {
            assert!(
                connection.closed().await.to_string().contains("crypto")
                    || connection.close_reason().is_some()
            );
        }
        left.close(0u32.into(), b"done");
        right.close(0u32.into(), b"done");
    }
}
