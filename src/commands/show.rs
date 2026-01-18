use anyhow::{bail, Result};
use hidapi::HidApi;

use crate::hid::{DeviceSelector, GmkDevice};

use super::init::build_config_report;

pub fn run_show(selector: &DeviceSelector, slot: u8) -> Result<()> {
    if slot > 1 {
        bail!("Invalid slot: {}. Must be 0 or 1.", slot);
    }

    let api = HidApi::new()?;
    let device = GmkDevice::open(&api, selector)?;

    device.drain()?;

    let shown_image = slot + 1;
    let config = build_config_report(shown_image, 1, 1);
    let ack = device.send_with_ack(&config)?;

    if !ack.is_ok() {
        bail!("Show slot {} failed: {}", slot, ack);
    }

    println!("Switched to slot {}", slot);
    Ok(())
}

pub fn run_show_time(selector: &DeviceSelector) -> Result<()> {
    let api = HidApi::new()?;
    let device = GmkDevice::open(&api, selector)?;

    device.drain()?;

    let config = build_config_report(0, 1, 1);
    let ack = device.send_with_ack(&config)?;

    if !ack.is_ok() {
        bail!("Show time failed: {}", ack);
    }

    println!("Switched to time display");
    Ok(())
}
