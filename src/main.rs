use anyhow::Result;
use clap::{Parser, Subcommand};
use log::LevelFilter;

mod commands;
mod hid;
mod lcd;
mod util;

use hid::DeviceSelector;
use util::parse_hex_u16;

#[derive(Parser, Debug)]
#[command(name = "gmk104ctl")]
#[command(about = "Control GMK104 keyboard LCD display", long_about = None)]
#[command(version)]
struct Cli {
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[arg(long, global = true, value_parser = parse_hex_u16)]
    vid: Option<u16>,

    #[arg(long, global = true, value_parser = parse_hex_u16)]
    pid: Option<u16>,

    #[arg(long, global = true)]
    serial: Option<String>,

    #[arg(long, global = true)]
    path: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    List {
        #[arg(long)]
        all: bool,
    },

    Info,

    Probe {
        #[arg(long, default_value = "0x01")]
        cmd: String,

        #[arg(long)]
        no_ack: bool,

        #[arg(long, default_value = "500")]
        listen_ms: u32,

        #[arg(long)]
        payload: Option<String>,

        #[arg(long)]
        handshake: bool,
    },

    Init {
        #[arg(long, default_value = "0")]
        shown: u8,
    },

    Upload {
        image: String,

        #[arg(long, default_value = "240")]
        width: u32,

        #[arg(long, default_value = "135")]
        height: u32,

        #[arg(long)]
        no_resize: bool,

        #[arg(long)]
        color: Option<String>,
    },

    Show {
        slot: String,
    },

    SyncTime,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let log_level = match cli.verbose {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };
    env_logger::Builder::new()
        .filter_level(log_level)
        .format_timestamp_millis()
        .init();

    let selector = DeviceSelector {
        vid: cli.vid,
        pid: cli.pid,
        serial: cli.serial,
        path: cli.path,
        product_filter: None,
    };

    match cli.command {
        Commands::List { all } => {
            commands::run_list(&selector, all)
        }

        Commands::Info => {
            commands::run_info(&selector)
        }

        Commands::Probe {
            cmd,
            no_ack,
            listen_ms,
            payload,
            handshake,
        } => {
            if handshake {
                commands::probe::run_probe_handshake(&selector)
            } else {
                let cmd_byte = parse_hex_u16(&cmd).map_err(anyhow::Error::msg)? as u8;
                let options = commands::probe::ProbeOptions {
                    cmd: cmd_byte,
                    no_ack,
                    listen_ms,
                    payload_hex: payload,
                };
                commands::run_probe(&selector, &options)
            }
        }

        Commands::Init { shown } => {
            let options = commands::init::InitOptions {
                shown,
                ..Default::default()
            };
            commands::run_init(&selector, &options)
        }

        Commands::Upload {
            image,
            width,
            height,
            no_resize,
            color,
        } => {
            let options = commands::upload::UploadOptions {
                width,
                height,
                resize: !no_resize,
                ..Default::default()
            };

            if let Some(color_str) = color {
                let rgb = parse_color(&color_str)?;
                commands::upload::run_upload_test_color(&selector, rgb.0, rgb.1, rgb.2, &options)
            } else {
                commands::run_upload(&selector, &image, &options)
            }
        }

        Commands::Show { slot } => {
            if slot.to_lowercase() == "time" {
                commands::show::run_show_time(&selector)
            } else {
                let slot_num: u8 = slot.parse()
                    .map_err(|_| anyhow::anyhow!("Invalid slot: '{}'. Use 0, 1, or 'time'", slot))?;
                commands::run_show(&selector, slot_num)
            }
        }

        Commands::SyncTime => {
            commands::run_sync_time(&selector)
        }
    }
}

fn parse_color(s: &str) -> Result<(u8, u8, u8)> {
    let s = s.trim();

    if s.starts_with('#') {
        let hex = s.trim_start_matches('#');
        if hex.len() != 6 {
            anyhow::bail!("Invalid hex color: {}. Use #RRGGBB format.", s);
        }
        let r = u8::from_str_radix(&hex[0..2], 16)?;
        let g = u8::from_str_radix(&hex[2..4], 16)?;
        let b = u8::from_str_radix(&hex[4..6], 16)?;
        return Ok((r, g, b));
    }

    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 3 {
        anyhow::bail!("Invalid color format: {}. Use R,G,B or #RRGGBB format.", s);
    }

    let r: u8 = parts[0].trim().parse()
        .map_err(|_| anyhow::anyhow!("Invalid red value: {}", parts[0]))?;
    let g: u8 = parts[1].trim().parse()
        .map_err(|_| anyhow::anyhow!("Invalid green value: {}", parts[1]))?;
    let b: u8 = parts[2].trim().parse()
        .map_err(|_| anyhow::anyhow!("Invalid blue value: {}", parts[2]))?;

    Ok((r, g, b))
}
