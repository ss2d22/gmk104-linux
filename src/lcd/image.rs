use anyhow::{Context, Result};
use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, DynamicImage, GenericImageView, ImageReader, RgbaImage};
use log::{debug, info};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use super::{BYTES_PER_PIXEL, DEFAULT_HEIGHT, DEFAULT_WIDTH};

#[derive(Debug, Clone)]
pub struct Animation {
    pub frames: Vec<LcdImage>,
    pub delay_ms: u16,
}

impl Animation {
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn is_static(&self) -> bool {
        self.frames.len() <= 1
    }
}

#[derive(Debug, Clone)]
pub struct LcdImage {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl LcdImage {
    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }

    pub fn expected_size(&self) -> usize {
        self.pixel_count() * BYTES_PER_PIXEL
    }
}

#[derive(Debug, Clone)]
pub struct ImageProcessor {
    pub target_width: u32,
    pub target_height: u32,
    pub resize: bool,
}

impl ImageProcessor {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            target_width: width,
            target_height: height,
            resize: true,
        }
    }

    pub fn with_resize(mut self, resize: bool) -> Self {
        self.resize = resize;
        self
    }

    pub fn load<P: AsRef<Path>>(&self, path: P) -> Result<LcdImage> {
        let path = path.as_ref();
        info!("Loading image: {}", path.display());

        let img = ImageReader::open(path)
            .with_context(|| format!("Failed to open image: {}", path.display()))?
            .decode()
            .with_context(|| format!("Failed to decode image: {}", path.display()))?;

        self.process(img)
    }

    pub fn load_animation<P: AsRef<Path>>(&self, path: P) -> Result<Animation> {
        let path = path.as_ref();
        info!("Loading animation: {}", path.display());

        let is_gif = path
            .extension()
            .map(|ext| ext.to_ascii_lowercase() == "gif")
            .unwrap_or(false);

        if is_gif {
            self.load_gif(path)
        } else {
            let frame = self.load(path)?;
            Ok(Animation {
                frames: vec![frame],
                delay_ms: 100,
            })
        }
    }

    fn load_gif<P: AsRef<Path>>(&self, path: P) -> Result<Animation> {
        let path = path.as_ref();
        let file = File::open(path)
            .with_context(|| format!("Failed to open GIF: {}", path.display()))?;
        let reader = BufReader::new(file);

        let decoder = GifDecoder::new(reader)
            .with_context(|| format!("Failed to decode GIF: {}", path.display()))?;

        let raw_frames = decoder.into_frames().collect_frames()
            .with_context(|| format!("Failed to collect GIF frames: {}", path.display()))?;

        if raw_frames.is_empty() {
            anyhow::bail!("GIF has no frames");
        }

        info!("GIF has {} frames", raw_frames.len());

        let first_delay = raw_frames[0].delay().numer_denom_ms();
        let delay_ms = (first_delay.0 / first_delay.1).max(60) as u16;
        debug!("Frame delay: {}ms", delay_ms);

        let mut frames = Vec::with_capacity(raw_frames.len());
        for (i, raw_frame) in raw_frames.into_iter().enumerate() {
            let rgba: RgbaImage = raw_frame.buffer().clone();
            let dynamic = DynamicImage::ImageRgba8(rgba);
            let lcd_image = self.process(dynamic)
                .with_context(|| format!("Failed to process frame {}", i))?;
            frames.push(lcd_image);
            debug!("Processed frame {}", i);
        }

        Ok(Animation { frames, delay_ms })
    }

    pub fn process(&self, img: DynamicImage) -> Result<LcdImage> {
        let (orig_w, orig_h) = img.dimensions();
        debug!("Original dimensions: {}x{}", orig_w, orig_h);

        let img = if self.resize && (orig_w != self.target_width || orig_h != self.target_height) {
            info!(
                "Resizing from {}x{} to {}x{}",
                orig_w, orig_h, self.target_width, self.target_height
            );
            img.resize_exact(
                self.target_width,
                self.target_height,
                image::imageops::FilterType::Lanczos3,
            )
        } else {
            img
        };

        let rgb = img.to_rgb8();
        let (width, height) = rgb.dimensions();

        let expected_size = (width * height) as usize * BYTES_PER_PIXEL;
        let mut data = Vec::with_capacity(expected_size);

        for y in 0..height {
            for x in 0..width {
                let pixel = rgb.get_pixel(x, y);
                let rgb565 = rgb_to_rgb565(pixel[0], pixel[1], pixel[2]);
                data.push((rgb565 & 0xFF) as u8);
                data.push((rgb565 >> 8) as u8);
            }
        }

        debug!(
            "Converted to RGB565: {} bytes ({} pixels)",
            data.len(),
            width * height
        );

        Ok(LcdImage {
            data,
            width,
            height,
        })
    }

    pub fn solid_color(&self, r: u8, g: u8, b: u8) -> LcdImage {
        let pixel_count = (self.target_width * self.target_height) as usize;
        let rgb565 = rgb_to_rgb565(r, g, b);
        let low = (rgb565 & 0xFF) as u8;
        let high = (rgb565 >> 8) as u8;

        let mut data = Vec::with_capacity(pixel_count * BYTES_PER_PIXEL);
        for _ in 0..pixel_count {
            data.push(low);
            data.push(high);
        }

        LcdImage {
            data,
            width: self.target_width,
            height: self.target_height,
        }
    }

    pub fn test_pattern(&self) -> LcdImage {
        let width = self.target_width;
        let height = self.target_height;
        let mut data = Vec::with_capacity((width * height) as usize * BYTES_PER_PIXEL);

        let cell_w = width / 8;
        let cell_h = height / 4;

        for y in 0..height {
            for x in 0..width {
                let cell_x = x / cell_w;
                let cell_y = y / cell_h;

                let is_light = (cell_x + cell_y) % 2 == 0;
                let is_border = x % cell_w == 0 || y % cell_h == 0;

                let rgb565 = if is_border {
                    rgb_to_rgb565(255, 255, 255)
                } else if is_light {
                    let intensity = ((cell_x * 32) % 256) as u8;
                    rgb_to_rgb565(intensity, 64, 64)
                } else {
                    let intensity = ((cell_y * 64) % 256) as u8;
                    rgb_to_rgb565(64, 64, intensity)
                };

                data.push((rgb565 & 0xFF) as u8);
                data.push((rgb565 >> 8) as u8);
            }
        }

        LcdImage {
            data,
            width,
            height,
        }
    }
}

