use std::ops::Deref;

use smallvec::{SmallVec, smallvec};

use crate::Real;

#[derive(Debug, Clone, Copy)]
pub struct ControlPoint {
    value: Real,
    offset: Real,
}

impl ControlPoint {
    /// Creates a new control point.
    ///
    /// # Panics
    /// - Panics if the value is not in `0.0..=1.0`.
    /// - Panics if the offset is non-finite or less than 0.
    pub fn new(value: Real, offset: Real) -> Self {
        if !(0.0..=1.0).contains(&value) {
            panic!("control point value must be normalized");
        }

        assert!(
            offset.is_finite() && offset >= 0.0,
            "offset must be positive or 0 and finite"
        );

        Self { value, offset }
    }

    #[inline]
    pub fn value(&self) -> Real {
        self.value
    }

    #[inline]
    pub fn offset(&self) -> Real {
        self.offset
    }
}

/// Holds a list of time-ordered control points.
///
/// No two control points whose `sample_offset` field are the same
/// may exist in a single [`ControlPoints`] structure.
///
/// If the number of control points is not 0, the first control point's
/// `offset` field is guaranteed to be 0.
///
/// These invariants are checked by the public API.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct ControlPoints(SmallVec<[ControlPoint; 4]>);

impl FromIterator<ControlPoint> for ControlPoints {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = ControlPoint>,
    {
        let mut points = ControlPoints::new();

        for point in iter.into_iter() {
            points.add_point(point);
        }

        points
    }
}

impl Default for ControlPoints {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ControlPoints {
    #[inline]
    pub fn new() -> Self {
        Self(smallvec![])
    }

    /// Adds a point to the end of the control point list.
    ///
    /// # Panics
    /// - Panics if the new control point comes before or at the same
    ///   time as the last added point with respect to time.
    /// - Panics if the new control point is the first and it has a non-zero offset.
    #[inline]
    pub fn add_point(&mut self, point: ControlPoint) {
        if let Some(last) = self.0.last() {
            assert!(
                last.offset < point.offset,
                "ascending time-ordering violated"
            );
        } else {
            assert_eq!(point.offset, 0.0, "the first element must have no offset");
        }

        self.0.push(point);
    }

    #[inline]
    pub fn points(&self) -> &[ControlPoint] {
        &self.0
    }

    pub fn sample(&self, offset: Real) -> Real {
        #[inline(always)]
        fn get_value(p: &ControlPoint) -> Real {
            p.value
        }

        let first_greater_idx = self.0.partition_point(|p| p.offset < offset);

        if first_greater_idx == 0 {
            return self.first().map(get_value).unwrap_or(0.0);
        }

        if first_greater_idx == self.len() {
            return self.last().map(get_value).unwrap_or(0.0);
        }

        let prev = self[first_greater_idx - 1];
        let next = self[first_greater_idx];

        let gap = next.offset - prev.offset;
        let offset_into = offset - prev.offset;

        debug_assert!(offset_into <= gap);

        if offset_into == 0.0 {
            return prev.value;
        }

        let normalized_offset = offset_into as Real / gap as Real;

        Real::mul_add(normalized_offset, next.value - prev.value, prev.value)
    }
}

impl Deref for ControlPoints {
    type Target = [ControlPoint];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Base shape for an LFO.
///
/// These shapes define different waveforms which all output all values in `[0, 1]` at least
/// once within their period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LfoShape {
    /// A two-valued function resembling a square wave.
    ///
    /// For the first half of its period, it outputs 0. For the remaining, it outputs 1.
    ///
    /// This function is equivalent to the following when used with a period of `p`:
    ///
    /// `f(x) = 0, x ∈ [0, p/2)`
    ///
    /// `f(x) = 1, x ∈ [p/2, p)`
    Square,

    /// A smoothly interpolated sinusoidal wave.
    ///
    /// This function is equivalent to the following when used with a period of `p`:
    ///
    /// `f(x) = 0.5 - cos(2π * x / p) / 2`
    Sinusoidal,

    /// A continuous function resembling a triangle wave.
    ///
    /// For a period of `p`, this function is:
    /// - 0 at `x = 0`
    /// - 1 at `x = p/2`
    /// - Linear-increasing for `x ∈ (0, p/2)`
    /// - Linear-decreasing for `x ∈ (p/2, p)`
    Triangle,

