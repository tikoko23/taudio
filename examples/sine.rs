use std::{error::Error, fs::File, io::BufWriter};

use smallvec::smallvec;
use taudio::{
    pipeline::PipelineBuilder,
    sample::{self},
    sinks::SampleSink,
    sources::Osc,
    wav::{WavChunk, WavFile, WavFormat, WavFormatMeta},
    waveform,
};

fn main() -> Result<(), Box<dyn Error>> {
    let osc = Osc::new(waveform::Sine, 440.0, 1.0, 1);

    let mut builder = PipelineBuilder::default();

    let osc_id = builder.add_source(osc)?;
    let sink_id = builder.add_sink([osc_id.output(0)], SampleSink::new(sample::Int16))?;

    let mut pipeline = builder.build()?;
    pipeline.sample(44100)?;

    let mut data = None;

    for (id, sink) in pipeline.sinks_mut() {
        if id != sink_id {
            continue;
        }

        if let Some(sink) = sink.downcast_mut::<SampleSink<sample::Int16>>() {
            data = Some(sink.take().next().unwrap());
        }
    }

    if let Some(data) = data {
        let meta = WavFormatMeta {
            sample_rate: 44100,
            audio_format: WavFormat::Pcm,
            num_channels: 1,
            bits_per_sample: 16,
        };

        let data_chunk = WavChunk {
            id: *b"data",
            data: data.into(),
        };

        let wav = WavFile {
            chunks: smallvec![WavChunk::new_format(&meta), data_chunk],
        };

        let file = File::create("sine440hz.wav")?;
        let mut writer = BufWriter::new(file);

        wav.write(&mut writer)?;
    }

    Ok(())
}
