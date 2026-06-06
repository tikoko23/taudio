use std::fmt::Debug;

use crate::{buffer::ChannelSlice, err::AudioError};

macro_rules! boxed_clone {
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

#[derive(Debug, Clone)]
pub enum AudioNode {
    Source(Box<dyn AudioSource>),
    Sink(Box<dyn AudioSink>),
    Processor(Box<dyn AudioProcessor>),
}

impl AudioNode {
    pub fn as_common(&self) -> &dyn AudioNodeCommon {
        match self {
            AudioNode::Source(s) => s.as_ref(),
            AudioNode::Processor(p) => p.as_ref(),
            AudioNode::Sink(s) => s.as_ref(),
        }
    }

    pub fn as_common_mut(&mut self) -> &mut dyn AudioNodeCommon {
        match self {
            AudioNode::Source(s) => s.as_mut(),
            AudioNode::Processor(p) => p.as_mut(),
            AudioNode::Sink(s) => s.as_mut(),
        }
    }

    pub fn as_source(&self) -> Option<&dyn AudioSource> {
        match self {
            Self::Sink(_) => None,
            Self::Source(s) => Some(s.as_ref()),
            Self::Processor(_) => None,
        }
    }

    pub fn as_sink(&self) -> Option<&dyn AudioSink> {
        match self {
            Self::Sink(s) => Some(s.as_ref()),
            Self::Source(_) => None,
            Self::Processor(_) => None,
        }
    }

    pub fn as_source_mut(&mut self) -> Option<&mut dyn AudioSource> {
        match self {
            Self::Sink(_) => None,
            Self::Source(s) => Some(s.as_mut()),
            Self::Processor(_) => None,
        }
    }

    pub fn as_sink_mut(&mut self) -> Option<&mut dyn AudioSink> {
        match self {
            Self::Sink(s) => Some(s.as_mut()),
            Self::Source(_) => None,
            Self::Processor(_) => None,
        }
    }
}

boxed_clone! {
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
    pub num_outputs: u32,
}

#[derive(Debug)]
pub struct AudioSinkCfg {
    pub num_inputs: u32,
    pub sample_rate: u32,
}

#[derive(Debug, Clone)]
pub struct AudioSinkInfo {}

#[derive(Debug, Clone)]
pub struct AudioProcessorCfg {
    pub sample_rate: u32,
    pub num_inputs: u32,
}

#[derive(Debug, Clone)]
pub struct AudioProcessorInfo {
    pub num_outputs: u32,
}

pub trait AudioNodeCommon: 'static + Debug {
    fn name(&self) -> &str {
        "<unnamed node>"
    }
}

pub trait AudioSource: AudioSourceClone + AudioNodeCommon {
    fn setup(&mut self, cfg: &AudioSourceCfg) -> Result<AudioSourceInfo, AudioError>;

    fn sample(&mut self, output: &mut ChannelSlice<'_>) -> Result<(), AudioError>;
}

pub trait AudioSink: AudioSinkClone + AudioNodeCommon {
    fn setup(&mut self, cfg: &AudioSinkCfg) -> Result<AudioSinkInfo, AudioError>;
    fn sample(&mut self, input: &ChannelSlice<'_>) -> Result<(), AudioError>;
}

pub trait AudioProcessor: AudioProcessorClone + AudioNodeCommon {
    fn setup(&mut self, cfg: &AudioProcessorCfg) -> Result<AudioProcessorInfo, AudioError>;

    fn sample(
        &mut self,
        input: &ChannelSlice<'_>,
        output: &mut ChannelSlice<'_>,
    ) -> Result<(), AudioError>;
}
