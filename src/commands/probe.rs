use anyhow::Result;
use hidapi::HidApi;

use crate::hid::{DeviceSelector, GmkDevice, ReportBuilder};
use crate::hid::protocol::commands;
use crate::util::format_hex;

pub struct ProbeOptions {
    pub cmd: u8,
    pub no_ack: bool,
    pub listen_ms: u32,
    pub payload_hex: Option<String>,
}

impl Default for ProbeOptions {
    fn default() -> Self {
        Self {
            cmd: commands::INIT,
            no_ack: false,
            listen_ms: 500,
            payload_hex: None,
        }
    }
}

pub fn run_probe(selector: &DeviceSelector, options: &ProbeOptions) -> Result<()> {
    let api = HidApi::new()?;
    let mut device = GmkDevice::open(&api, selector)?;
    device.no_ack = options.no_ack;

    println!(
        "Probing device {:04x}:{:04x}",
        device.info.vendor_id, device.info.product_id
    );
    println!("Path: {}", device.info.path);
    println!();

    println!("Draining pending data...");
    let drained = device.drain()?;
    if drained > 0 {
        println!("  Drained {} bytes", drained);
    } else {
        println!("  No pending data");
    }
    println!();

    let mut builder = ReportBuilder::new().command(options.cmd);

    if let Some(ref hex) = options.payload_hex {
        let payload = parse_hex_payload(hex)?;
        builder = builder.bytes(0, &payload);
        println!("Payload ({} bytes): {}", payload.len(), format_hex(&payload));
    }

    let report = builder.build();

    println!("Sending command: 0x{:02x}", options.cmd);
    println!("Report checksum: 0x{:04x}", report.checksum());
    println!("First 16 bytes: {}", format_hex(&report.raw()[..16]));
    println!();

    if options.no_ack {
        device.send_no_ack(&report)?;
        println!("Sent (no ACK verification)");
    } else {
        let ack = device.send_with_ack(&report)?;
        println!("Response: {}", ack);

        if ack.is_ok() {
            println!("✓ ACK received successfully");
        } else {
            println!("✗ ACK failed or unexpected response");
        }
    }
    println!();

    if options.listen_ms > 0 {
        println!("Listening for {} ms...", options.listen_ms);
        device.set_read_timeout(100);

        let mut total_received = 0;
        let start = std::time::Instant::now();

        while start.elapsed().as_millis() < options.listen_ms as u128 {
            if let Some(data) = device.read_raw(100)? {
                total_received += 1;
                println!("  RX #{}: {} bytes", total_received, data.len());
                println!("    First 16: {}", format_hex(&data[..16.min(data.len())]));

                if data.len() >= 4 {
                    let cmd = data[3];
                    println!("    Command byte: 0x{:02x}", cmd);
                }
            }
        }

        if total_received == 0 {
            println!("  No additional responses received");
        } else {
            println!("  Total: {} response(s)", total_received);
        }
    }

    Ok(())
}

pub fn run_probe_handshake(selector: &DeviceSelector) -> Result<()> {
    let api = HidApi::new()?;
    let device = GmkDevice::open(&api, selector)?;

    println!(
        "Testing handshake with {:04x}:{:04x}",
        device.info.vendor_id, device.info.product_id
    );
    println!();

    device.drain()?;

    print!("1. RESET (0x00)... ");
    let reset = ReportBuilder::new().command(commands::RESET).build();
    let ack = device.send_with_ack(&reset)?;
    println!("{}", ack);

    print!("2. READY (0x23)... ");
    let ready = ReportBuilder::new().command(commands::READY).build();
    let ack = device.send_with_ack(&ready)?;
    println!("{}", ack);

    std::thread::sleep(std::time::Duration::from_millis(50));
    device.drain()?;

    print!("3. INIT (0x01)... ");
    let init = ReportBuilder::new().command(commands::INIT).build();
    let ack = device.send_with_ack(&init)?;
    println!("{}", ack);

    print!("4. CONFIG (0x06)... ");
    let config = ReportBuilder::new()
        .command(commands::CONFIG)
        .byte(0, 0x29)
        .build();
    let ack = device.send_with_ack(&config)?;
    println!("{}", ack);

    print!("5. COMMIT (0x02)... ");
    let commit = ReportBuilder::new().command(commands::COMMIT).build();
    let ack = device.send_with_ack(&commit)?;
    println!("{}", ack);

    println!();
    if ack.is_ok() {
        println!("✓ Handshake sequence completed successfully");
    } else {
        println!("✗ Handshake sequence had failures - protocol may differ");
    }

    Ok(())
}

fn parse_hex_payload(hex: &str) -> Result<Vec<u8>> {
    let hex = hex.replace(' ', "").replace("0x", "");

    if hex.len() % 2 != 0 {
        anyhow::bail!("Hex string must have even length");
    }

    let bytes: Result<Vec<u8>, _> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16))
        .collect();

    bytes.map_err(|e| anyhow::anyhow!("Invalid hex: {}", e))
}
