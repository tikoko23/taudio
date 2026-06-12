use smallvec::{SmallVec, smallvec};

use crate::{
    Real,
    buffer::SampleChannels,
    err::AudioError,
    node::{AudioNode, AudioSink, AudioSinkCfg, AudioSinkInfo, SamplingContext},
};

#[derive(Debug, Clone)]
pub struct BufferedSink {
    buffers: SmallVec<[Vec<Real>; 4]>,
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
            buffers: smallvec![],
        }
    }

    /// Visits each buffer, calls the callback with the stored samples and clears the buffer after.
    pub fn visit<F>(&mut self, mut cb: F)
    where
        F: FnMut(usize, &[Real]),
    {
        for (i, buf) in self.buffers.iter_mut().enumerate() {
            cb(i, buf);

            buf.clear();
        }
    }

    /// Returns the number of channels.
    #[inline]
    pub fn num_channels(&self) -> usize {
        self.buffers.len()
    }

    /// Returns an iterator over the stored samples.
    ///
    /// This function will move the buffers into the iterator. Reuse of the [`BufferedSink`] will
    /// cause new allocations. See [`BufferedSink::visit`] for a non-owning, no-alloc alternative
    /// if you allocate your own buffers.
    pub fn take(&mut self) -> impl Iterator<Item = Vec<Real>> {
        self.buffers.iter_mut().map(std::mem::take)
    }

    /// Visits the specified channel, calls the callback with the stored samples and clears the
    /// buffer.
    ///
    /// # Panics
    /// This function will panic if the given channel index is out of bounds.
    pub fn visit_channel<F>(&mut self, channel: usize, mut cb: F)
    where
        F: FnMut(&[Real]),
    {
        cb(&self.buffers[channel]);
    }

    /// Returns the stored samples of the channel with the given index.
    ///
    /// # Panics
    /// This function will panic if the given channel index is out of bounds.
    pub fn get_channel(&self, channel: usize) -> &[Real] {
        &self.buffers[channel]
    }

    /// Returns the stored samples of the channel with the given index.
    ///
    /// # Panics
    /// This function will panic if the given channel index is out of bounds.
    pub fn get_channel_mut(&mut self, channel: usize) -> &mut [Real] {
        &mut self.buffers[channel]
    }

    /// Returns the stored samples of the channel with the given index.
    ///
    /// # Panics
    /// This function will panic if the given channel index is out of bounds.
    pub fn try_get_channel(&self, channel: usize) -> Option<&[Real]> {
        self.buffers.get(channel).map(|c| c.as_slice())
    }

    /// Returns the stored samples of the channel with the given index.
    pub fn try_get_channel_mut(&mut self, channel: usize) -> Option<&mut [Real]> {
        self.buffers.get_mut(channel).map(|c| c.as_mut_slice())
    }

    /// Returns the stored samples of the channel with the given index and clears the inner buffer.
    ///
    /// This function will move the buffer into the return value. Reuse of the [`BufferedSink`] will
    /// cause new allocations. See [`BufferedSink::get_channel`] for a non-owning, no-alloc alternative
    /// if you allocate your own buffers.
    pub fn take_channel(&mut self, channel: usize) -> Vec<Real> {
        std::mem::take(&mut self.buffers[channel])
    }
}

impl AudioNode for BufferedSink {
    fn name(&self) -> &str {
        "@builtin:buffered-sink"
    }
}

impl AudioSink for BufferedSink {
    fn setup(&mut self, cfg: &AudioSinkCfg) -> Result<AudioSinkInfo, AudioError> {
        self.buffers.clear();

        for _ in 0..cfg.num_inputs {
            self.buffers
                .push(Vec::with_capacity(8 * cfg.sample_rate as usize));
        }

        Ok(AudioSinkInfo {})
    }

    fn sample(
        &mut self,
        ctx: &SamplingContext,
        input: &SampleChannels<'_>,
    ) -> Result<(), AudioError> {
        let _ = ctx;

        for (i, buf) in self.buffers.iter_mut().enumerate() {
            let chan = input.get_channel(i);
            buf.extend_from_slice(&chan);
        }

        Ok(())
    }
}
