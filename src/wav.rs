use std::io::{self, Write};

use bytes::Bytes;
use smallvec::SmallVec;

/// The format of samples used by a wave file.
///
/// More formats may be introduced in the future.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u16)]
#[non_exhaustive]
pub enum WavFormat {
    Pcm = 1,
    Float = 3,
}

/// Metadata of a wave file's `fmt ` chunk.
#[derive(Debug, Clone)]
pub struct WavFormatMeta {
    pub audio_format: WavFormat,
    pub num_channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
}

/// A single chunk of a wave file.
#[derive(Debug, Clone)]
pub struct WavChunk {
    pub id: [u8; 4],
    pub data: Bytes,
}

impl WavChunk {
    /// Constructs a new [`WavChunk`] whose id is `fmt ` and data is
    /// derived from the given metadata value.
    pub fn new_format(meta: &WavFormatMeta) -> Self {
        const FORMAT_CHUNK_DATA_SIZE: usize = 16;

        let mut data = Vec::with_capacity(32);

        let byte_rate: u32 =
            meta.sample_rate * meta.num_channels as u32 * meta.bits_per_sample as u32 / 8;

        let block_align: u16 = meta.num_channels * (meta.bits_per_sample / 8);

        data.extend((meta.audio_format as u16).to_le_bytes());
        data.extend(meta.num_channels.to_le_bytes());
        data.extend(meta.sample_rate.to_le_bytes());
        data.extend(byte_rate.to_le_bytes());
        data.extend(block_align.to_le_bytes());
        data.extend(meta.bits_per_sample.to_le_bytes());

        debug_assert_eq!(data.len(), FORMAT_CHUNK_DATA_SIZE);

        Self {
            id: *b"fmt ",
            data: data.into(),
        }
    }

    /// Writes the wave chunk into the provided stream.
    ///
    /// # Panics
    /// This function will panic if the data's length doesn't fit in a [`u32`].
    pub fn write(&self, w: &mut dyn Write) -> io::Result<()> {
        let chunk_size: u32 = self.data.len().try_into().expect("chunk too big");

        w.write_all(&self.id)?;
        w.write_all(&chunk_size.to_le_bytes())?;
        w.write_all(&self.data)?;

        Ok(())
    }
}

/// Represents a wave file split up into its chunks.
#[derive(Debug, Clone)]
pub struct WavFile {
    pub chunks: SmallVec<[WavChunk; 4]>,
}

impl WavFile {
    /// Writes the wave file into the provided stream.
    ///
    /// # Panics
    /// This function will panic if the chunks' total size doesn't fit in a [`u32`].
    pub fn write(&self, w: &mut dyn Write) -> io::Result<()> {
        const CHUNK_META_SIZE: usize = 8;

        let file_size: usize = self
            .chunks
            .iter()
            .map(|chunk| chunk.data.len() + CHUNK_META_SIZE)
            .sum();

        let file_size: u32 = file_size.try_into().expect("file too big");

        w.write_all(b"RIFF")?;
        w.write_all(&file_size.to_le_bytes())?; // This already excludes the header size.
        w.write_all(b"WAVE")?;

        for chunk in &self.chunks {
            chunk.write(w)?;
        }

        Ok(())
    }
}
