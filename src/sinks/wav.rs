use std::io::{Seek, Write};

pub mod sample {
    use crate::sinks::SampleType;

    pub struct Int8;
    pub struct Int16;
    pub struct Int32;
    pub struct Float32;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Dyn(pub SampleType);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum SampleType {
    Int8,
    Int16,
    Int32,
    Float32,
}

mod sealed {
    use std::io::{self, Write};

    use crate::{
        Real,
        sinks::{
            SampleType,
            sample::{Dyn, Float32, Int8, Int16, Int32},
        },
    };

    pub trait Sealed: Sized {
        fn write(&self, sample: Real, w: &mut dyn Write) -> io::Result<()>;
    }

    impl Sealed for Int8 {
        fn write(&self, sample: Real, w: &mut dyn Write) -> io::Result<()> {
            let s = (sample * 127.0) as i8;

            w.write_all(&s.to_le_bytes())
        }
    }

    impl Sealed for Int16 {
        fn write(&self, sample: Real, w: &mut dyn Write) -> io::Result<()> {
            let s = (sample * 32767.0) as i16;

            w.write_all(&s.to_le_bytes())
        }
    }

    impl Sealed for Int32 {
        fn write(&self, sample: Real, w: &mut dyn Write) -> io::Result<()> {
            let s = (sample * 2147483647.0) as i32;

            w.write_all(&s.to_le_bytes())
        }
    }

    impl Sealed for Float32 {
        fn write(&self, sample: Real, w: &mut dyn Write) -> io::Result<()> {
            let s = sample as f32;

            w.write_all(&s.to_le_bytes())
        }
    }

    impl Sealed for Dyn {
        fn write(&self, sample: Real, w: &mut dyn Write) -> io::Result<()> {
            match self.0 {
                SampleType::Int8 => Int8::write(&Int8, sample, w),
                SampleType::Int16 => Int16::write(&Int16, sample, w),
                SampleType::Int32 => Int32::write(&Int32, sample, w),
                SampleType::Float32 => Float32::write(&Float32, sample, w),
            }
        }
    }
}

pub trait WavSample: sealed::Sealed {}

impl<T: sealed::Sealed> WavSample for T {}

pub struct WavSink<S: WavSample, W: Write + Seek> {
    sample: S,
    writer: W,
}

impl<S: WavSample, W: Write + Seek> WavSink<S, W> {
    pub fn new(sample: S, writer: W) -> Self {
        Self { sample, writer }
    }
}
