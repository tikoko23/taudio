use std::{
    borrow::Cow,
    fs::File,
    io::{self, BufWriter, Write},
    ops::{Deref, DerefMut},
    path::Path,
};

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
///
/// This type can represent borrowed and owned binary data.
/// [`WavChunk::try_into_owned`] and [`WavChunk::into_owned`] can be used to create
/// new chukns with static lifetimes, which own their data.
#[derive(Debug, Clone)]
pub struct WavChunk<'a> {
    pub id: [u8; 4],
    pub data: Cow<'a, [u8]>,
}

impl WavChunk<'static> {
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
}

impl<'a> WavChunk<'a> {
    #[inline]
    pub fn new_data(data: impl Into<Cow<'a, [u8]>>) -> Self {
        Self {
            id: *b"data",
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

    /// Converts a borrowed chunk into an owned one.
    ///
    /// If the inner [`Cow`] is [`Cow::Owned`], this is a no-op.
    /// If the inner [`Cow`] is [`Cow::Borrowed`], it's cloned into a [`Cow::Owned`]
    #[inline]
    pub fn into_owned(self) -> WavChunk<'static> {
        WavChunk {
            data: Cow::Owned(self.data.into_owned()),
            id: self.id,
        }
    }

    /// Tries to convert a borrowed chunk into an owned one.
    ///
    /// This will fail if the internal [`Cow`] pointer is [`Cow::Borrowed`].
    /// In the case of failure, the original chunk is returned via the [`Err`] variant.
    pub fn try_into_owned(self) -> Result<WavChunk<'static>, Self> {
        match self.data {
            Cow::Borrowed(_) => Err(self),
            Cow::Owned(data) => Ok(WavChunk::<'static> {
                data: Cow::Owned(data),
                id: self.id,
            }),
        }
    }

    /// Returns whether the chunk owns the data it points to.
    #[inline]
    pub fn is_owned(&self) -> bool {
        matches!(self.data, Cow::Owned(_))
    }

    #[inline]
    pub fn is_fmt(&self) -> bool {
        self.id == *b"fmt "
    }

    #[inline]
    pub fn is_data(&self) -> bool {
        self.id == *b"data"
    }
}

/// Represents a wave file split up into its chunks.
///
/// Chunks may borrow data, as indicated by the lifetime.
/// [`WavFile::try_into_owned`] and [`WavFile::into_owned`] can be used to create
/// new file instances with static lifetimes, which own all their data.
#[derive(Debug, Clone)]
pub struct WavFile<'a> {
    chunks: SmallVec<[WavChunk<'a>; 4]>,
}

impl<'a> Deref for WavFile<'a> {
    type Target = [WavChunk<'a>];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.chunks
    }
}

impl DerefMut for WavFile<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.chunks
    }
}

impl<'a> FromIterator<WavChunk<'a>> for WavFile<'a> {
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = WavChunk<'a>>,
    {
        Self::from_chunks(iter)
    }
}

macro_rules! linear_time_doc_warning {
    (#[$meta:meta] $($tt:tt)*) => {
        #[$meta]
        ///
        /// This operation is _O(n)_ but it shouldn't be a problem since each file
        /// will only have a few chunks at most.
        $($tt)*
    };
}

impl<'a> AsRef<[WavChunk<'a>]> for WavFile<'a> {
    #[inline]
    fn as_ref(&self) -> &[WavChunk<'a>] {
        self.as_chunks()
    }
}

impl<'a> AsMut<[WavChunk<'a>]> for WavFile<'a> {
    #[inline]
    fn as_mut(&mut self) -> &mut [WavChunk<'a>] {
        self.as_chunks_mut()
    }
}

impl<'a> WavFile<'a> {
    #[inline]
    pub fn from_chunks(it: impl IntoIterator<Item = WavChunk<'a>>) -> Self {
        Self {
            chunks: it.into_iter().collect(),
        }
    }

