use anyhow::Result;
use hidapi::HidApi;

use crate::hid::{DeviceSelector, GmkDevice};

pub fn run_info(selector: &DeviceSelector) -> Result<()> {
    let api = HidApi::new()?;
    let device = GmkDevice::open(&api, selector)?;

    println!("Device Information");
    println!("==================");
    println!();

    println!("Connection:");
    println!("  Path: {}", device.info.path);
    println!(
        "  VID:PID: {:04x}:{:04x}",
        device.info.vendor_id, device.info.product_id
    );
    println!("  Interface: {}", device.info.interface_number);
    println!(
        "  Usage: page=0x{:04x} usage=0x{:04x}",
        device.info.usage_page, device.info.usage
    );
    println!();

    println!("Device Strings:");
    if let Ok(Some(mfg)) = device.manufacturer() {
        println!("  Manufacturer: {}", mfg);
    } else if let Some(ref mfg) = device.info.manufacturer {
        println!("  Manufacturer: {}", mfg);
    } else {
        println!("  Manufacturer: (not available)");
    }

    if let Ok(Some(prod)) = device.product() {
        println!("  Product: {}", prod);
    } else if let Some(ref prod) = device.info.product {
        println!("  Product: {}", prod);
    } else {
        println!("  Product: (not available)");
    }

    if let Ok(Some(serial)) = device.serial() {
        println!("  Serial: {}", serial);
    } else if let Some(ref serial) = device.info.serial_number {
        println!("  Serial: {}", serial);
    } else {
        println!("  Serial: (not available)");
    }

    Ok(())
}
