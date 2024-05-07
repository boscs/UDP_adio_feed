use bitcode::{Decode, Encode};
use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering::Relaxed;
use std::time::Duration;
use std::usize;
const VERSION: u16 = 1;
pub const BUFFER_SIZE: u32 = 2;
pub type AudioNum = i16;

pub const FRAME_DURATION: Duration = Duration::from_millis(1);

#[derive(Encode, Decode, PartialEq, Debug)]
enum Message {
    SoundChunck {
        version: u16,
        message_counter: u16,
        sample_rate: u32,
        n_audio_channels: u16,
        data: Vec<AudioNum>,
    },
    Other,
}

static MESSAGE_COUNTER: AtomicU16 = AtomicU16::new(0);
static INCOMING_MESSAGE_COUNTER: AtomicU16 = AtomicU16::new(0);

pub fn pack_data(data: &Vec<AudioNum>, sample_rate: u32, n_audio_channels: u16) -> Vec<u8> {
    let msg = Message::SoundChunck {
        version: VERSION,
        message_counter: MESSAGE_COUNTER.fetch_add(1, Relaxed),
        sample_rate: sample_rate,
        n_audio_channels,
        data: data.clone(),
    };
    let ret = bitcode::encode(&msg);
    // println!("{msg:?}");
    ret
}
pub fn unpack_data(incoming_data: &[u8]) -> Option<(Vec<AudioNum>, u32, u16)> {
    let decoded: Message = bitcode::decode(incoming_data).ok()?;

    if let Message::SoundChunck {
        version,
        message_counter,
        sample_rate,
        data,
        n_audio_channels,
    } = decoded
    {
        assert!(VERSION == version);
        // let msg_type = (MessageType::sound_data as u16).to_le_bytes();
        let expected_incoming_msg_count =
            INCOMING_MESSAGE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if message_counter != expected_incoming_msg_count {
            // println!("error missed packet");
            INCOMING_MESSAGE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        return Some((data, sample_rate, n_audio_channels));
    } else {
        return None;
    }
}
