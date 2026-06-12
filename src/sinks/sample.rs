use std::ops::{Deref, DerefMut};

use crate::{
    buffer::SampleChannels,
    err::AudioError,
    node::{AudioNode, AudioSink, AudioSinkCfg, AudioSinkInfo, SamplingContext},
    sample::Sample,
    sinks::ChannelBuffers,
};

/// Buffers the audio data in memory as encoded
#[derive(Debug, Clone)]
pub struct SampleSink<S: Sample> {
    buffers: ChannelBuffers<u8>,
    sample: S,
}

impl<S: Sample> Deref for SampleSink<S> {
    type Target = ChannelBuffers<u8>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buffers
    }
}

impl<S: Sample> DerefMut for SampleSink<S> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffers
    }
}

impl<S: Sample> SampleSink<S> {
    pub fn new(sample: S) -> Self {
        Self {
            buffers: ChannelBuffers::new(),
            sample,
        }
    }

    #[inline]
    pub fn get_sample(&self) -> &S {
        &self.sample
    }
}

impl<S: Sample> AudioNode for SampleSink<S> {
    fn name(&self) -> &str {
        "@builtin:sample-sink"
    }
}

impl<S: Sample> AudioSink for SampleSink<S> {
    fn setup(&mut self, cfg: &AudioSinkCfg) -> Result<AudioSinkInfo, AudioError> {
        let cap = cfg.sample_rate as usize * 8 * self.sample.size_of();

        self.buffers.create_channels(cfg.num_inputs, cap);

        Ok(AudioSinkInfo {})
    }

    fn sample(
        &mut self,
        ctx: &SamplingContext,
        input: &SampleChannels<'_>,
    ) -> Result<(), AudioError> {
        let _ = ctx;

        for i in 0..self.buffers.num_channels() {
            let chan = input.get_channel(i);
            let buf = self.buffers.get_buffer_mut(i);

            for &sample in chan.iter() {
                // Appending to vec is infallible
                let _ = self.sample.write(sample, buf);
            }
        }

        Ok(())
    }
}
