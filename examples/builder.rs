use taudio::{
    buffer::SampleChannels,
    err::AudioError,
    node::{
        AudioNode, AudioProcessor, AudioProcessorCfg, AudioProcessorInfo, AudioSink, AudioSinkCfg,
        AudioSinkInfo, AudioSource, AudioSourceCfg, AudioSourceInfo, SamplingContext,
    },
    pipeline::{PipelineBuilder, PipelineOpts},
};

#[derive(Debug, Clone)]
struct TestSource(usize);

#[derive(Debug, Clone)]
struct TestProcessor(usize, usize);

#[derive(Debug, Clone)]
struct TestSink(usize);

impl AudioNode for TestSource {
    fn name(&self) -> &str {
        "@test:source"
    }
}

impl AudioNode for TestProcessor {
    fn name(&self) -> &str {
        "@test:processor"
    }
}

impl AudioNode for TestSink {
    fn name(&self) -> &str {
        "@test:sink"
    }
}

impl AudioSource for TestSource {
    fn setup(&mut self, cfg: &AudioSourceCfg) -> Result<AudioSourceInfo, AudioError> {
        let _ = cfg;

        Ok(AudioSourceInfo {
            num_outputs: self.0,
        })
    }

    fn sample(
        &mut self,
        _ctx: &SamplingContext,
        _output: &mut SampleChannels<'_>,
    ) -> Result<(), AudioError> {
        Ok(())
    }
}

impl AudioProcessor for TestProcessor {
    fn setup(&mut self, cfg: &AudioProcessorCfg) -> Result<AudioProcessorInfo, AudioError> {
        AudioError::expect_channels(self.0..=self.0, cfg.num_inputs)?;

        Ok(AudioProcessorInfo {
            num_outputs: self.1,
        })
    }

    fn sample(
        &mut self,
        _ctx: &SamplingContext,
        _input: &SampleChannels<'_>,
        _output: &mut SampleChannels<'_>,
    ) -> Result<(), AudioError> {
        Ok(())
    }
}

impl AudioSink for TestSink {
    fn setup(&mut self, cfg: &AudioSinkCfg) -> Result<AudioSinkInfo, AudioError> {
        AudioError::expect_channels(self.0..=self.0, cfg.num_inputs)?;

        Ok(AudioSinkInfo {})
    }

    fn sample(
        &mut self,
        _ctx: &SamplingContext,
        _input: &SampleChannels<'_>,
    ) -> Result<(), AudioError> {
        Ok(())
    }
}

fn main() -> Result<(), AudioError> {
    let opts = PipelineOpts::default();
    let mut builder = PipelineBuilder::new(opts);

    let source = builder.add_source(TestSource(2))?;
    let proc1 = builder.add_processor([source.output(0)], TestProcessor(1, 1))?;
    let proc2 = builder.add_processor([source.output(1)], TestProcessor(1, 1))?;
    let proc3 = builder.add_processor([proc1.output(0), proc2.output(0)], TestProcessor(2, 1))?;
    let _ = builder.add_sink([proc3.output(0)], TestSink(1))?;

    let template = builder.into_template()?;

    println!("{template:#?}");

    Ok(())
}