    /// A steadily increasing (within its period) function resembling a saw-tooth wave.
    ///
    /// This function is equivalent to the following when used with a period of `p`:
    ///
    /// `f(x) = (x / p) % 1`
    ///
    /// Where "remainder one" represents the fractional part of the expression.
    Saw,
}

#[derive(Debug, Clone)]
pub struct Lfo {
    kind: LfoShape,
    period: Real,
}

fn lfo_square(t: Real, p: Real) -> Real {
    ((2.0 * t / p) % 2.0).floor()
}

fn lfo_sinusoidal(t: Real, p: Real) -> Real {
    use crate::consts::TAU;

    0.5 - 0.5 * Real::cos(TAU * t / p)
}

fn lfo_triangle(t: Real, p: Real) -> Real {
    1.0 - 2.0 / p * (t % p - p / 2.0).abs()
}

fn lfo_saw(t: Real, p: Real) -> Real {
    (t / p) % 1.0
}

impl Lfo {
    /// Creates a new LFO.
    ///
    /// # Panics
    /// - Panics if the period is non-finite or not greater than 0.
    #[inline]
    pub fn new(kind: LfoShape, period: Real) -> Self {
        assert!(period.is_finite(), "period must be finite");
        assert!(period > 0.0, "period must be positive");

        Self { kind, period }
    }

    #[inline]
    pub fn period(&self) -> Real {
        self.period
    }

