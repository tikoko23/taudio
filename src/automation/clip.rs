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

/// Holds a list of time-ordered control points.
///
/// No two control points whose `sample_offset` field are the same
/// may exist in a single [`ControlPoints`] structure.
///
/// If the number of control points is not 0, the first control point's
/// `sample_offset` field is guaranteed to be 0.
///
/// These invariants are checked by the public API.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct ControlPoints(SmallVec<[ControlPoint; 4]>);

impl ControlPoints {
    #[inline]
    pub fn add_point(&mut self, point: ControlPoint) {
        if let Some(last) = self.0.last() {
            assert!(
                last.sample_offset < point.sample_offset,
                "ascending time-ordering violated"
            );
        } else {
            assert_eq!(
                point.sample_offset, 0,
                "the first element must have no offset"
            );
        }

        self.0.push(point);
    }

    #[inline]
    pub fn points(&self) -> &[ControlPoint] {
        &self.0
    }

    pub fn sample(&self, sample_offset: u32) -> f32 {
        #[inline(always)]
        fn get_value(p: &ControlPoint) -> f32 {
            p.value
        }

        let first_greater_idx = self.0.partition_point(|p| p.sample_offset < sample_offset);

        if first_greater_idx == 0 {
            return self.first().map(get_value).unwrap_or(0.0);
        }

        if first_greater_idx == self.len() {
            return self.last().map(get_value).unwrap_or(0.0);
        }

        let prev = self[first_greater_idx - 1];
        let next = self[first_greater_idx];

        let gap = next.sample_offset - prev.sample_offset;
        let offset_into = sample_offset - prev.sample_offset;

        debug_assert!(offset_into <= gap);

        if offset_into == 0 {
            return prev.value;
        }

        let normalized_offset = offset_into as f32 / gap as f32;

        f32::mul_add(normalized_offset, next.value - prev.value, prev.value)
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

    #[inline]
    pub fn sample(&self, sample_offset: u32) -> f32 {
        let _ = sample_offset;

        todo!("add different oscillator kinds")
    }
}

#[derive(Debug, Clone)]
pub enum AutomationClip {
    Controlled(ControlPoints),
    Lfo(Lfo),
}

impl From<ControlPoints> for AutomationClip {
    #[inline]
    fn from(value: ControlPoints) -> Self {
        Self::Controlled(value)
    }
}

impl From<Lfo> for AutomationClip {
    #[inline]
    fn from(value: Lfo) -> Self {
        Self::Lfo(value)
    }
}

impl AutomationClip {
    pub fn sample(&self, sample_offset: u32) -> f32 {
        match self {
            Self::Lfo(lfo) => lfo.sample(sample_offset),
            Self::Controlled(points) => points.sample(sample_offset),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn make_points(pts: &[(u32, f32)]) -> ControlPoints {
        let mut cp = ControlPoints(SmallVec::new());
        for &(offset, value) in pts {
            cp.add_point(ControlPoint::new(value, offset));
        }
        cp
    }

    #[test]
    fn sample_empty_returns_zero() {
        let cp = ControlPoints(SmallVec::new());
        assert_eq!(cp.sample(0), 0.0);
        assert_eq!(cp.sample(999), 0.0);
    }

    #[test]
    fn sample_single_point_any_offset_returns_its_value() {
        let cp = make_points(&[(0, 0.75)]);
        assert_eq!(cp.sample(0), 0.75);
        assert_eq!(cp.sample(1000), 0.75);
        assert_eq!(cp.sample(u32::MAX), 0.75);
    }

    #[test]
    fn sample_before_first_point_returns_first_value() {
        // sample_offset = 0 is always first, so query offset = 0 explicitly
        let cp = make_points(&[(0, 0.5), (100, 1.0)]);
        assert_eq!(cp.sample(0), 0.5);
    }

    #[test]
    fn sample_beyond_last_point_returns_last_value() {
        let cp = make_points(&[(0, 0.0), (100, 1.0)]);
        assert_eq!(cp.sample(200), 1.0);
        assert_eq!(cp.sample(u32::MAX), 1.0);
    }

    #[test]
    fn sample_exact_first_point() {
        let cp = make_points(&[(0, 0.25), (100, 0.75)]);
        assert_eq!(cp.sample(0), 0.25);
    }

    #[test]
    fn sample_exact_last_point() {
        let cp = make_points(&[(0, 0.0), (100, 0.8)]);
        assert_eq!(cp.sample(100), 0.8);
    }

    #[test]
    fn sample_exact_middle_point() {
        let cp = make_points(&[(0, 0.0), (100, 0.5), (200, 1.0)]);
        assert_eq!(cp.sample(100), 0.5);
    }

    #[test]
    fn sample_midpoint_interpolates_correctly() {
        let cp = make_points(&[(0, 0.0), (100, 1.0)]);
        let result = cp.sample(50);
        assert!((result - 0.5).abs() < 1e-6, "got {result}");
    }

    #[test]
    fn sample_quarter_interpolates_correctly() {
        let cp = make_points(&[(0, 0.0), (100, 1.0)]);
        let result = cp.sample(25);
        assert!((result - 0.25).abs() < 1e-6, "got {result}");
    }

    #[test]
    fn sample_three_quarter_interpolates_correctly() {
        let cp = make_points(&[(0, 0.0), (100, 1.0)]);
        let result = cp.sample(75);
        assert!((result - 0.75).abs() < 1e-6, "got {result}");
    }

    #[test]
    fn sample_interpolates_between_nonzero_values() {
        // 0.2 to 0.6 over 100 samples => midpoint should be 0.4
        let cp = make_points(&[(0, 0.2), (100, 0.6)]);
        let result = cp.sample(50);
        assert!((result - 0.4).abs() < 1e-6, "got {result}");
    }

    #[test]
    fn sample_interpolates_descending() {
        let cp = make_points(&[(0, 1.0), (100, 0.0)]);
        let result = cp.sample(50);
        assert!((result - 0.5).abs() < 1e-6, "got {result}");
    }

    #[test]
    fn sample_selects_correct_segment_with_multiple_points() {
        let cp = make_points(&[(0, 0.0), (100, 0.5), (200, 1.0)]);
        // In the second segment only
        let result = cp.sample(150);
        assert!((result - 0.75).abs() < 1e-6, "got {result}");
    }

    #[test]
    fn sample_flat_segment_returns_constant() {
        let cp = make_points(&[(0, 0.5), (100, 0.5), (200, 1.0)]);
        assert!((cp.sample(50) - 0.5).abs() < 1e-6);
        assert!((cp.sample(99) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn sample_one_before_next_point_is_close_but_not_equal() {
        let cp = make_points(&[(0, 0.0), (100, 1.0)]);
        let result = cp.sample(99);
        assert!(result < 1.0);
        assert!(result > 0.9);
    }

    #[test]
    fn sample_one_after_prev_point_is_close_but_not_zero() {
        let cp = make_points(&[(0, 0.0), (100, 1.0)]);
        let result = cp.sample(1);
        assert!(result > 0.0);
        assert!(result < 0.1);
    }

    #[test]
    #[should_panic = "no offset"]
    fn add_point_first_must_have_zero_offset() {
        let mut cp = ControlPoints(SmallVec::new());
        cp.add_point(ControlPoint::new(0.5, 1));
    }

    #[test]
    #[should_panic = "order"]
    fn add_point_must_be_ascending() {
        let mut cp = make_points(&[(0, 0.0), (100, 1.0)]);
        cp.add_point(ControlPoint::new(0.5, 50)); // Out of order
    }

    #[test]
    #[should_panic = "order"]
    fn add_point_duplicate_offset_panics() {
        let mut cp = make_points(&[(0, 0.0), (100, 1.0)]);
        cp.add_point(ControlPoint::new(0.5, 100)); // Same offset as last
    }
}
