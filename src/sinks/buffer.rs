use std::ops::{Deref, DerefMut};

use crate::{
    Real,
    buffer::SampleChannels,
    err::AudioError,
    node::{AudioNode, AudioSink, AudioSinkCfg, AudioSinkInfo, SamplingContext},
    sinks::ChannelBuffers,
};

#[derive(Debug, Clone)]
pub struct BufferedSink {
    buffers: ChannelBuffers<Real>,
}

impl Deref for BufferedSink {
    type Target = ChannelBuffers<Real>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.buffers
    }
}

impl DerefMut for BufferedSink {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffers
    }
}

impl Default for BufferedSink {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl BufferedSink {
    pub fn new() -> Self {
        Self {
            buffers: ChannelBuffers::new(),
        }
    }
}

impl AudioNode for BufferedSink {
    fn name(&self) -> &str {
        "@builtin:buffered-sink"
    }
}

impl AudioSink for BufferedSink {
    fn setup(&mut self, cfg: &AudioSinkCfg) -> Result<AudioSinkInfo, AudioError> {
        self.buffers
            .create_channels(cfg.num_inputs, 8 * cfg.sample_rate as usize);

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

            self.buffers.feed_channel(i, &chan);
        }

        Ok(())
    }
}
