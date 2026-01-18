use byteorder::{BigEndian, ByteOrder, LittleEndian};
use log::trace;

use super::{COMMAND_OFFSET, PAYLOAD_OFFSET, PAYLOAD_SIZE, REPORT_ID, REPORT_SIZE};

pub mod commands {
    pub const RESET: u8 = 0x00;
    pub const INIT: u8 = 0x01;
    pub const COMMIT: u8 = 0x02;
    pub const CONFIG: u8 = 0x06;
    pub const IMAGE_DATA: u8 = 0x21;
    pub const READY: u8 = 0x23;
}

#[derive(Clone)]
pub struct Report {
    data: [u8; REPORT_SIZE],
}

impl Report {
    pub fn new() -> Self {
        Self {
            data: [0u8; REPORT_SIZE],
        }
    }

    pub fn from_bytes(data: [u8; REPORT_SIZE]) -> Self {
        Self { data }
    }

    pub fn command(&self) -> u8 {
        self.data[COMMAND_OFFSET]
    }

    pub fn set_command(&mut self, cmd: u8) {
        self.data[COMMAND_OFFSET] = cmd;
    }

    pub fn payload(&self) -> &[u8] {
        &self.data[PAYLOAD_OFFSET..REPORT_SIZE]
    }

    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.data[PAYLOAD_OFFSET..REPORT_SIZE]
    }

    pub fn set_payload(&mut self, offset: usize, bytes: &[u8]) {
        let start = PAYLOAD_OFFSET + offset;
        let end = start + bytes.len();
        assert!(
            end <= REPORT_SIZE,
            "Payload overflow: trying to write {} bytes at offset {}, max is {}",
            bytes.len(),
            offset,
            PAYLOAD_SIZE
        );
        self.data[start..end].copy_from_slice(bytes);
    }

    pub fn set_payload_byte(&mut self, offset: usize, value: u8) {
        assert!(
            offset < PAYLOAD_SIZE,
            "Payload offset {} out of range (max {})",
            offset,
            PAYLOAD_SIZE - 1
        );
        self.data[PAYLOAD_OFFSET + offset] = value;
    }

    pub fn finalize(&mut self) {
        let sum: u16 = self.data[COMMAND_OFFSET..REPORT_SIZE]
            .iter()
            .map(|&b| b as u16)
            .sum();
        LittleEndian::write_u16(&mut self.data[1..3], sum);
        trace!("Report checksum: 0x{:04x}", sum);
    }

    pub fn verify_checksum(&self) -> bool {
        let stored = LittleEndian::read_u16(&self.data[1..3]);
        let computed: u16 = self.data[COMMAND_OFFSET..REPORT_SIZE]
            .iter()
            .map(|&b| b as u16)
            .sum();
        stored == computed
    }

    pub fn checksum(&self) -> u16 {
        LittleEndian::read_u16(&self.data[1..3])
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(REPORT_SIZE + 1);
        buf.push(REPORT_ID);
        buf.extend_from_slice(&self.data);
        buf
    }

    pub fn raw(&self) -> &[u8; REPORT_SIZE] {
        &self.data
    }

    pub fn header(&self, n: usize) -> &[u8] {
        &self.data[..n.min(REPORT_SIZE)]
    }
}

impl Default for Report {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Report {{ cmd: 0x{:02x}, checksum: 0x{:04x}, payload: [{:02x} {:02x} {:02x} {:02x} ...] }}",
            self.command(),
            self.checksum(),
            self.data[PAYLOAD_OFFSET],
            self.data[PAYLOAD_OFFSET + 1],
            self.data[PAYLOAD_OFFSET + 2],
            self.data[PAYLOAD_OFFSET + 3],
        )
    }
}

pub struct ReportBuilder {
    report: Report,
}

impl ReportBuilder {
    pub fn new() -> Self {
        Self {
            report: Report::new(),
        }
    }

    pub fn command(mut self, cmd: u8) -> Self {
        self.report.set_command(cmd);
        self
    }

    pub fn byte(mut self, offset: usize, value: u8) -> Self {
        self.report.set_payload_byte(offset, value);
        self
    }

    pub fn u16_be(mut self, offset: usize, value: u16) -> Self {
        let start = PAYLOAD_OFFSET + offset;
        BigEndian::write_u16(&mut self.report.data[start..], value);
        self
    }

    pub fn u16_le(mut self, offset: usize, value: u16) -> Self {
        let start = PAYLOAD_OFFSET + offset;
        LittleEndian::write_u16(&mut self.report.data[start..], value);
        self
    }

    pub fn bytes(mut self, offset: usize, data: &[u8]) -> Self {
        self.report.set_payload(offset, data);
        self
    }

    pub fn build(mut self) -> Report {
        self.report.finalize();
        self.report
    }

    pub fn build_raw(self) -> Report {
        self.report
    }
}

impl Default for ReportBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AckStatus {
    Ok,
    ChecksumMismatch { expected: u16, got: u16 },
    HeaderMismatch { expected: Vec<u8>, got: Vec<u8> },
    Timeout,
    Error(u8),
}

impl AckStatus {
    pub fn from_response(sent: &Report, response: &[u8; REPORT_SIZE]) -> Self {
        let sent_header = sent.header(8);

        let resp_header_direct = &response[..8];
        if sent_header == resp_header_direct {
            return Self::Ok;
        }

        if response[0] == REPORT_ID {
            let resp_header_shifted = &response[1..8];
            let sent_header_short = &sent.raw()[..7];
            if sent_header_short == resp_header_shifted {
                return Self::Ok;
            }
        }

        let sent_cmd = sent.command();
        let resp_cmd_direct = response[COMMAND_OFFSET];
        let resp_cmd_shifted = response[COMMAND_OFFSET + 1];

        if sent_cmd == resp_cmd_direct || sent_cmd == resp_cmd_shifted {
            return Self::Ok;
        }

        Self::HeaderMismatch {
            expected: sent_header.to_vec(),
            got: resp_header_direct.to_vec(),
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok)
    }
}

impl std::fmt::Display for AckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok => write!(f, "OK"),
            Self::ChecksumMismatch { expected, got } => {
                write!(f, "Checksum mismatch: expected 0x{:04x}, got 0x{:04x}", expected, got)
            }
            Self::HeaderMismatch { expected, got } => {
                write!(
                    f,
                    "Header mismatch: expected {:02x?}, got {:02x?}",
                    expected, got
                )
            }
            Self::Timeout => write!(f, "Timeout"),
            Self::Error(code) => write!(f, "Error 0x{:02x}", code),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_checksum() {
        let mut report = Report::new();
        report.set_command(0x01);
        report.finalize();

        assert!(report.verify_checksum());
        assert_eq!(report.checksum(), 0x0001);
    }

    #[test]
    fn test_report_builder() {
        let report = ReportBuilder::new()
            .command(0x21)
            .byte(0, 0x38)
            .u16_le(1, 0x1234)
            .build();

        assert_eq!(report.command(), 0x21);
        assert_eq!(report.payload()[0], 0x38);
        assert_eq!(report.payload()[1], 0x34);
        assert_eq!(report.payload()[2], 0x12);
        assert!(report.verify_checksum());
    }

    #[test]
    fn test_report_to_bytes() {
        let report = ReportBuilder::new().command(0x01).build();
        let bytes = report.to_bytes();

        assert_eq!(bytes.len(), 65);
        assert_eq!(bytes[0], REPORT_ID);
        assert_eq!(bytes[4], 0x01);
    }
}
