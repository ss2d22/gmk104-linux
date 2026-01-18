pub mod frames;
pub mod image;

pub use frames::{FrameChunker, ImageFrame};
pub use image::{Animation, ImageProcessor, LcdImage};

pub const DEFAULT_WIDTH: u32 = 240;
pub const DEFAULT_HEIGHT: u32 = 135;
pub const BYTES_PER_PIXEL: usize = 2;
pub const BYTES_PER_FRAME: usize = 0x38;
