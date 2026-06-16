use smallvec::{SmallVec, smallvec};

mod buffer;
mod sample;

pub use buffer::*;
pub use sample::*;

#[derive(Debug, Clone)]
pub struct ChannelBuffers<T: Clone> {
    buffers: SmallVec<[Vec<T>; 4]>,
}

impl<T: Clone> ChannelBuffers<T> {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            buffers: smallvec![],
        }
    }

    pub(crate) fn create_channels(&mut self, num_channels: usize, capacity_per_channel: usize) {
        self.buffers.clear();

        for _ in 0..num_channels {
            let v = Vec::with_capacity(capacity_per_channel);
            self.buffers.push(v);
        }
    }

    /// Feed data to a channel.
    #[inline]
    pub(crate) fn feed_channel(&mut self, channel: usize, data: &[T]) {
        self.buffers[channel].extend_from_slice(data);
    }

    #[inline]
    pub(crate) fn get_buffer_mut(&mut self, channel: usize) -> &mut Vec<T> {
        &mut self.buffers[channel]
    }

    /// Visits each buffer, calls the callback with the stored samples and clears the buffer after.
    pub fn visit<F>(&mut self, mut cb: F)
    where
        F: FnMut(usize, &[T]),
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
    /// This function will move the buffers into the iterator. Reuse of the [`ChannelBuffers`] will
    /// cause new allocations. See [`ChannelBuffers::visit`] for a non-owning, no-alloc alternative
    /// if you allocate your own buffers.
    pub fn take(&mut self) -> impl Iterator<Item = Vec<T>> {
        self.buffers.iter_mut().map(std::mem::take)
    }

    /// Visits the specified channel, calls the callback with the stored samples and clears the
    /// buffer.
    ///
    /// # Panics
    /// This function will panic if the given channel index is out of bounds.
    pub fn visit_channel<F>(&mut self, channel: usize, mut cb: F)
    where
        F: FnMut(&[T]),
    {
        cb(&self.buffers[channel]);
    }

    /// Returns the stored samples of the channel with the given index.
    ///
    /// # Panics
    /// This function will panic if the given channel index is out of bounds.
    pub fn get_channel(&self, channel: usize) -> &[T] {
        &self.buffers[channel]
    }

    /// Returns the stored samples of the channel with the given index.
    ///
    /// # Panics
    /// This function will panic if the given channel index is out of bounds.
    pub fn get_channel_mut(&mut self, channel: usize) -> &mut [T] {
        &mut self.buffers[channel]
    }

    /// Returns the stored samples of the channel with the given index.
    ///
    /// # Panics
    /// This function will panic if the given channel index is out of bounds.
    pub fn try_get_channel(&self, channel: usize) -> Option<&[T]> {
        self.buffers.get(channel).map(|c| c.as_slice())
    }

    /// Returns the stored samples of the channel with the given index.
    pub fn try_get_channel_mut(&mut self, channel: usize) -> Option<&mut [T]> {
        self.buffers.get_mut(channel).map(|c| c.as_mut_slice())
    }

    /// Returns the stored samples of the channel with the given index and clears the inner buffer.
    ///
    /// This function will move the buffer into the return value. Reuse of the [`ChannelBuffers`] will
    /// cause new allocations. See [`ChannelBuffers::get_channel`] for a non-owning, no-alloc alternative
    /// if you allocate your own buffers.
    pub fn take_channel(&mut self, channel: usize) -> Vec<T> {
        std::mem::take(&mut self.buffers[channel])
    }
}
