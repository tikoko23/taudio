use std::{num::NonZeroU32, ops::Deref};

use smallvec::SmallVec;

#[derive(Debug, Clone, Copy)]
pub struct ControlPoint {
    value: f32,
    sample_offset: u32,
}

impl ControlPoint {
    pub fn new(value: f32, sample_offset: u32) -> Self {
        if !(0.0..=1.0).contains(&value) {
            panic!("control point value must be normalized");
        }

        Self {
            value,
            sample_offset,
        }
    }

    #[inline]
    pub fn value(&self) -> f32 {
        self.value
    }

    #[inline]
    pub fn sample_offset(&self) -> u32 {
        self.sample_offset
    }
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct ControlPoints(pub(crate) SmallVec<[ControlPoint; 4]>);

impl ControlPoints {
    #[inline]
    pub fn add_point(&mut self, point: ControlPoint) {
        if let Some(last) = self.0.last() {
            assert!(
                last.sample_offset < point.sample_offset,
                "ascending time-ordering violated"
            );
        }

        self.0.push(point);
    }

    #[inline]
    pub fn points(&self) -> &[ControlPoint] {
        &self.0
    }
}

impl Deref for ControlPoints {
    type Target = [ControlPoint];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct Lfo {
    period: NonZeroU32,
}

impl Lfo {
    #[inline]
    pub fn new(period: NonZeroU32) -> Self {
        Self { period }
    }

    #[inline]
    pub fn period(&self) -> NonZeroU32 {
        self.period
    }
}

#[derive(Debug, Clone)]
pub enum AutomationClip {
    Controlled(ControlPoints),
    Lfo(Lfo),
}
