//! Opt-in integration test against `npm run dev`. No microphone, speakers,
//! external STUN/TURN, or third-party server is involved.
pub async fn native_end_to_end() {
    use crate::{
        config::Config,
        coordination::Coordinator,
        model::ControlMessage,
        transport::{self, Identity},
    };
    use bytes::{BufMut, BytesMut};
    use opus::{Application, Channels, Decoder, Encoder};
    use std::{net::SocketAddr, time::Duration};
    let _ = rustls::crypto::ring::default_provider().install_default();
    let api = Coordinator::new("http://127.0.0.1:5173").unwrap();
    let a = Identity::new().unwrap();
    let b = Identity::new().unwrap();
    let left = transport::endpoint(0).unwrap();
    let right = transport::endpoint(0).unwrap();
    let addr_a = format!("127.0.0.1:{}", left.local_addr().unwrap().port());
    let addr_b = format!("127.0.0.1:{}", right.local_addr().unwrap().port());
    let config = Config {
        name: "Native integration A".into(),
        room_name: "Native integration".into(),
        ..Default::default()
    };
    let first = api
        .enter(&config, None, &a.published(), std::slice::from_ref(&addr_a))
        .await
        .unwrap();
    let second = api
        .enter(
            &Config {
                name: "Native integration B".into(),
                ..config.clone()
            },
            Some(&first.invitation()),
            &b.published(),
            std::slice::from_ref(&addr_b),
        )
        .await
        .unwrap();
    let roster_a = api.sync(&first, &[addr_a], false, false).await.unwrap();
    let roster_b = api
        .sync(&second, std::slice::from_ref(&addr_b), false, false)
        .await
        .unwrap();
    assert_eq!(roster_a.peers.len(), 2);
    assert_eq!(roster_b.peers.len(), 2);
    let peer_a = roster_b
        .peers
        .iter()
        .find(|peer| peer.id == first.peer)
        .unwrap();
    let peer_b = roster_a
        .peers
        .iter()
        .find(|peer| peer.id == second.peer)
        .unwrap();
    right.set_server_config(b.server_config(std::slice::from_ref(peer_a)).unwrap());
    let outgoing = left
        .connect_with(
            a.client_config(&peer_b.certificate).unwrap(),
            addr_b.parse::<SocketAddr>().unwrap(),
            "hush.local",
        )
        .unwrap();
    let incoming = right.accept().await.unwrap();
    let (client, server) = tokio::join!(outgoing, incoming);
    let client = client.unwrap();
    let server = server.unwrap();
    assert_eq!(
        transport::identify(&server, &roster_b.peers).unwrap().id,
        first.peer
    );
    let mut encoder = Encoder::new(48_000, Channels::Mono, Application::Voip).unwrap();
    encoder.set_complexity(3).unwrap();
    encoder.set_bitrate(opus::Bitrate::Bits(32_000)).unwrap();
    let mut decoder = Decoder::new(48_000, Channels::Mono).unwrap();
    let mut encoded = [0u8; 1100];
    let mut decoded = [0f32; 960];
    let mut energy = 0.;
    let start = std::time::Instant::now();
    let mut encoding_time = Duration::ZERO;
    for sequence in 0u32..12 {
        let pcm: Vec<_> = (0..960)
            .map(|i| {
                ((i + sequence as usize * 960) as f32 * 440. * std::f32::consts::TAU / 48_000.)
                    .sin()
                    * 0.2
            })
            .collect();
        let encode_start = std::time::Instant::now();
        let length = encoder.encode_float(&pcm, &mut encoded).unwrap();
        encoding_time += encode_start.elapsed();
        let mut packet = BytesMut::with_capacity(length + 5);
        packet.put_u8(1);
        packet.put_u32(sequence);
        packet.extend_from_slice(&encoded[..length]);
        client.send_datagram(packet.freeze()).unwrap();
        let received = tokio::time::timeout(Duration::from_secs(2), server.read_datagram())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            u32::from_be_bytes(received[1..5].try_into().unwrap()),
            sequence
        );
        assert_eq!(
            decoder
                .decode_float(&received[5..], &mut decoded, false)
                .unwrap(),
            960
        );
        energy += decoded.iter().map(|sample| sample * sample).sum::<f32>();
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(energy > 1.);
    eprintln!("Native path: 12 Opus frames authenticated, sent, and decoded in {:?}; mean encode compute {:?} per 20 ms frame (this Mac, synthetic input)",start.elapsed(),encoding_time/12);
    let message = serde_json::to_vec(&ControlMessage::Chat {
        text: "One more game?".into(),
    })
    .unwrap();
    let mut stream = client.open_uni().await.unwrap();
    stream.write_all(&message).await.unwrap();
    stream.finish().unwrap();
    let mut incoming = server.accept_uni().await.unwrap();
    let bytes = incoming.read_to_end(4096).await.unwrap();
    assert!(
        matches!(serde_json::from_slice::<ControlMessage>(&bytes).unwrap(),ControlMessage::Chat{text} if text=="One more game?")
    );
    client.close(0u32.into(), b"done");
    server.close(0u32.into(), b"done");
    left.close(0u32.into(), b"done");
    right.close(0u32.into(), b"done");
    api.leave(&second).await.unwrap();
    api.leave(&first).await.unwrap();
    assert!(api.sync(&first, &[], false, false).await.is_err());
}
