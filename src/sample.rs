use crate::Real;
use std::io::{self, Write};

/// Little-endian, 8 bit signed integer sample.
#[derive(Debug, Clone, Copy)]
pub struct Int8;

/// Little-endian, 16 bit signed integer sample.
#[derive(Debug, Clone, Copy)]
pub struct Int16;

/// Little-endian, 32 bit signed integer sample.
#[derive(Debug, Clone, Copy)]
pub struct Int32;

/// Little-endian, 32 bit floating point sample.
#[derive(Debug, Clone, Copy)]
pub struct Float32;

/// Runtime determined sample.
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
    use std::fmt::Debug;

    use crate::sample::{Dyn, Float32, Int8, Int16, Int32};

    pub trait Sealed: Sized + Clone + Debug + 'static {}

    impl Sealed for Int8 {}
    impl Sealed for Int16 {}
    impl Sealed for Int32 {}
    impl Sealed for Float32 {}
    impl Sealed for Dyn {}
}

pub trait Sample: sealed::Sealed {
    fn write(&self, sample: Real, w: &mut dyn Write) -> io::Result<()>;

    /// Returns the number of bytes that [`Sample::write`] would write.
    fn size_of(&self) -> usize;
}

/// Represents a [`Sample`] which has a non-changing type.
pub trait TypedSample {
    type Type: Sized + Clone + 'static;

    fn into_typed(sample: Real) -> Self::Type;
}

impl Sample for Int8 {
    fn write(&self, sample: Real, w: &mut dyn Write) -> io::Result<()> {
        w.write_all(&Self::into_typed(sample).to_le_bytes())
    }

    #[inline]
    fn size_of(&self) -> usize {
        std::mem::size_of::<i8>()
    }
}

impl Sample for Int16 {
    fn write(&self, sample: Real, w: &mut dyn Write) -> io::Result<()> {
        w.write_all(&Self::into_typed(sample).to_le_bytes())
    }

    #[inline]
    fn size_of(&self) -> usize {
        std::mem::size_of::<i16>()
    }
}

impl Sample for Int32 {
    fn write(&self, sample: Real, w: &mut dyn Write) -> io::Result<()> {
        w.write_all(&Self::into_typed(sample).to_le_bytes())
    }

    #[inline]
    fn size_of(&self) -> usize {
        std::mem::size_of::<i32>()
    }
}

impl Sample for Float32 {
    fn write(&self, sample: Real, w: &mut dyn Write) -> io::Result<()> {
        w.write_all(&Self::into_typed(sample).to_le_bytes())
    }

    #[inline]
    fn size_of(&self) -> usize {
        std::mem::size_of::<f32>()
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

    fn size_of(&self) -> usize {
        match self.0 {
            SampleType::Int8 => Int8.size_of(),
            SampleType::Int16 => Int16.size_of(),
            SampleType::Int32 => Int32.size_of(),
            SampleType::Float32 => Float32.size_of(),
        }
    }
}

impl TypedSample for Int8 {
    type Type = i8;

    #[inline]
    fn into_typed(sample: Real) -> i8 {
        (sample * 127.0) as i8
    }
}

impl TypedSample for Int16 {
    type Type = i16;

    #[inline]
    fn into_typed(sample: Real) -> i16 {
        (sample * 32767.0) as i16
    }
}

impl TypedSample for Int32 {
    type Type = i32;

    #[inline]
    fn into_typed(sample: Real) -> i32 {
        (sample * 2147483647.0) as i32
    }
}

impl TypedSample for Float32 {
    type Type = f32;

    #[inline]
    fn into_typed(sample: Real) -> f32 {
        sample as f32
    }
}
