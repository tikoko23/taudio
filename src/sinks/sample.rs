use smallvec::{SmallVec, smallvec};

use crate::{
    buffer::SampleChannels,
    err::AudioError,
    node::{AudioNode, AudioSink, AudioSinkCfg, AudioSinkInfo, SamplingContext},
    sample::Sample,
};

/// Buffers the audio data in memory as encoded
#[derive(Debug, Clone)]
pub struct SampleSink<S: Sample> {
    buffers: SmallVec<[Vec<u8>; 4]>,
    sample: S,
}

impl<S: Sample> SampleSink<S> {
    pub fn new(sample: S) -> Self {
        Self {
            buffers: smallvec![],
            sample,
        }
    }

    /// Visits each buffer, calls the callback with the stored samples and clears the buffer after.
    pub fn visit<F>(&mut self, mut cb: F)
    where
        F: FnMut(usize, &[u8]),
    {
        for (i, buf) in self.buffers.iter_mut().enumerate() {
            cb(i, buf);

            buf.clear();
        }
    }

    /// Returns an iterator over the stored samples.
    ///
    /// This function will move the buffers into the iterator. Reuse of the [`SampleSink`] will
    /// cause new allocations. See [`SampleSink::visit`] for a non-owning, no-alloc alternative
    /// if you allocate your own buffers.
    pub fn take(&mut self) -> impl Iterator<Item = Vec<u8>> {
        self.buffers.iter_mut().map(std::mem::take)
    }
}

impl<S: Sample> AudioNode for SampleSink<S> {
    fn name(&self) -> &str {
        "@builtin:sample-sink"
    }
}

impl<S: Sample> AudioSink for SampleSink<S> {
    fn setup(&mut self, cfg: &AudioSinkCfg) -> Result<AudioSinkInfo, AudioError> {
        self.buffers.clear();

        for _ in 0..cfg.num_inputs {
            self.buffers
                .push(Vec::with_capacity(cfg.sample_rate as usize * 8));
        }

        Ok(AudioSinkInfo {})
    }

    fn sample(
        &mut self,
        ctx: &SamplingContext,
        input: &SampleChannels<'_>,
    ) -> Result<(), AudioError> {
        let _ = ctx;

        for i in 0..self.buffers.len() {
            let chan = input.get_channel(i);
            let buf = &mut self.buffers[i];

            for &sample in chan.iter() {
                // Appending to vec is infallible
                let _ = self.sample.write(sample, buf);
            }
        }

        Ok(())
    }
}
