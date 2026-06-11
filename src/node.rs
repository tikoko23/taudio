use std::{any::Any, fmt::Debug};

use crate::{Real, buffer::SampleChannels, err::AudioError};

macro_rules! boxed_dupe {
    ($($vis:vis trait $TraitName:ident for<dyn $DynName:path> { ... })*) => {
        $(
            $vis trait $TraitName {
                fn boxed_clone(&self) -> ::std::boxed::Box<dyn $DynName>;
            }

            impl<T: 'static + $DynName + ::core::clone::Clone> $TraitName for T {
                fn boxed_clone(&self) -> ::std::boxed::Box<dyn $DynName> {
                   ::std::boxed::Box::new(::core::clone::Clone::clone(self))
                }
            }

            impl ::core::clone::Clone for ::std::boxed::Box<dyn $DynName> {
                #[inline]
                fn clone(&self) -> Self {
                    $TraitName::boxed_clone(self.as_ref())
                }
            }
        )*
    };
}

boxed_dupe! {
    pub trait AudioSourceClone for<dyn AudioSource> { ... }
    pub trait AudioSinkClone for<dyn AudioSink> { ... }
    pub trait AudioProcessorClone for<dyn AudioProcessor> { ... }
}

#[derive(Debug)]
pub struct AudioSourceCfg {
    pub sample_rate: u32,
}

#[derive(Debug, Clone)]
pub struct AudioSourceInfo {
    pub num_outputs: usize,
}

#[derive(Debug)]
pub struct AudioSinkCfg {
    pub num_inputs: usize,
    pub sample_rate: u32,
}

#[derive(Debug, Clone)]
pub struct AudioSinkInfo {}

#[derive(Debug, Clone)]
pub struct AudioProcessorCfg {
    pub sample_rate: u32,
    pub num_inputs: usize,
}

#[derive(Debug, Clone)]
pub struct AudioProcessorInfo {
    pub num_outputs: usize,
}

#[derive(Debug)]
pub struct SamplingContext {
    pub(crate) sample_rate: u32,
    pub(crate) batch_begin: u64,
    pub(crate) num_samples: u32,
}

impl SamplingContext {
    #[inline]
    pub fn get_samples_per_second(&self) -> u32 {
        self.sample_rate
    }

    /// Returns the index of the sample of the batch start.
    #[inline]
    pub fn get_batch_start(&self) -> u64 {
        self.batch_begin
    }

    /// Get the global time offset (in seconds) of the given *local* sample index.
    #[inline]
    pub fn time_of(&self, index: usize) -> Real {
        (self.batch_begin + index as u64) as Real / self.sample_rate as Real
    }

    /// Returns the number of batches in this sample.
    #[inline]
    pub fn batch_size(&self) -> usize {
        self.num_samples as usize
    }
}

pub trait AudioNodeReflection: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: AudioNode> AudioNodeReflection for T {
    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl dyn AudioNode {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any().downcast_ref()
    }

    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut()
    }
}

impl dyn AudioSource {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any().downcast_ref()
    }

    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut()
    }
}

impl dyn AudioProcessor {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any().downcast_ref()
    }

    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut()
    }
}

impl dyn AudioSink {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.as_any().downcast_ref()
    }

    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut()
    }
}

pub trait AudioNode: 'static + AudioNodeReflection + Debug {
    fn name(&self) -> &str {
        "<unnamed node>"
    }
}

pub trait AudioSource: AudioSourceClone + AudioNode {
    fn setup(&mut self, cfg: &AudioSourceCfg) -> Result<AudioSourceInfo, AudioError>;

    fn sample(
        &mut self,
        ctx: &SamplingContext,
        output: &mut SampleChannels<'_>,
    ) -> Result<(), AudioError>;

    #[inline]
    fn finish(&mut self) -> Result<(), AudioError> {
        Ok(())
    }
}

pub trait AudioSink: AudioSinkClone + AudioNode {
    fn setup(&mut self, cfg: &AudioSinkCfg) -> Result<AudioSinkInfo, AudioError>;

    fn sample(
        &mut self,
        ctx: &SamplingContext,
        input: &SampleChannels<'_>,
    ) -> Result<(), AudioError>;

    #[inline]
    fn finish(&mut self) -> Result<(), AudioError> {
        Ok(())
    }
}

pub trait AudioProcessor: AudioProcessorClone + AudioNode {
    fn setup(&mut self, cfg: &AudioProcessorCfg) -> Result<AudioProcessorInfo, AudioError>;

    /// The batch size of the input and the output are *guaranteed* to be the same.
    fn sample(
        &mut self,
        ctx: &SamplingContext,
        input: &SampleChannels<'_>,
        output: &mut SampleChannels<'_>,
    ) -> Result<(), AudioError>;

    #[inline]
    fn finish(&mut self) -> Result<(), AudioError> {
        Ok(())
    }
}
