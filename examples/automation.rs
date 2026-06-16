use std::{error::Error, fs::File, io::BufWriter};

use taudio::{
    automation::{
        AutomationClip, AutomationTimeline, AutomationTrack, ControlPoint, ControlPoints,
        CurveMapping, Parameter,
    },
    pipeline::PipelineBuilder,
    sample,
    sinks::SampleSink,
    sources::Osc,
    wav::{WavChunk, WavFile, WavFormat, WavFormatMeta},
    waveform,
};

fn descending_clip() -> AutomationClip {
    let mut points = ControlPoints::new();

    points.add_point(ControlPoint::new(1.0, 0.0));
    points.add_point(ControlPoint::new(0.0, 1.0));
    points.add_point(ControlPoint::new(0.5, 2.0));

    AutomationClip::Controlled(points)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut automations = AutomationTimeline::new();
    let mut freq_track = AutomationTrack::new();

    freq_track.add_clip(descending_clip(), 0.0..2.0);
    let freq_param = automations.register(freq_track);

    let osc = Osc::new(
        waveform::Sine,
        Parameter::Automated {
            id: freq_param,
            mapping: CurveMapping::Exp(220.0, 440.0),
        },
        Parameter::Constant(1.0),
        1,
    );

    let mut builder = PipelineBuilder::default();

    let osc_id = builder.add_source(osc)?;
    let sink_id = builder.add_sink([osc_id.output(0)], SampleSink::new(sample::Int16))?;

    let mut pipeline = builder.build()?;
    pipeline.sample(44100, &automations)?;
    pipeline.sample(44100, &automations)?;

    let sink = pipeline
        .get_sink_mut(sink_id)
        .and_then(|x| x.downcast_mut::<SampleSink<sample::Int16>>());

    if let Some(data) = sink.map(|s| s.take_channel(0)) {
        let meta = WavFormatMeta {
            sample_rate: 44100,
            audio_format: WavFormat::Pcm,
            num_channels: 1,
            bits_per_sample: 16,
        };

        let wav = WavFile::from_chunks([WavChunk::new_format(&meta), WavChunk::new_data(data)]);

        let file = File::create("sine_wobbly.wav")?;
        let mut writer = BufWriter::new(file);

        wav.write(&mut writer)?;
    }

    Ok(())
}
