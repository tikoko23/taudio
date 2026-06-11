use crate::Real;
use std::io::{self, Write};

#[derive(Debug)]
pub struct Int8;

#[derive(Debug)]
pub struct Int16;

#[derive(Debug)]
pub struct Int32;

#[derive(Debug)]
pub struct Float32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dyn(pub SampleType);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum SampleType {
    Int8,
    Int16,
    Int32,
    Float32,
}

mod sealed {
    use crate::sample::{Dyn, Float32, Int8, Int16, Int32};

    pub trait Sealed: Sized {}

    impl Sealed for Int8 {}
    impl Sealed for Int16 {}
    impl Sealed for Int32 {}
    impl Sealed for Float32 {}
    impl Sealed for Dyn {}
}

pub trait Sample: sealed::Sealed {
    fn write(&self, sample: Real, w: &mut dyn Write) -> io::Result<()>;
}

impl Sample for Int8 {
    fn write(&self, sample: Real, w: &mut dyn Write) -> io::Result<()> {
        let s = (sample * 127.0) as i8;

        w.write_all(&s.to_le_bytes())
    }
}

impl Sample for Int16 {
    fn write(&self, sample: Real, w: &mut dyn Write) -> io::Result<()> {
        let s = (sample * 32767.0) as i16;

        w.write_all(&s.to_le_bytes())
    }
}

impl Sample for Int32 {
    fn write(&self, sample: Real, w: &mut dyn Write) -> io::Result<()> {
        let s = (sample * 2147483647.0) as i32;

        w.write_all(&s.to_le_bytes())
    }
}

impl Sample for Float32 {
    fn write(&self, sample: Real, w: &mut dyn Write) -> io::Result<()> {
        let s = sample as f32;

        w.write_all(&s.to_le_bytes())
    }
}

impl Sample for Dyn {
    fn write(&self, sample: Real, w: &mut dyn Write) -> io::Result<()> {
        match self.0 {
            SampleType::Int8 => Int8::write(&Int8, sample, w),
            SampleType::Int16 => Int16::write(&Int16, sample, w),
            SampleType::Int32 => Int32::write(&Int32, sample, w),
            SampleType::Float32 => Float32::write(&Float32, sample, w),
        }
    }
}
