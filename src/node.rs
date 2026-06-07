use std::fmt::Debug;

use crate::{Real, buffer::SampleChannels, err::AudioError};

macro_rules! boxed_dupe {
    ($($vis:vis trait $TraitName:ident for<dyn $DynName:path> { ... })*) => {
        $(
            $vis trait $TraitName {
                fn boxed_dupe(&self) -> ::std::option::Option<::std::boxed::Box<dyn $DynName>>;
            }

            impl<T: 'static + $DynName + $crate::dupe::Dupe> $TraitName for T {
                fn boxed_dupe(&self) -> ::std::option::Option<::std::boxed::Box<dyn $DynName>> {
                    $crate::dupe::Dupe::dupe(self).map(|x| ::std::boxed::Box::new(x) as ::std::boxed::Box<dyn $DynName>)
                }
            }

            impl $crate::dupe::Dupe for ::std::boxed::Box<dyn $DynName> {
                #[inline]
                fn dupe(&self) -> ::std::option::Option<Self> {
                    $TraitName::boxed_dupe(self.as_ref())
                }
            }
        )*
    };
}

boxed_dupe! {
    pub trait AudioSourceDupe for<dyn AudioSource> { ... }
    pub trait AudioSinkDupe for<dyn AudioSink> { ... }
    pub trait AudioProcessorDupe for<dyn AudioProcessor> { ... }
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

pub trait AudioNode: 'static + Debug {
    fn name(&self) -> &str {
        "<unnamed node>"
    }
}

pub trait AudioSource: AudioSourceDupe + AudioNode {
    fn setup(&mut self, cfg: &AudioSourceCfg) -> Result<AudioSourceInfo, AudioError>;

    fn sample(
        &mut self,
        ctx: &SamplingContext,
        output: &mut SampleChannels<'_>,
    ) -> Result<(), AudioError>;
}

pub trait AudioSink: AudioSinkDupe + AudioNode {
    fn setup(&mut self, cfg: &AudioSinkCfg) -> Result<AudioSinkInfo, AudioError>;

    fn sample(
        &mut self,
        ctx: &SamplingContext,
        input: &SampleChannels<'_>,
    ) -> Result<(), AudioError>;
}

pub trait AudioProcessor: AudioProcessorDupe + AudioNode {
    fn setup(&mut self, cfg: &AudioProcessorCfg) -> Result<AudioProcessorInfo, AudioError>;

    /// The batch size of the input and the output are *guaranteed* to be the same.
    fn sample(
        &mut self,
        ctx: &SamplingContext,
        input: &SampleChannels<'_>,
        output: &mut SampleChannels<'_>,
    ) -> Result<(), AudioError>;
}
