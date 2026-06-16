use std::ops::RangeInclusive;

use crate::Real;

/// Wrapper around [`RangeInclusive`] which forbids empty ranges.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Parameter(RangeInclusive<Real>);

impl Parameter {
    /// Constructs a new parameter.
    pub fn new(range: RangeInclusive<Real>) -> Self {
        assert!(range.end() >= range.start(), "range cannot be empty");

        Self(range)
    }

    #[inline]
    pub fn new_single_value(value: Real) -> Self {
        Self(value..=value)
    }

    #[inline]
    pub fn is_single_valued(&self) -> bool {
        self.start() == self.end()
    }

    #[inline]
    pub const fn start(&self) -> Real {
        *self.0.start()
    }

    #[inline]
    pub const fn end(&self) -> Real {
        *self.0.end()
    }

    #[inline]
    pub fn into_range(self) -> RangeInclusive<Real> {
        self.into()
    }
}

impl From<Parameter> for RangeInclusive<Real> {
    #[inline]
    fn from(value: Parameter) -> Self {
        value.0
    }
}

#[cfg(test)]
mod test {
    use crate::automation::Parameter;

    #[test]
    fn param_ok() {
        let p = Parameter::new(23.0..=37.0);

        assert_eq!(p.start(), 23.0);
        assert_eq!(p.end(), 37.0);
        assert_eq!(p.into_range(), 23.0..=37.0);
    }

    #[test]
    fn param_eq() {
        let p = Parameter::new(0.0..=0.0);

        assert_eq!(p.into_range(), 0.0..=0.0);
    }

    #[test]
    #[should_panic = "range cannot be empty"]
    fn param_bad() {
        let _ = Parameter::new(1.0..=0.0);
    }
}