    pub fn into_chunks(self) -> Vec<WavChunk<'a>> {
        self.chunks.into_vec()
    }

    #[inline]
    pub fn as_chunks(&self) -> &[WavChunk<'a>] {
        &self.chunks
    }

    #[inline]
    pub fn as_chunks_mut(&mut self) -> &mut [WavChunk<'a>] {
        &mut self.chunks
    }

    #[inline]
    pub fn push_chunk(&mut self, chunk: WavChunk<'a>) {
        self.chunks.push(chunk);
    }

    #[inline]
    pub fn pop_chunk(&mut self) -> Option<WavChunk<'a>> {
        self.chunks.pop()
    }

    /// # Panics
    /// This function will panic if index is out of bounds.
    #[inline]
    pub fn remove_chunk(&mut self, index: usize) -> WavChunk<'a> {
        self.chunks.remove(index)
    }

    /// # Panics
    /// This function will panic if index is out of bounds.
    #[inline]
    pub fn swap_remove_chunk(&mut self, index: usize) -> WavChunk<'a> {
        self.chunks.swap_remove(index)
    }

    /// Returns an iterator over the chunks of a wave file.
    ///
    /// Note that the mutable variant for this function does not exist because
    /// a [`Cow`] would have to be cloned in order for it to be written to.
    ///
    /// Consider using [`WavFile::as_chunks_mut`] to modify chunk contents.
    pub fn iter_chunks(&self) -> impl Iterator<Item = ([u8; 4], &[u8])> {
        self.chunks.iter().map(|c| (c.id, c.data.as_ref()))
    }

    linear_time_doc_warning! {
        /// Returns a reference to the first format chunk found.
        #[inline]
        pub fn get_fmt_chunk(&self) -> Option<&WavChunk<'a>> {
            self.get_chunk_by_id(*b"fmt ")
        }
    }

    linear_time_doc_warning! {
        /// Returns a reference to the first format chunk found.
        #[inline]
        pub fn get_fmt_chunk_mut(&mut self) -> Option<&mut WavChunk<'a>> {
            self.get_chunk_by_id_mut(*b"fmt ")
        }
    }

    linear_time_doc_warning! {
        /// Returns a reference to the first data chunk found.
        #[inline]
        pub fn get_data_chunk(&self) -> Option<&WavChunk<'a>> {
            self.get_chunk_by_id(*b"data")
        }
    }

    linear_time_doc_warning! {
        /// Returns a reference to the first data chunk found.
        #[inline]
        pub fn get_data_chunk_mut(&mut self) -> Option<&mut WavChunk<'a>> {
            self.get_chunk_by_id_mut(*b"data")
        }
    }

    linear_time_doc_warning! {
        /// Returns a reference to the first chunk with the given id.
        #[inline]
        pub fn get_chunk_by_id(&self, id: [u8; 4]) -> Option<&WavChunk<'a>> {
            self.chunks.iter().find(|c| c.id == id)
        }
    }

    linear_time_doc_warning! {
        /// Returns a reference to the first chunk with the given id.
        #[inline]
        pub fn get_chunk_by_id_mut(&mut self, id: [u8; 4]) -> Option<&mut WavChunk<'a>> {
            self.chunks.iter_mut().find(|c| c.id == id)
        }
    }

    linear_time_doc_warning! {
        /// Returns an iterator over all chunks with the given id.
        #[inline]
        pub fn filter_chunks_by_id(&self, id: [u8; 4]) -> impl Iterator<Item = &WavChunk<'a>> {
            self.chunks.iter().filter(move |c| c.id == id)
        }
    }

    linear_time_doc_warning! {
        /// Returns an iterator over all chunks with the given id.
        #[inline]
        pub fn filter_chunks_by_id_mut(
            &mut self,
            id: [u8; 4],
        ) -> impl Iterator<Item = &mut WavChunk<'a>> {
            self.chunks.iter_mut().filter(move |c| c.id == id)
        }
    }

    /// Writes the wave file into the provided stream.
    ///
    /// # Panics
    /// This function will panic if the chunks' total size doesn't fit in a [`u32`].
    pub fn write(&self, w: &mut dyn Write) -> io::Result<()> {
        const CHUNK_META_SIZE: usize = 8;
        const WAVE_SIGNATURE_SIZE: usize = 4;

        let file_size: usize = self
            .chunks
            .iter()
            .map(|chunk| chunk.data.len() + CHUNK_META_SIZE)
            .chain(std::iter::once(WAVE_SIGNATURE_SIZE))
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

    /// Converts a borrowed wave file into an owned one.
    ///
    /// This is done by calling [`WavChunk::into_owned`] on each chunk.
    /// See its doc comment for details.
    pub fn into_owned(self) -> WavFile<'static> {
        WavFile {
            chunks: self
                .chunks
                .into_iter()
                .map(|chunk| chunk.into_owned())
                .collect(),
        }
    }

    /// Tries to convert a borrowed wave file into an owned one.
    ///
    /// This will fail if any chunk's internal [`Cow`] pointer is [`Cow::Borrowed`].
    /// In the case of failure, the original file is returned via the [`Err`] variant.
    #[allow(clippy::result_large_err)]
    pub fn try_into_owned(self) -> Result<WavFile<'static>, Self> {
        let has_borrow = self.chunks.iter().any(|c| !c.is_owned());

        if has_borrow {
            Err(self)
        } else {
            // This won't clone because we verified it above.
            Ok(self.into_owned())
        }
    }
}

pub trait WavSample: bytemuck::Pod {
    const BITS_PER_SAMPLE: u16;
    const WAV_FORMAT: WavFormat;

    fn write_le<W: Write>(&self, w: &mut W) -> std::io::Result<()>;
}

