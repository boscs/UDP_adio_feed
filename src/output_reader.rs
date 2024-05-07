use dashmap::DashMap;

use cpal::traits::{DeviceTrait, StreamTrait};
use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::hash::Hash;
use std::net::UdpSocket;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

use crate::message_packing::{unpack_data, AudioNum, BUFFER_SIZE};
const STREAMZ_TIMEOUT: Duration = Duration::from_secs(5);

static OUTPUT_STREAMZZ: Lazy<DashMap<StreamerKey, Streamer>> = Lazy::new(|| DashMap::new());
static NEXT_STREAM_ID: AtomicU32 = AtomicU32::new(0);

/*

*/
struct IDStream(u32, cpal::Stream);
impl IDStream {
    fn new(s: cpal::Stream) -> Self {
        Self(NEXT_STREAM_ID.fetch_add(1, Ordering::Relaxed), s)
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
struct StreamerKey {
    emiter_ip: SocketAddr,
    sample_rate: u32,
    n_audio_channels: u16,
}
impl Hash for StreamerKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.emiter_ip.hash(state);
        self.sample_rate.hash(state);
        self.n_audio_channels.hash(state);
    }
}
impl Into<cpal::StreamConfig> for StreamerKey {
    fn into(self) -> cpal::StreamConfig {
        cpal::StreamConfig {
            channels: self.n_audio_channels,
            sample_rate: cpal::SampleRate(self.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(BUFFER_SIZE),
        }
    }
}
// #[derive(Send)]
struct Streamer {
    pub recive_pile_sender: mpsc::Sender<Vec<AudioNum>>,
    pub last_recv: Instant,
    pub stream_config: cpal::StreamConfig,
    pub stream_id: u32,
    pub stream_kill_sender: mpsc::Sender<u32>,
}

impl Streamer {
    pub fn new(
        sk: StreamerKey,
        output_device: &cpal::Device,
        output_streamz_process_storage: &mut Vec<IDStream>,
        stream_kill_sender: Sender<u32>,
    ) -> Self {
        let (s, r) = mpsc::channel::<Vec<AudioNum>>();

        let mut ret = Streamer {
            recive_pile_sender: s,
            last_recv: Instant::now(),
            stream_config: sk.into(),
            stream_id: 0,
            stream_kill_sender,
        };
        let mut current_audio_buff: VecDeque<AudioNum> = VecDeque::new();
        let output_fn = move |output: &mut [AudioNum], _: &cpal::OutputCallbackInfo| {
            let frame_size = output.len();
            for (i, x) in output.iter_mut().enumerate() {
                if current_audio_buff.len() == 0 {
                    current_audio_buff = r.recv().unwrap_or(vec![0; frame_size - i]).into();
                }
                *x = current_audio_buff.pop_front().unwrap();
            }
        };
        let output_stream = output_device
            .build_output_stream(&ret.stream_config, output_fn, err_fn, None)
            .unwrap();
        output_stream.play().unwrap();
        let wrapped_stream = IDStream::new(output_stream);
        ret.stream_id = wrapped_stream.0;
        output_streamz_process_storage.push(wrapped_stream);
        ret
    }
    pub fn process(&mut self, data: Vec<AudioNum>) {
        self.last_recv = std::time::Instant::now();
        let _ = self.recive_pile_sender.send(data);
    }
}
impl Drop for Streamer {
    fn drop(&mut self) {
        println!("dropping stream id: {}", self.stream_id);
        let _ = self.stream_kill_sender.send(self.stream_id);
    }
}

fn gc_streamerz(order66_pile: &Receiver<u32>, output_streamz_process_storage: &mut Vec<IDStream>) {
    OUTPUT_STREAMZZ.retain(|_, v| {
        let ret = v.last_recv.elapsed() < STREAMZ_TIMEOUT;
        if !ret {
            println!("discarding {}", v.stream_id);
        }
        ret
    });
    order66_pile
        .try_iter()
        .for_each(|k_to_kill| output_streamz_process_storage.retain(|x| x.0 != k_to_kill))
}

fn listen(output_device: cpal::Device, mut output_streamz_process_storage: &mut Vec<IDStream>) {
    let recv_sock = UdpSocket::bind(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        16789,
    ))
    .unwrap();
    let _ = recv_sock.set_read_timeout(Some(Duration::from_secs(1)));
    let mut recv_buf: [u8; 10000] = [0; 10000];
    let (stream_kill_sender, stream_kill_reciver) = mpsc::channel();
    loop {
        if let Ok((msg_len, emiter_ip)) = recv_sock.recv_from(&mut recv_buf) {
            if let Some((data, sample_rate, n_audio_channels)) = unpack_data(&recv_buf[..msg_len]) {
                let sk = StreamerKey {
                    emiter_ip: emiter_ip,
                    sample_rate,
                    n_audio_channels,
                };
                if !OUTPUT_STREAMZZ.contains_key(&sk) {
                    let new_stream = Streamer::new(
                        sk,
                        &output_device,
                        &mut output_streamz_process_storage,
                        stream_kill_sender.clone(),
                    );
                    OUTPUT_STREAMZZ.insert(sk, new_stream);
                }
                if let Some(mut streamer) = OUTPUT_STREAMZZ.get_mut(&sk) {
                    streamer.value_mut().process(data);
                }
            } else {
                eprintln!("Broken msg")
            }
        }
        gc_streamerz(&stream_kill_reciver, output_streamz_process_storage);
    }
}

pub fn initiate_output(output_device: cpal::Device) -> anyhow::Result<()> {
    let mut output_streamz_process_storage = Vec::<IDStream>::with_capacity(50);
    println!("Using output device: {}", output_device.name()?);
    listen(output_device, &mut output_streamz_process_storage);
    Ok(())
}

fn err_fn(err: cpal::StreamError) {
    eprintln!("an error occurred on stream: {}", err);
}
