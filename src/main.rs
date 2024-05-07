extern crate anyhow;
extern crate cpal;
use clap::Parser;
use cpal::traits::HostTrait;
use input_reader::run_input_streamer;
use once_cell::sync::Lazy;
use output_reader::initiate_output;
use std::net::{SocketAddr, ToSocketAddrs};
mod input_reader;
mod message_packing;
mod output_reader;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Force the use as an output node even if a microphone is plugged in
    #[arg(short, long)]
    force_output: bool,
    #[arg(short, long)]
    sample_rate: Option<u32>,
    #[arg(short, long)]
    target_domain: Option<String>,
}

static CMD_ARGS: Lazy<Args> = Lazy::new(|| Args::parse());
static TARGET_ADDRESS: Lazy<SocketAddr> = Lazy::new(|| {
    let domain = CMD_ARGS
        .target_domain
        .clone()
        .unwrap_or("127.0.0.1:3000".to_string());
    domain.to_socket_addrs().unwrap().last().unwrap()
});

fn main() -> Result<(), anyhow::Error> {
    let host = cpal::default_host();
    let default_in = host.default_input_device();
    let default_out = host.default_output_device();
    if CMD_ARGS.force_output || default_in.is_none() {
        if let Some(output_device) = default_out {
            initiate_output(output_device)?;
        }
    } else {
        if let Some(input_peripheral) = default_in {
            run_input_streamer(input_peripheral, TARGET_ADDRESS.clone());
        }
    }

    Ok(())
}
