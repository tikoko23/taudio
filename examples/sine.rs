use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
};

use taudio::{
    buffer::SampleChannels,
    dupe::Dupe,
    err::AudioError,
    node::{AudioNode, AudioSink, AudioSinkCfg, AudioSinkInfo, SamplingContext},
    pipeline::PipelineBuilder,
    sources::Osc,
    waveform,
};

#[derive(Debug)]
struct FileSink {
    file: Option<BufWriter<File>>,
}

impl Dupe for FileSink {}

impl AudioNode for FileSink {
    fn name(&self) -> &str {
        "@example:file-sink"
    }
}

impl AudioSink for FileSink {
    fn setup(&mut self, cfg: &AudioSinkCfg) -> Result<AudioSinkInfo, AudioError> {
        AudioError::expect_channels(1..=1, cfg.num_inputs)?;

        Ok(AudioSinkInfo {})
    }

    fn sample(
        &mut self,
        ctx: &SamplingContext,
        input: &SampleChannels<'_>,
    ) -> Result<(), AudioError> {
        let _ = ctx;

        let chan = input.get_channel(0);
        let bytes = bytemuck::cast_slice(&chan);

        self.file
            .as_mut()
            .unwrap()
            .write_all(bytes)
            .map_err(AudioError::boxed)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let osc = Osc::new(waveform::Sine, 1.0, 1.0, 1);

    let mut builder = PipelineBuilder::default();

    let osc_id = builder.add_source(osc)?;

    builder.add_sink(
        [osc_id.output(0)],
        FileSink {
            file: Some(BufWriter::new(File::create("dump.bin")?)),
        },
    )?;

    let mut pipeline = builder.build()?;
    pipeline.sample(44100)?;

    Ok(())
}
