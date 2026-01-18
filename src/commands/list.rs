use anyhow::Result;
use hidapi::HidApi;

use crate::hid::{DeviceSelector, GmkDevice};

pub fn run_list(selector: &DeviceSelector, show_all: bool) -> Result<()> {
    let api = HidApi::new()?;

    let devices = if show_all {
        GmkDevice::enumerate_all(&api)
    } else {
        GmkDevice::enumerate(&api, selector)
    };

    if devices.is_empty() {
        if show_all {
            println!("No HID devices found.");
        } else {
            println!("No GMK104-compatible devices found.");
            println!(
                "  Searching for VID: {:04x}",
                selector.effective_vid()
            );
            if let Some(pid) = selector.effective_pid() {
                println!("  Searching for PID: {:04x}", pid);
            } else {
                println!("  PID: any (use --pid to filter)");
            }
            println!("\nTry 'gmk104ctl list --all' to see all HID devices.");
            println!("Or check device connection with 'lsusb' (Linux) or 'system_profiler SPUSBDataType' (macOS).");
        }
        return Ok(());
    }

    println!("Found {} device(s):\n", devices.len());

    for (i, dev) in devices.iter().enumerate() {
        println!("[{}] {:04x}:{:04x}", i, dev.vendor_id, dev.product_id);
        println!("    Path: {}", dev.path);

        if let Some(ref mfg) = dev.manufacturer {
            println!("    Manufacturer: {}", mfg);
        }
        if let Some(ref prod) = dev.product {
            println!("    Product: {}", prod);
        }
        if let Some(ref serial) = dev.serial_number {
            println!("    Serial: {}", serial);
        }

        println!(
            "    Interface: {}, Usage: page=0x{:04x} usage=0x{:04x}",
            dev.interface_number, dev.usage_page, dev.usage
        );
        println!();
    }

    if !show_all && devices.len() > 1 {
        println!("Tip: Use --path <path> to select a specific device.");
    }

    Ok(())
}