macro_rules! impl_wav_sample {
    ($(($T:ty, $format:ident)),* $(,)?) => {
        $(
            impl WavSample for $T {
                const BITS_PER_SAMPLE: u16 = std::mem::size_of::<$T>() as u16 * 8;
                const WAV_FORMAT: WavFormat = WavFormat::$format;

                fn write_le<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
                    w.write_all(&self.to_le_bytes())
                }
            }
        )*
    };
}

impl_wav_sample! {
    (i8, Pcm),
    (i16, Pcm),
    (i32, Pcm),
    (f32, Float),
}

/// # Safety
/// Calling this function with an empty slice is a logic error.
/// Calling this function with slices of different lengths is a logic error.
unsafe fn flatten_channels<S: WavSample>(channels: &[&[S]]) -> Vec<u8> {
    let num_samples: usize = channels.len() * channels[0].len();
    let bytes_per_sample = S::BITS_PER_SAMPLE as usize / 8;

    let mut buffer = Vec::with_capacity(num_samples * bytes_per_sample);

    for t in 0..channels[0].len() {
        for chan in channels {
            let sample = chan[t];

            // Writes to Vec are infallible.
            let _ = sample.write_le(&mut buffer);
        }
    }

    buffer
}

/// Dumps samples directly into a wave file.
///
/// For finer control over wave serialization, [`WavFile`] or [`WavChunk`] primitives
/// can be used.
///
/// This function will not allocate any memory for the samples if the host machine
/// is little-endian and there is only one channel (mono audio).
///
/// If you already allocate your own data, consider using [`WavFile::write`] or
/// [`WavChunk::write`] for a no-alloc solution.
///
/// # Panics
/// This function will panic if the channel iterator:
///   - Yields no elements (i.e. is empty).
///   - Yields more than 65535 elements.
///   - Yields slices with different lengths.
///
/// # Example
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// # const path: &str = ".taudio-doctest-sine.wav";
/// #
/// use taudio::{
///     Real,
///     wav,
///     waveform::{self, WaveSource},
/// };
///
/// let mut samples = vec![];
///
/// for i in 0..44100 {
///     let t = (i as Real) / 44100.0;
///
///     samples.push(waveform::Sine.sample(440.0, t) as f32)
/// }
///
/// wav::dump(path, 44100, [samples.as_slice()])?;
/// #
/// # std::fs::remove_file(path)?;
/// # Ok(())
/// # }
/// ```
pub fn dump<'a, S, T, P>(filename: P, sample_rate: u32, channels: T) -> std::io::Result<()>
where
    P: AsRef<Path>,
    S: WavSample,
    T: IntoIterator<Item = &'a [S]>,
{
    let channels: SmallVec<[_; 2]> = channels.into_iter().collect();

    assert!(
        !channels.is_empty(),
        "at least one channel must be provided"
    );

    assert!(
        channels.len() <= 65535,
        "too many channels provided (got {}, max is 65535)",
        channels.len()
    );

    let channel_len = channels[0].len();
    let all_same_length = channels[1..]
        .iter()
        .copied()
        .all(|x| x.len() == channel_len);

    assert!(all_same_length, "channels must have the same length");

    let data = match channels.as_slice() {
        #[cfg(target_endian = "little")]
        &[x] => {
            let bytes = bytemuck::cast_slice(x);
            WavChunk::new_data(bytes)
        }
        xs => {
            let bytes = unsafe { flatten_channels(xs) };
            WavChunk::new_data(bytes)
        }
    };

    let fmt = WavFormatMeta {
        audio_format: S::WAV_FORMAT,
        bits_per_sample: S::BITS_PER_SAMPLE,
        num_channels: channels.len() as u16,
        sample_rate,
    };

    let fmt = WavChunk::new_format(&fmt);
    let wav = WavFile::from_chunks([fmt, data]);

    let file = File::create(filename.as_ref())?;
    let mut writer = BufWriter::new(file);

    wav.write(&mut writer)?;
    writer.flush()?;

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::wav::dump;

    const DUMP_PATH: &str = "/tmp/taudio-test-dump";

    #[test]
    #[should_panic = "at least one"]
    fn dump_panics_on_empty() {
        let _ = dump::<i16, _, _>(DUMP_PATH, 44100, []);
    }

    #[test]
    #[should_panic = "too many"]
    fn dump_panics_on_too_many() {
        let _ = dump::<i16, _, _>(DUMP_PATH, 44100, std::iter::repeat_n([].as_slice(), 65536));
    }

    #[test]
    #[should_panic = "same length"]
    fn dump_panics_on_different_length() {
        let _ = dump::<i16, _, _>(DUMP_PATH, 44100, [[1, 2, 3].as_slice(), [1, 2].as_slice()]);
    }
}
