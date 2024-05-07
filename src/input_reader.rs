use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{BufferSize, Device, InputCallbackInfo, StreamError};
use once_cell::sync::Lazy;
use std::net::UdpSocket;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::mpsc;

use crate::message_packing::{pack_data, AudioNum, BUFFER_SIZE, FRAME_DURATION};

static SOCK: Lazy<UdpSocket> = Lazy::new(|| {
    UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0)).unwrap()
});

fn error_callback(err: StreamError) {
    println!("StreamError: \"{}\"", err);
}
pub fn run_input_streamer(input_device: Device, target_address: SocketAddr) -> Option<()> {
    println!("Using input device: \"{}\"", input_device.name().unwrap());
    let mut config: cpal::StreamConfig = input_device.default_input_config().unwrap().into();
    config.buffer_size = BufferSize::Fixed(BUFFER_SIZE);
    let s_rate = config.sample_rate.0;
    let n_audio_channels = config.channels;
    let (s, r) = mpsc::channel();
    let input_data_fn = move |data: &[AudioNum], _info: &InputCallbackInfo| {
        for x in data {
            let _ = s.send(*x);
        }
    };
    println!("Attempting to build stream with `{config:?}`.");
    let input_stream = input_device
        .build_input_stream(&config, input_data_fn, error_callback, None)
        .ok()
        .expect("issues creating the stream");

    input_stream
        .play()
        .expect("issues with launching the stream");

    let len_buff: usize =
        (n_audio_channels as usize * s_rate as usize * FRAME_DURATION.as_millis() as usize) / 1000;
    let mut buf_audionum: Vec<AudioNum> = vec![0 as AudioNum; len_buff];
    loop {
        for i in 0..len_buff {
            buf_audionum[i] = r.recv().unwrap();
        }
        let arr = pack_data(&buf_audionum, s_rate, n_audio_channels);
        let _a = SOCK.send_to(&arr, target_address.clone());
    }
}
