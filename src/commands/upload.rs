use anyhow::{bail, Result};
use hidapi::HidApi;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, info};
use std::path::Path;

use crate::hid::protocol::commands;
use crate::hid::{DeviceSelector, GmkDevice};
use crate::lcd::{ImageProcessor, LcdImage};

const MAX_DATA_PER_FRAME: usize = 56;
const FRAME_SIZE: usize = 0x10000;
const FRAMES_1_OFFSET: usize = 34;
const FRAMES_2_OFFSET: usize = 46;
const DELAY_OFFSET: usize = 43;

pub struct UploadOptions {
    pub width: u32,
    pub height: u32,
    pub resize: bool,
    pub ack_timeout_ms: i32,
}

impl Default for UploadOptions {
    fn default() -> Self {
        Self {
            width: 240,
            height: 135,
            resize: true,
            ack_timeout_ms: 5000,
        }
    }
}

pub fn run_upload<P: AsRef<Path>>(
    selector: &DeviceSelector,
    image_path: P,
    options: &UploadOptions,
) -> Result<()> {
    let api = HidApi::new()?;
    let mut device = GmkDevice::open(&api, selector)?;
    device.set_read_timeout(options.ack_timeout_ms);

    let path = image_path.as_ref();
    info!(
        "Uploading {} on {:04x}:{:04x}",
        path.display(),
        device.info.vendor_id,
        device.info.product_id
    );

    let processor = ImageProcessor::new(options.width, options.height).with_resize(options.resize);
    let animation = processor.load_animation(path)?;

    let frame_count = animation.frame_count();
    let delay_ms = animation.delay_ms;

    if frame_count == 1 {
        println!("Image: {}x{} -> {} bytes (static)",
            animation.frames[0].width,
            animation.frames[0].height,
            animation.frames[0].size());
    } else {
        println!("Animation: {} frames, {}ms delay", frame_count, delay_ms);
    }

    let mut encoded_frames: Vec<Vec<u8>> = Vec::with_capacity(frame_count);
    for frame in &animation.frames {
        encoded_frames.push(encode_frame(frame));
    }

    let mut encoded = Vec::with_capacity(encoded_frames.len() * FRAME_SIZE * 2);

    for frame in &encoded_frames {
        encoded.extend_from_slice(frame);
    }
    for frame in &encoded_frames {
        encoded.extend_from_slice(frame);
    }

    let total_bytes = encoded.len();
    let total_chunks = (total_bytes + MAX_DATA_PER_FRAME - 1) / MAX_DATA_PER_FRAME;

    println!("Encoded: {} bytes ({} chunks) [{} frames x 2 slots]",
        total_bytes, total_chunks, frame_count);

    print!("Loading config... ");
    let mut config = load_config(&device)?;
    println!("done");

    config[FRAMES_1_OFFSET] = frame_count as u8;
    config[FRAMES_2_OFFSET] = frame_count as u8;

    let delay = delay_ms.max(60);
    config[DELAY_OFFSET] = (delay & 0xFF) as u8;
    if DELAY_OFFSET + 1 < config.len() {
        config[DELAY_OFFSET + 1] = ((delay >> 8) & 0xFF) as u8;
    }

    debug!("Config: frames1={}, frames2={}, delay={}ms",
        config[FRAMES_1_OFFSET], config[FRAMES_2_OFFSET], delay);

    print!("Updating config... ");
    update_config(&device, &config)?;
    println!("done");

    print!("Starting upload... ");
    send_command(&device, commands::READY, &[], 0)?;
    send_command(&device, commands::INIT, &[], 0)?;
    println!("done");

    let pb = ProgressBar::new(total_chunks as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({percent}%)")
            .expect("Invalid progress bar template")
            .progress_chars("=>-"),
    );

    let mut pos: usize = 0;
    while pos < total_bytes {
        let chunk_size = std::cmp::min(MAX_DATA_PER_FRAME, total_bytes - pos);
        let chunk = &encoded[pos..pos + chunk_size];

        send_command(&device, commands::IMAGE_DATA, chunk, pos as u32)?;

        pos += chunk_size;
        pb.inc(1);
    }

    pb.finish_with_message("Transfer complete");

    print!("Committing... ");
    send_command(&device, commands::COMMIT, &[], 0)?;
    println!("done");

    println!("\nUpload complete!");
    Ok(())
}

