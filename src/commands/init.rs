use anyhow::{bail, Result};
use hidapi::HidApi;
use log::debug;

use crate::hid::protocol::commands;
use crate::hid::{DeviceSelector, GmkDevice, ReportBuilder};
use crate::util::delay_ms;

pub struct InitOptions {
    pub shown: u8,
    pub pre_delay_ms: u64,
}

impl Default for InitOptions {
    fn default() -> Self {
        Self {
            shown: 0,
            pre_delay_ms: 200,
        }
    }
}

pub fn run_init(selector: &DeviceSelector, options: &InitOptions) -> Result<()> {
    let api = HidApi::new()?;
    let device = GmkDevice::open(&api, selector)?;

    init_lcd(&device, options.shown)?;

    println!("LCD initialized successfully");
    if options.shown > 0 {
        println!("Displaying slot {}", options.shown - 1);
    }

    Ok(())
}

pub fn init_lcd(device: &GmkDevice, shown_image: u8) -> Result<()> {
    debug!("Draining pending data...");
    device.drain()?;

    debug!("Sending RESET (0x00)...");
    let reset = ReportBuilder::new().command(commands::RESET).build();
    device.send_no_ack(&reset)?;

    delay_ms(50);

    debug!("Sending READY (0x23)...");
    let ready = ReportBuilder::new().command(commands::READY).build();
    device.send_no_ack(&ready)?;

    delay_ms(100);
    device.drain()?;

    delay_ms(200);

    debug!("Sending INIT (0x01)...");
    let init = ReportBuilder::new().command(commands::INIT).build();
    let ack = device.send_with_ack(&init)?;

    if !ack.is_ok() {
        debug!("First INIT failed, retrying...");
        delay_ms(100);
        let ack = device.send_with_ack(&init)?;
        if !ack.is_ok() {
            bail!("INIT command failed: {}", ack);
        }
    }

    debug!("Sending CONFIG (0x06) with shownImage={}...", shown_image);
    let config = build_config_report(shown_image, 1, 1);
    let ack = device.send_with_ack(&config)?;

    if !ack.is_ok() {
        bail!("CONFIG command failed: {}", ack);
    }

    debug!("Sending COMMIT (0x02)...");
    let commit = ReportBuilder::new().command(commands::COMMIT).build();
    let ack = device.send_with_ack(&commit)?;

    if !ack.is_ok() {
        bail!("COMMIT command failed: {}", ack);
    }

    Ok(())
}

pub fn reset_device_state(device: &GmkDevice) -> Result<()> {
    device.drain()?;

    let reset = ReportBuilder::new().command(commands::RESET).build();
    device.send_no_ack(&reset)?;

    delay_ms(50);

    let ready = ReportBuilder::new().command(commands::READY).build();
    device.send_no_ack(&ready)?;

    delay_ms(100);
    device.drain()?;
    delay_ms(200);

    Ok(())
}

pub fn build_config_report(shown_image: u8, image0_frames: u8, image1_frames: u8) -> crate::hid::Report {
    let mut builder = ReportBuilder::new().command(commands::CONFIG);

    builder = builder.byte(0x00, 0x29);
    builder = builder.byte(0x25, shown_image);
    builder = builder.byte(0x26, image0_frames);
    builder = builder.byte(0x32, image1_frames);

    let now = chrono::Local::now();
    builder = add_time_to_config(builder, &now);

    builder.build()
}

pub fn build_time_config_report() -> crate::hid::Report {
    let now = chrono::Local::now();

    let mut builder = ReportBuilder::new()
        .command(commands::CONFIG)
        .byte(0x00, 0x29);

    builder = add_time_to_config(builder, &now);
    builder.build()
}

fn add_time_to_config(
    builder: crate::hid::ReportBuilder,
    time: &chrono::DateTime<chrono::Local>,
) -> crate::hid::ReportBuilder {
    use chrono::{Datelike, Timelike};

    let year = time.year() as u32;
    let year_high = ((year / 100) % 100) as u8;
    let year_low = (year % 100) as u8;

    builder
        .byte(0x27, to_bcd(year_high))
        .byte(0x28, to_bcd(year_low))
        .byte(0x29, to_bcd(time.month() as u8))
        .byte(0x2A, to_bcd(time.day() as u8))
        .byte(0x2B, to_bcd(time.hour() as u8))
        .byte(0x2C, to_bcd(time.minute() as u8))
        .byte(0x2D, to_bcd(time.second() as u8))
        .byte(0x2E, time.weekday().num_days_from_sunday() as u8)
}

fn to_bcd(n: u8) -> u8 {
    ((n / 10) << 4) | (n % 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_bcd() {
        assert_eq!(to_bcd(0), 0x00);
        assert_eq!(to_bcd(9), 0x09);
        assert_eq!(to_bcd(10), 0x10);
        assert_eq!(to_bcd(25), 0x25);
        assert_eq!(to_bcd(59), 0x59);
        assert_eq!(to_bcd(99), 0x99);
    }
}