impl Default for ImageProcessor {
    fn default() -> Self {
        Self::new(DEFAULT_WIDTH, DEFAULT_HEIGHT)
    }
}

#[inline]
pub fn rgb_to_rgb565(r: u8, g: u8, b: u8) -> u16 {
    let r5 = (r as u16 >> 3) & 0x1F;
    let g6 = (g as u16 >> 2) & 0x3F;
    let b5 = (b as u16 >> 3) & 0x1F;
    (r5 << 11) | (g6 << 5) | b5
}

#[inline]
pub fn rgb565_to_rgb(rgb565: u16) -> (u8, u8, u8) {
    let r5 = ((rgb565 >> 11) & 0x1F) as u8;
    let g6 = ((rgb565 >> 5) & 0x3F) as u8;
    let b5 = (rgb565 & 0x1F) as u8;

    let r = (r5 << 3) | (r5 >> 2);
    let g = (g6 << 2) | (g6 >> 4);
    let b = (b5 << 3) | (b5 >> 2);

    (r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb565_conversion() {
        assert_eq!(rgb_to_rgb565(0xFF, 0x00, 0x00), 0xF800);
        assert_eq!(rgb_to_rgb565(0x00, 0xFF, 0x00), 0x07E0);
        assert_eq!(rgb_to_rgb565(0x00, 0x00, 0xFF), 0x001F);
        assert_eq!(rgb_to_rgb565(0xFF, 0xFF, 0xFF), 0xFFFF);
        assert_eq!(rgb_to_rgb565(0x00, 0x00, 0x00), 0x0000);
    }

    #[test]
    fn test_rgb565_roundtrip() {
        let colors = [
            (255, 0, 0),
            (0, 255, 0),
            (0, 0, 255),
            (255, 255, 255),
            (0, 0, 0),
            (128, 128, 128),
        ];

        for (r, g, b) in colors {
            let rgb565 = rgb_to_rgb565(r, g, b);
            let (r2, g2, b2) = rgb565_to_rgb(rgb565);
            assert!((r as i16 - r2 as i16).abs() <= 8, "Red mismatch for {:?}", (r, g, b));
            assert!((g as i16 - g2 as i16).abs() <= 4, "Green mismatch for {:?}", (r, g, b));
            assert!((b as i16 - b2 as i16).abs() <= 8, "Blue mismatch for {:?}", (r, g, b));
        }
    }

    #[test]
    fn test_solid_color() {
        let processor = ImageProcessor::new(10, 10);
        let img = processor.solid_color(255, 0, 0);

        assert_eq!(img.width, 10);
        assert_eq!(img.height, 10);
        assert_eq!(img.size(), 200);

        let first_pixel = (img.data[0] as u16) | ((img.data[1] as u16) << 8);
        assert_eq!(first_pixel, 0xF800);
    }
}