    #[inline]
    pub fn sample(&self, offset: Real) -> Real {
        let t = offset;
        let p = self.period();

        match self.kind {
            LfoShape::Square => lfo_square(t, p),
            LfoShape::Sinusoidal => lfo_sinusoidal(t, p),
            LfoShape::Triangle => lfo_triangle(t, p),
            LfoShape::Saw => lfo_saw(t, p),
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum AutomationClip {
    Controlled(ControlPoints),
    Lfo(Lfo),
    Constant(Real),
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
    pub fn sample(&self, offset: Real) -> Real {
        match self {
            Self::Lfo(lfo) => lfo.sample(offset),
            Self::Controlled(points) => points.sample(offset),
            Self::Constant(x) => *x,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn make_points(pts: &[(Real, Real)]) -> ControlPoints {
        let mut cp = ControlPoints(SmallVec::new());
        for &(offset, value) in pts {
            cp.add_point(ControlPoint::new(value, offset));
        }
        cp
    }

    #[test]
    fn sample_empty_returns_zero() {
        let cp = ControlPoints(SmallVec::new());
        assert_eq!(cp.sample(0.0), 0.0);
        assert_eq!(cp.sample(999.0), 0.0);
    }

    #[test]
    fn sample_single_point_any_offset_returns_its_value() {
        let cp = make_points(&[(0.0, 0.75)]);
        assert_eq!(cp.sample(0.0), 0.75);
        assert_eq!(cp.sample(1000.0), 0.75);
        assert_eq!(cp.sample(1e9), 0.75);
    }

    #[test]
    fn sample_before_first_point_returns_first_value() {
        // sample_offset = 0 is always first, so query offset = 0 explicitly
        let cp = make_points(&[(0.0, 0.5), (100.0, 1.0)]);
        assert_eq!(cp.sample(0.0), 0.5);
    }

    #[test]
    fn sample_beyond_last_point_returns_last_value() {
        let cp = make_points(&[(0.0, 0.0), (100.0, 1.0)]);
        assert_eq!(cp.sample(200.0), 1.0);
        assert_eq!(cp.sample(1e9), 1.0);
    }

    #[test]
    fn sample_exact_first_point() {
        let cp = make_points(&[(0.0, 0.25), (100.0, 0.75)]);
        assert_eq!(cp.sample(0.0), 0.25);
    }

    #[test]
    fn sample_exact_last_point() {
        let cp = make_points(&[(0.0, 0.0), (100.0, 0.8)]);
        assert_eq!(cp.sample(100.0), 0.8);
    }

    #[test]
    fn sample_exact_middle_point() {
        let cp = make_points(&[(0.0, 0.0), (100.0, 0.5), (200.0, 1.0)]);
        assert_eq!(cp.sample(100.0), 0.5);
    }

    #[test]
    fn sample_midpoint_interpolates_correctly() {
        let cp = make_points(&[(0.0, 0.0), (100.0, 1.0)]);
        let result = cp.sample(50.0);
        assert!((result - 0.5).abs() < 1e-9, "got {result}");
    }

    #[test]
    fn sample_quarter_interpolates_correctly() {
        let cp = make_points(&[(0.0, 0.0), (100.0, 1.0)]);
        let result = cp.sample(25.0);
        assert!((result - 0.25).abs() < 1e-9, "got {result}");
    }

    #[test]
    fn sample_three_quarter_interpolates_correctly() {
        let cp = make_points(&[(0.0, 0.0), (100.0, 1.0)]);
        let result = cp.sample(75.0);
        assert!((result - 0.75).abs() < 1e-9, "got {result}");
    }

    #[test]
    fn sample_interpolates_between_nonzero_values() {
        // 0.2 to 0.6 over 100 samples => midpoint should be 0.4
        let cp = make_points(&[(0.0, 0.2), (100.0, 0.6)]);
        let result = cp.sample(50.0);
        assert!((result - 0.4).abs() < 1e-9, "got {result}");
    }

    #[test]
    fn sample_interpolates_descending() {
        let cp = make_points(&[(0.0, 1.0), (100.0, 0.0)]);
        let result = cp.sample(50.0);
        assert!((result - 0.5).abs() < 1e-9, "got {result}");
    }

    #[test]
    fn sample_selects_correct_segment_with_multiple_points() {
        let cp = make_points(&[(0.0, 0.0), (100.0, 0.5), (200.0, 1.0)]);
        // In the second segment only
        let result = cp.sample(150.0);
        assert!((result - 0.75).abs() < 1e-9, "got {result}");
    }

    #[test]
    fn sample_flat_segment_returns_constant() {
        let cp = make_points(&[(0.0, 0.5), (100.0, 0.5), (200.0, 1.0)]);
        assert!((cp.sample(50.0) - 0.5).abs() < 1e-9);
        assert!((cp.sample(99.0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn sample_one_before_next_point_is_close_but_not_equal() {
        let cp = make_points(&[(0.0, 0.0), (100.0, 1.0)]);
        let result = cp.sample(99.0);
        assert!(result < 1.0);
        assert!(result > 0.9);
    }

    #[test]
    fn sample_one_after_prev_point_is_close_but_not_zero() {
        let cp = make_points(&[(0.0, 0.0), (100.0, 1.0)]);
        let result = cp.sample(1.0);
        assert!(result > 0.0);
        assert!(result < 0.1);
    }

    #[test]
    #[should_panic = "no offset"]
    fn add_point_first_must_have_zero_offset() {
        let mut cp = ControlPoints(SmallVec::new());
        cp.add_point(ControlPoint::new(0.5, 1.0));
    }

    #[test]
    #[should_panic = "order"]
    fn add_point_must_be_ascending() {
        let mut cp = make_points(&[(0.0, 0.0), (100.0, 1.0)]);
        cp.add_point(ControlPoint::new(0.5, 50.0)); // Out of order
    }

    #[test]
    #[should_panic = "order"]
    fn add_point_duplicate_offset_panics() {
        let mut cp = make_points(&[(0.0, 0.0), (100.0, 1.0)]);
        cp.add_point(ControlPoint::new(0.5, 100.0)); // Same offset as last
    }

    fn assert_close(lhs: Real, rhs: Real) {
        const EPS: Real = 1e-9;

        assert!((lhs - rhs).abs() <= EPS)
    }

    #[test]
    fn lfo_square_period_one() {
        assert_eq!(lfo_square(0.0, 1.0), 0.0);
        assert_eq!(lfo_square(1.0, 1.0), 0.0);
        assert_eq!(lfo_square(2.0, 1.0), 0.0);
        assert_eq!(lfo_square(3.0, 1.0), 0.0);
    }

    #[test]
    fn lfo_square_period_halftime() {
        assert_eq!(lfo_square(0.0, 1.0), 0.0);
        assert_eq!(lfo_square(0.5, 1.0), 1.0);
        assert_eq!(lfo_square(1.0, 1.0), 0.0);
    }

    #[test]
    fn lfo_sinusoidal_period_one() {
        assert_close(lfo_sinusoidal(0.0, 1.0), 0.0);
        assert_close(lfo_sinusoidal(1.0, 1.0), 0.0);
        assert_close(lfo_sinusoidal(2.0, 1.0), 0.0);
        assert_close(lfo_sinusoidal(3.0, 1.0), 0.0);
    }

    #[test]
    fn lfo_sinusoidal_period_halftime() {
        assert!(lfo_sinusoidal(0.0, 1.0) < 1.0);
        assert_close(lfo_sinusoidal(0.5, 1.0), 1.0);
        assert_close(lfo_sinusoidal(1.0, 1.0), 0.0);
    }

    #[test]
    fn lfo_triangle_period_one() {
        assert_close(lfo_triangle(0.0, 1.0), 0.0);
        assert_close(lfo_triangle(1.0, 1.0), 0.0);
        assert_close(lfo_triangle(2.0, 1.0), 0.0);
        assert_close(lfo_triangle(3.0, 1.0), 0.0);
    }

    #[test]
    fn lfo_triangle_period_halftime() {
        assert!(lfo_triangle(0.0, 1.0) < 1.0);
        assert_close(lfo_triangle(0.5, 1.0), 1.0);
        assert_close(lfo_triangle(1.0, 1.0), 0.0);
    }

    #[test]
    fn lfo_saw_period_one() {
        assert_close(lfo_saw(0.0, 1.0), 0.0);
        assert_close(lfo_saw(1.0, 1.0), 0.0);
        assert_close(lfo_saw(2.0, 1.0), 0.0);
        assert_close(lfo_saw(3.0, 1.0), 0.0);
    }

    #[test]
    fn lfo_saw_period_halftime() {
        assert!(lfo_saw(0.4999, 1.0) < 0.5);
        assert_close(lfo_saw(0.5, 1.0), 0.5);
        assert_close(lfo_saw(1.0, 1.0), 0.0);
    }

    #[test]
    fn lfo_saw_increasing() {
        let mut last = Real::NEG_INFINITY;

        for t in 0..1000 {
            let new = lfo_saw(t as Real / 1000.0, 1.0);

            assert!(last < new);

            last = new;
        }
    }
}
