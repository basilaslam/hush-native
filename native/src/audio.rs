//! Device callbacks only convert samples and touch bounded SPSC rings. Opus,
//! jitter buffering and mixing run on one dedicated 20 ms worker.
use crate::{
    config::Config,
    model::{Controls, SharedState},
};
use anyhow::{bail, Context, Result};
use bytes::{BufMut, Bytes, BytesMut};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SizedSample,
};
use crossbeam_channel::{Receiver, Sender};
use opus::{Application, Bitrate, Channels, Decoder, Encoder};
use rtrb::{Consumer, Producer, RingBuffer};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

pub const RATE: u32 = 48_000;
pub const FRAME: usize = 960;
const TICK: Duration = Duration::from_millis(20);
const MAX_ENCODED: usize = 1100;
#[derive(Clone)]
pub struct Encoded(pub Bytes);
pub enum AudioCommand {
    Packet {
        peer: String,
        sequence: u32,
        bytes: Bytes,
    },
    Remove(String),
}
pub struct AudioHandle {
    pub incoming: Sender<AudioCommand>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl AudioHandle {
    pub fn start(
        config: Config,
        controls: Arc<Controls>,
        state: SharedState,
        repaint: eframe::egui::Context,
        encoded: tokio::sync::mpsc::Sender<Encoded>,
    ) -> Result<Self> {
        let (tx, rx) = crossbeam_channel::bounded(64);
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let thread = std::thread::Builder::new()
            .name("hush-audio".into())
            .spawn(move || {
                let result = run(AudioWorker {
                    config,
                    controls,
                    state: state.clone(),
                    repaint: repaint.clone(),
                    encoded,
                    commands: rx,
                    stop: stopped,
                    ready: ready_tx,
                });
                if let Err(error) = result {
                    state.lock().unwrap().error =
                        format!("Audio stopped: {error:#}. Check your devices, then rejoin.");
                    repaint.request_repaint();
                }
            })?;
        match ready_rx.recv_timeout(Duration::from_secs(15)) {
            Ok(Ok(())) => Ok(Self {
                incoming: tx,
                stop,
                thread: Some(thread),
            }),
            Ok(Err(message)) => {
                stop.store(true, Ordering::Relaxed);
                let _ = thread.join();
                bail!("{message}");
            }
            Err(_) => {
                stop.store(true, Ordering::Relaxed);
                bail!(
                    "Audio device initialization timed out. Check Windows microphone permissions."
                );
            }
        }
    }
}
impl Drop for AudioHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
pub fn devices(input: bool) -> Vec<String> {
    let host = cpal::default_host();
    let devices = if input {
        host.input_devices()
    } else {
        host.output_devices()
    };
    devices
        .map(|devices| devices.filter_map(|device| device.name().ok()).collect())
        .unwrap_or_default()
}
fn device(host: &cpal::Host, input: bool, name: &str) -> Result<cpal::Device> {
    if name.is_empty() {
        return (if input {
            host.default_input_device()
        } else {
            host.default_output_device()
        })
        .context(if input {
            "No microphone found"
        } else {
            "No playback device found"
        });
    }
    (if input {
        host.input_devices()?
    } else {
        host.output_devices()?
    })
    .find(|device| device.name().is_ok_and(|value| value == name))
    .context("The selected audio device is no longer available")
}
fn input_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut producer: Producer<f32>,
    errors: Sender<String>,
) -> Result<cpal::Stream>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;
    Ok(device.build_input_stream(
        config,
        move |data: &[T], _| {
            for frame in data.chunks(channels) {
                let mono = frame
                    .iter()
                    .map(|sample| f32::from_sample(*sample))
                    .sum::<f32>()
                    / channels as f32;
                let _ = producer.push(mono);
            }
        },
        move |error| {
            let _ = errors.try_send(error.to_string());
        },
        None,
    )?)
}
fn output_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut consumer: Consumer<f32>,
    controls: Arc<Controls>,
    errors: Sender<String>,
) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    Ok(device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            let volume = f32::from_bits(controls.volume.load(Ordering::Relaxed));
            let deafened = controls.deafened.load(Ordering::Relaxed);
            for frame in data.chunks_mut(channels) {
                let mono = consumer.pop().unwrap_or(0.0);
                let value = if deafened {
                    0.
                } else {
                    (mono * volume).clamp(-1., 1.)
                };
                for sample in frame {
                    *sample = T::from_sample(value);
                }
            }
        },
        move |error| {
            let _ = errors.try_send(error.to_string());
        },
        None,
    )?)
}
macro_rules! device_stream {
    ($format:expr, $builder:ident, $($args:expr),+) => { match $format {
        cpal::SampleFormat::F32 => $builder::<f32>($($args),+), cpal::SampleFormat::I16 => $builder::<i16>($($args),+),
        cpal::SampleFormat::U16 => $builder::<u16>($($args),+), cpal::SampleFormat::I32 => $builder::<i32>($($args),+),
        cpal::SampleFormat::F64 => $builder::<f64>($($args),+), format => anyhow::bail!("Unsupported device sample format: {format}")
    } };
}
fn make_encoder(bitrate: i32) -> Result<Encoder> {
    let mut encoder = Encoder::new(RATE, Channels::Mono, Application::Voip)?;
    encoder.set_bitrate(Bitrate::Bits(bitrate))?;
    encoder.set_complexity(3)?;
    encoder.set_vbr(true)?;
    encoder.set_dtx(true)?;
    encoder.set_inband_fec(true)?;
    encoder.set_packet_loss_perc(10)?;
    Ok(encoder)
}
struct AudioWorker {
    config: Config,
    controls: Arc<Controls>,
    state: SharedState,
    repaint: eframe::egui::Context,
    encoded: tokio::sync::mpsc::Sender<Encoded>,
    commands: Receiver<AudioCommand>,
    stop: Arc<AtomicBool>,
    ready: std::sync::mpsc::SyncSender<std::result::Result<(), String>>,
}
fn run(worker: AudioWorker) -> Result<()> {
    let AudioWorker {
        config,
        controls,
        state,
        repaint,
        encoded,
        commands,
        stop,
        ready,
    } = worker;
    let initialized = (|| -> Result<_> {
        let host = cpal::default_host();
        let input = device(&host, true, &config.input)?;
        let output = device(&host, false, &config.output)?;
        let input_format = input.default_input_config()?;
        let output_format = output.default_output_config()?;
        let input_config: cpal::StreamConfig = input_format.clone().into();
        let output_config: cpal::StreamConfig = output_format.clone().into();
        let (capture_tx, capture_rx) =
            RingBuffer::<f32>::new(input_config.sample_rate.0 as usize / 10);
        let (playback_tx, playback_rx) =
            RingBuffer::<f32>::new(output_config.sample_rate.0 as usize / 10);
        let (errors, error_rx) = crossbeam_channel::bounded(4);
        let input_stream = device_stream!(
            input_format.sample_format(),
            input_stream,
            &input,
            &input_config,
            capture_tx,
            errors.clone()
        )?;
        let output_stream = device_stream!(
            output_format.sample_format(),
            output_stream,
            &output,
            &output_config,
            playback_rx,
            controls.clone(),
            errors
        )?;
        let encoder = make_encoder(config.bitrate)?;
        input_stream.play()?;
        output_stream.play()?;
        let description = format!("{} → Opus 48 kHz mono → {}", input.name()?, output.name()?);
        Ok((
            input_stream,
            output_stream,
            capture_rx,
            playback_tx,
            input_config.sample_rate.0,
            output_config.sample_rate.0,
            encoder,
            error_rx,
            description,
        ))
    })();
    let (
        input_stream,
        output_stream,
        mut capture,
        mut playback,
        input_rate,
        output_rate,
        mut encoder,
        errors,
        description,
    ) = match initialized {
        Ok(value) => {
            let _ = ready.send(Ok(()));
            value
        }
        Err(error) => {
            let _ = ready.send(Err(format!("{error:#}")));
            return Ok(());
        }
    };
    state.lock().unwrap().audio = description;
    repaint.request_repaint();
    let _streams = (input_stream, output_stream); // Lifetime stays on this thread, including on macOS.
    let mut input_resampler = Resampler::new(input_rate, RATE);
    let mut output_resampler = Resampler::new(RATE, output_rate);
    let mut captures = std::collections::VecDeque::<f32>::with_capacity(FRAME * 4);
    let mut decoders: HashMap<String, JitterDecoder> = HashMap::new();
    let mut next = Instant::now() + TICK;
    let mut sequence = 0u32;
    let mut pcm = [0f32; FRAME];
    let mut mix = [0f32; FRAME];
    let mut decoded = [0f32; FRAME];
    let mut opus = [0u8; MAX_ENCODED];
    while !stop.load(Ordering::Acquire) {
        if let Ok(message) = errors.try_recv() {
            bail!("{message}");
        }
        // Bounded work prevents a noisy peer from delaying the audio deadline.
        for _ in 0..128 {
            match commands.try_recv() {
                Ok(AudioCommand::Packet {
                    peer,
                    sequence,
                    bytes,
                }) => {
                    if decoders.len() < 5 || decoders.contains_key(&peer) {
                        if let std::collections::hash_map::Entry::Vacant(entry) =
                            decoders.entry(peer.clone())
                        {
                            entry.insert(JitterDecoder::new()?);
                        }
                        decoders.get_mut(&peer).unwrap().insert(sequence, bytes);
                    }
                }
                Ok(AudioCommand::Remove(peer)) => {
                    decoders.remove(&peer);
                }
                Err(_) => break,
            }
        }
        while let Ok(sample) = capture.pop() {
            input_resampler.push(sample, |value| {
                if captures.len() < FRAME * 4 {
                    captures.push_back(value);
                }
            });
        }
        if Instant::now() < next {
            std::thread::sleep(next.saturating_duration_since(Instant::now()));
            continue;
        }
        let gain = f32::from_bits(controls.gain.load(Ordering::Relaxed));
        let mut power = 0.;
        while captures.len() > FRAME * 2 {
            captures.pop_front();
        }
        for sample in &mut pcm {
            *sample = (captures.pop_front().unwrap_or(0.) * gain).clamp(-1., 1.);
            power += *sample * *sample;
        }
        controls
            .level
            .store((power / FRAME as f32).sqrt().to_bits(), Ordering::Relaxed);
        if controls.transmitting() && controls.connections.load(Ordering::Relaxed) > 0 {
            let size = encoder.encode_float(&pcm, &mut opus)?;
            let mut packet = BytesMut::with_capacity(size + 5);
            packet.put_u8(1);
            packet.put_u32(sequence);
            packet.extend_from_slice(&opus[..size]);
            // No retransmission of obsolete voice frames; a full queue drops this frame.
            let _ = encoded.try_send(Encoded(packet.freeze()));
            sequence = sequence.wrapping_add(1);
        }
        mix.fill(0.);
        let mut active = 0;
        for decoder in decoders.values_mut() {
            decoded.fill(0.);
            if decoder.render(&mut decoded) {
                active += 1;
                for (target, source) in mix.iter_mut().zip(decoded) {
                    *target += source;
                }
            }
        }
        let scale = (active.max(1) as f32).sqrt();
        for value in &mix {
            output_resampler.push((value / scale).clamp(-1., 1.), |sample| {
                let _ = playback.push(sample);
            });
        }
        next += TICK;
        if next + TICK * 2 < Instant::now() {
            next = Instant::now() + TICK;
        }
    }
    controls.level.store(0, Ordering::Relaxed);
    Ok(())
}
/// Continuous linear resampling without an allocation in either device callback.
pub struct Resampler {
    ratio: f64,
    phase: f64,
    previous: f32,
}
impl Resampler {
    pub fn new(input: u32, output: u32) -> Self {
        Self {
            ratio: output as f64 / input as f64,
            phase: 0.,
            previous: 0.,
        }
    }
    pub fn push(&mut self, value: f32, mut emit: impl FnMut(f32)) {
        self.phase += self.ratio;
        while self.phase >= 1. {
            let fraction = (1. - (self.phase - 1.) / self.ratio).clamp(0., 1.) as f32;
            emit(self.previous + (value - self.previous) * fraction);
            self.phase -= 1.;
        }
        self.previous = value;
    }
}
struct JitterDecoder {
    decoder: Decoder,
    packets: BTreeMap<u32, Bytes>,
    expected: Option<u32>,
    arrived: Instant,
    waiting_since: Instant,
    missing: u8,
}
impl JitterDecoder {
    fn new() -> Result<Self> {
        Ok(Self {
            decoder: Decoder::new(RATE, Channels::Mono)?,
            packets: BTreeMap::new(),
            expected: None,
            arrived: Instant::now(),
            waiting_since: Instant::now(),
            missing: 0,
        })
    }
    fn insert(&mut self, sequence: u32, bytes: Bytes) {
        if self.expected.is_some_and(|expected| sequence < expected) {
            return;
        }
        if self.packets.is_empty() && self.expected.is_none() {
            self.waiting_since = Instant::now();
        }
        self.arrived = Instant::now();
        self.packets.insert(sequence, bytes);
        while self.packets.len() > 8 {
            self.packets.pop_first();
            self.expected = self
                .packets
                .first_key_value()
                .map(|(sequence, _)| *sequence);
        }
    }
    fn render(&mut self, output: &mut [f32; FRAME]) -> bool {
        if self.expected.is_none() {
            if self.packets.is_empty()
                || (self.packets.len() < 2
                    && self.waiting_since.elapsed() < Duration::from_millis(40))
            {
                return false;
            }
            self.expected = self
                .packets
                .first_key_value()
                .map(|(sequence, _)| *sequence);
        }
        let sequence = self.expected.unwrap();
        if let Some(packet) = self.packets.remove(&sequence) {
            self.missing = 0;
            let _ = self.decoder.decode_float(&packet, output, false);
        } else if self.missing < 3 && self.arrived.elapsed() < Duration::from_millis(100) {
            self.missing += 1;
            let fec = self.packets.get(&sequence.wrapping_add(1));
            let _ = self.decoder.decode_float(
                fec.map_or(&[][..], |packet| packet.as_ref()),
                output,
                fec.is_some(),
            );
        } else {
            self.expected = None;
            self.missing = 0;
            return false;
        }
        self.expected = Some(sequence.wrapping_add(1));
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resampling_preserves_duration_for_common_windows_device_rates() {
        for (input, output) in [
            (44100, 48000),
            (48000, 44100),
            (48000, 48000),
            (96000, 48000),
        ] {
            let mut converter = Resampler::new(input, output);
            let mut count = 0;
            for _ in 0..input {
                converter.push(0.5, |sample| {
                    assert!(sample.is_finite());
                    count += 1;
                });
            }
            assert!(
                (count as i64 - output as i64).abs() <= 1,
                "{input} → {output}: {count}"
            );
        }
    }
    #[test]
    fn opus_roundtrip_decodes_real_audio_and_jitter_is_bounded() {
        let mut encoder = make_encoder(32000).unwrap();
        let pcm: Vec<f32> = (0..FRAME)
            .map(|i| (i as f32 * 440. * std::f32::consts::TAU / RATE as f32).sin() * 0.2)
            .collect();
        let mut packet = [0u8; MAX_ENCODED];
        let length = encoder.encode_float(&pcm, &mut packet).unwrap();
        assert!(length > 5);
        let bytes = Bytes::copy_from_slice(&packet[..length]);
        let mut decoder = JitterDecoder::new().unwrap();
        decoder.insert(1, bytes.clone());
        decoder.insert(0, bytes.clone());
        let mut output = [0.; FRAME];
        assert!(decoder.render(&mut output));
        assert!(output.iter().any(|sample| sample.abs() > 0.0001));
        for i in 2..100 {
            decoder.insert(i, bytes.clone());
        }
        assert!(decoder.packets.len() <= 8);
    }
    #[test]
    fn deafen_and_ptt_cannot_leak_microphone_transmission() {
        let controls = Controls::default();
        assert!(controls.transmitting());
        controls.ptt.store(true, Ordering::Relaxed);
        assert!(!controls.transmitting());
        controls.pressed.store(true, Ordering::Relaxed);
        assert!(controls.transmitting());
        controls.deafened.store(true, Ordering::Relaxed);
        assert!(!controls.transmitting());
        controls.deafened.store(false, Ordering::Relaxed);
        controls.muted.store(true, Ordering::Relaxed);
        assert!(!controls.transmitting());
    }
}