pub fn run_upload_test_color(
    selector: &DeviceSelector,
    r: u8,
    g: u8,
    b: u8,
    options: &UploadOptions,
) -> Result<()> {
    let api = HidApi::new()?;
    let mut device = GmkDevice::open(&api, selector)?;
    device.set_read_timeout(options.ack_timeout_ms);

    println!("Uploading solid color RGB({}, {}, {})", r, g, b);

    let processor = ImageProcessor::new(options.width, options.height);
    let image = processor.solid_color(r, g, b);

    println!("Image: {}x{} -> {} bytes", image.width, image.height, image.size());

    let frame = encode_frame(&image);

    let mut encoded = Vec::with_capacity(frame.len() * 2);
    encoded.extend_from_slice(&frame);
    encoded.extend_from_slice(&frame);

    let total_bytes = encoded.len();
    let total_chunks = (total_bytes + MAX_DATA_PER_FRAME - 1) / MAX_DATA_PER_FRAME;

    println!("Encoded: {} bytes ({} chunks) [2 animation slots]", total_bytes, total_chunks);

    print!("Loading config... ");
    let mut config = load_config(&device)?;
    println!("done");

    config[FRAMES_1_OFFSET] = 1;
    config[FRAMES_2_OFFSET] = 1;

    print!("Updating config... ");
    update_config(&device, &config)?;
    println!("done");

    print!("Starting upload... ");
    send_command(&device, commands::READY, &[], 0)?;
    send_command(&device, commands::INIT, &[], 0)?;
    println!("done");

    let pb = ProgressBar::new(total_chunks as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({percent}%)")
            .expect("Invalid progress bar template")
            .progress_chars("=>-"),
    );

    let mut pos: usize = 0;
    while pos < total_bytes {
        let chunk_size = std::cmp::min(MAX_DATA_PER_FRAME, total_bytes - pos);
        let chunk = &encoded[pos..pos + chunk_size];

        send_command(&device, commands::IMAGE_DATA, chunk, pos as u32)?;

        pos += chunk_size;
        pb.inc(1);
    }

    pb.finish_with_message("Transfer complete");

    print!("Committing... ");
    send_command(&device, commands::COMMIT, &[], 0)?;
    println!("done");

    println!("\nUpload complete!");
    Ok(())
}

fn load_config(device: &GmkDevice) -> Result<Vec<u8>> {
    send_command(device, commands::INIT, &[], 0)?;

    for i in 0..9 {
        send_command(device, 0x03, &[0x00; 4], (i * 4) as u32)?;
    }
    send_command(device, 0x03, &[0x00], 36)?;

    send_command(device, commands::COMMIT, &[], 0)?;

    let mut config = Vec::with_capacity(48);
    for i in 0..12 {
        let response = send_command_with_response(device, 0x05, &[0x00; 4], (i * 4) as u32)?;
        config.extend_from_slice(&response[..4.min(response.len())]);
    }

    debug!("Loaded config: {} bytes", config.len());
    Ok(config)
}

fn update_config(device: &GmkDevice, config: &[u8]) -> Result<()> {
    send_command(device, commands::INIT, &[], 0)?;
    send_command(device, commands::CONFIG, config, 0)?;
    send_command(device, commands::COMMIT, &[], 0)?;
    Ok(())
}

fn send_command(device: &GmkDevice, command_id: u8, data: &[u8], pos: u32) -> Result<()> {
    send_command_with_response(device, command_id, data, pos)?;
    Ok(())
}

fn send_command_with_response(device: &GmkDevice, command_id: u8, data: &[u8], pos: u32) -> Result<Vec<u8>> {
    if data.len() > MAX_DATA_PER_FRAME {
        bail!("Data payload cannot exceed {} bytes", MAX_DATA_PER_FRAME);
    }

    if command_id == 2 {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let mut buffer = vec![0u8; 64];
    buffer[0] = 0x04;
    buffer[3] = command_id;
    buffer[4] = data.len() as u8;
    buffer[5] = (pos & 0xff) as u8;
    buffer[6] = ((pos >> 8) & 0xff) as u8;
    buffer[7] = ((pos >> 16) & 0xff) as u8;

    for (i, &b) in data.iter().enumerate() {
        if 8 + i < 64 {
            buffer[8 + i] = b;
        }
    }

    let checksum: u16 = buffer[3..64].iter().map(|&b| b as u16).sum();
    buffer[1] = (checksum & 0xff) as u8;
    buffer[2] = ((checksum >> 8) & 0xff) as u8;

    debug!("TX: cmd=0x{:02x} len={} pos={} checksum=0x{:04x}",
           command_id, data.len(), pos, checksum);

    device.write_raw(&buffer)?;

    loop {
        match device.read_raw(50000)? {
            Some(response) => {
                if response.len() >= 3 &&
                   response[0] == buffer[0] &&
                   response[1] == buffer[1] &&
                   response[2] == buffer[2] {
                    debug!("RX: ACK OK for cmd 0x{:02x}", command_id);
                    if response.len() > 4 {
                        return Ok(response[4..].to_vec());
                    }
                    return Ok(vec![]);
                }
                debug!("RX: got {:02x?}, continuing...", &response[..8.min(response.len())]);
            }
            None => {
                bail!("Timeout waiting for response to command 0x{:02x}", command_id);
            }
        }
    }
}

fn encode_frame(image: &LcdImage) -> Vec<u8> {
    let pixel_count = (image.width * image.height) as usize;

    let mut buffer = vec![0u8; FRAME_SIZE];

    for i in 0..pixel_count {
        let idx = i * 2;
        if idx + 1 < image.data.len() && idx + 1 < FRAME_SIZE {
            buffer[idx] = image.data[idx + 1];
            buffer[idx + 1] = image.data[idx];
        }
    }

    debug!("Encoded {} pixels to {} bytes (padded to {})", pixel_count, pixel_count * 2, FRAME_SIZE);
    buffer
}
