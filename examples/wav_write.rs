use std::{error::Error, fs::File, io::BufWriter};

use taudio::wav::{WavChunk, WavFile, WavFormat, WavFormatMeta};

fn main() -> Result<(), Box<dyn Error>> {
    let samples = include_bytes!("sine440hz.bin");

    let meta = WavFormatMeta {
        audio_format: WavFormat::Pcm,
        bits_per_sample: 16,
        num_channels: 1,
        sample_rate: 44100,
    };

    let fmt_chunk = WavChunk::new_format(&meta);
    let data_chunk = WavChunk::new_data(samples.to_vec());

    let wav = WavFile::from_chunks([fmt_chunk, data_chunk]);

    let file = File::create("examples/sine440hz.wav")?;
    let mut writer = BufWriter::new(file);

    wav.write(&mut writer)?;

    Ok(())
}
