use log::debug;

use super::{LcdImage, BYTES_PER_FRAME};

#[derive(Debug, Clone)]
pub struct ImageFrame {
    pub sequence: u16,
    pub total: u16,
    pub offset: usize,
    pub data: Vec<u8>,
}

impl ImageFrame {
    pub fn is_last(&self) -> bool {
        self.sequence + 1 == self.total
    }

    pub fn data_size(&self) -> usize {
        self.data.len()
    }
}

pub struct FrameChunker<'a> {
    image: &'a LcdImage,
    current_offset: usize,
    current_sequence: u16,
    total_frames: u16,
    bytes_per_frame: usize,
}

impl<'a> FrameChunker<'a> {
    pub fn new(image: &'a LcdImage) -> Self {
        Self::with_chunk_size(image, BYTES_PER_FRAME)
    }

    pub fn with_chunk_size(image: &'a LcdImage, bytes_per_frame: usize) -> Self {
        let total_bytes = image.data.len();
        let total_frames = ((total_bytes + bytes_per_frame - 1) / bytes_per_frame) as u16;

        debug!(
            "Chunking {} bytes into {} frames ({} bytes/frame)",
            total_bytes, total_frames, bytes_per_frame
        );

        Self {
            image,
            current_offset: 0,
            current_sequence: 0,
            total_frames,
            bytes_per_frame,
        }
    }

    pub fn total(&self) -> u16 {
        self.total_frames
    }

    pub fn remaining(&self) -> u16 {
        self.total_frames - self.current_sequence
    }

    pub fn reset(&mut self) {
        self.current_offset = 0;
        self.current_sequence = 0;
    }
}

impl<'a> Iterator for FrameChunker<'a> {
    type Item = ImageFrame;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_offset >= self.image.data.len() {
            return None;
        }

        let start = self.current_offset;
        let end = std::cmp::min(start + self.bytes_per_frame, self.image.data.len());
        let data = self.image.data[start..end].to_vec();

        let frame = ImageFrame {
            sequence: self.current_sequence,
            total: self.total_frames,
            offset: start,
            data,
        };

        self.current_offset = end;
        self.current_sequence += 1;

        Some(frame)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining() as usize;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for FrameChunker<'a> {
    fn len(&self) -> usize {
        self.remaining() as usize
    }
}

pub fn frames_needed(total_bytes: usize, bytes_per_frame: usize) -> u16 {
    ((total_bytes + bytes_per_frame - 1) / bytes_per_frame) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_image(size: usize) -> LcdImage {
        LcdImage {
            data: (0..size).map(|i| (i % 256) as u8).collect(),
            width: 10,
            height: (size / 20) as u32,
        }
    }

    #[test]
    fn test_chunker_basic() {
        let image = make_test_image(200);
        let chunker = FrameChunker::with_chunk_size(&image, 50);

        assert_eq!(chunker.total(), 4);

        let frames: Vec<_> = chunker.collect();
        assert_eq!(frames.len(), 4);

        assert_eq!(frames[0].sequence, 0);
        assert_eq!(frames[0].offset, 0);
        assert_eq!(frames[0].data.len(), 50);

        assert_eq!(frames[3].sequence, 3);
        assert_eq!(frames[3].offset, 150);
        assert_eq!(frames[3].data.len(), 50);
        assert!(frames[3].is_last());
    }

    #[test]
    fn test_chunker_partial_last_frame() {
        let image = make_test_image(175);
        let chunker = FrameChunker::with_chunk_size(&image, 50);

        assert_eq!(chunker.total(), 4);

        let frames: Vec<_> = chunker.collect();
        assert_eq!(frames.len(), 4);

        assert_eq!(frames[3].data.len(), 25);
        assert!(frames[3].is_last());
    }

    #[test]
    fn test_chunker_exact_fit() {
        let image = make_test_image(100);
        let chunker = FrameChunker::with_chunk_size(&image, 50);

        assert_eq!(chunker.total(), 2);

        let frames: Vec<_> = chunker.collect();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].data.len(), 50);
        assert_eq!(frames[1].data.len(), 50);
    }

    #[test]
    fn test_chunker_single_frame() {
        let image = make_test_image(30);
        let chunker = FrameChunker::with_chunk_size(&image, 50);

        assert_eq!(chunker.total(), 1);

        let frames: Vec<_> = chunker.collect();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].data.len(), 30);
        assert!(frames[0].is_last());
    }

    #[test]
    fn test_frames_needed() {
        assert_eq!(frames_needed(100, 50), 2);
        assert_eq!(frames_needed(101, 50), 3);
        assert_eq!(frames_needed(50, 50), 1);
        assert_eq!(frames_needed(49, 50), 1);
        assert_eq!(frames_needed(0, 50), 0);
    }
}
