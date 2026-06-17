use std::{error::Error, fs::File, io::BufWriter};

use taudio::{
    automation::AutomationTimeline,
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
    pipeline.sample(44100, &AutomationTimeline::default())?;

    let sink = pipeline.resolve_handle_mut(&sink_id);
    let data = sink.take_channel(0);

    let meta = WavFormatMeta {
        sample_rate: 44100,
        audio_format: WavFormat::Pcm,
        num_channels: 1,
        bits_per_sample: 16,
    };

    let wav = WavFile::from_chunks([WavChunk::new_format(&meta), WavChunk::new_data(data)]);

    let file = File::create("sine440hz.wav")?;
    let mut writer = BufWriter::new(file);

    wav.write(&mut writer)?;

    Ok(())
}
