use std::{
    cell::{Ref, RefCell, RefMut},
    ops::{Deref, DerefMut},
};

use smallvec::SmallVec;

use crate::{Real, id::IdContainer, pipeline::BufferId};

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct AudioBuffer(Box<[Real]>);

impl AudioBuffer {
    /// Allocates a zero initialized buffer.
    pub fn new(sample_count: usize) -> Self {
        Self(vec![0.0; sample_count].into_boxed_slice())
    }

    /// Allocates a buffer whose sample countents are uninitialized.
    ///
    /// # Safety
    /// The caller must handle uninitialized data properly.
    pub unsafe fn new_uninit(sample_count: usize) -> Self {
        let buf = Box::new_uninit_slice(sample_count);
        let samples = unsafe { buf.assume_init() };

        Self(samples)
    }

    #[inline]
    pub fn as_slice(&self) -> &[Real] {
        &self.0
    }

    #[inline]
    pub fn as_slice_mut(&mut self) -> &mut [Real] {
        &mut self.0
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.len()
    }
}

impl AsRef<[Real]> for AudioBuffer {
    fn as_ref(&self) -> &[Real] {
        &self.0
    }
}

impl AsMut<[Real]> for AudioBuffer {
    fn as_mut(&mut self) -> &mut [Real] {
        &mut self.0
    }
}

impl Deref for AudioBuffer {
    type Target = [Real];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AudioBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Box<[Real]>> for AudioBuffer {
    #[inline]
    fn from(value: Box<[Real]>) -> Self {
        Self(value)
    }
}

impl From<AudioBuffer> for Box<[Real]> {
    #[inline]
    fn from(value: AudioBuffer) -> Self {
        value.0
    }
}

#[derive(Debug)]
pub struct SampleChannels<'a> {
    pub(crate) num_samples: usize,
    pub(crate) buffers: &'a IdContainer<Vec<RefCell<AudioBuffer>>>,
    pub(crate) channels: &'a SmallVec<[BufferId; 16]>,
}

impl<'a> SampleChannels<'a> {
    #[inline]
    pub fn get_channel(&self, index: usize) -> Ref<'_, [Real]> {
        Ref::map(self.buffers[self.channels[index]].borrow(), |b| {
            &b[0..self.num_samples]
        })
    }

    #[inline]
    pub fn get_channel_mut(&mut self, index: usize) -> RefMut<'_, [Real]> {
        RefMut::map(self.buffers[self.channels[index]].borrow_mut(), |b| {
            &mut b[0..self.num_samples]
        })
    }
}
