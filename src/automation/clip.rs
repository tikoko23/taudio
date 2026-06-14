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
    period: NonZeroU32,
}

fn lfo_square(t: u32, p: u32) -> f32 {
    ((2 * t / p) % 2) as f32
}

fn lfo_sinusoidal(t: u32, p: u32) -> f32 {
    use std::f32::consts::TAU;

    let t = t as f32;
    let p = p as f32;

    0.5 - 0.5 * f32::cos(TAU * t / p)
}

fn lfo_triangle(t: u32, p: u32) -> f32 {
    let t = t as f32;
    let p = p as f32;

    1.0 - 2.0 / p * f32::abs(t % p - p / 2.0)
}

fn lfo_saw(t: u32, p: u32) -> f32 {
    let t = t as f32;
    let p = p as f32;

    (t / p) % 1.0
}

impl Lfo {
    #[inline]
    pub fn new(kind: LfoShape, period: NonZeroU32) -> Self {
        Self { kind, period }
    }

    #[inline]
    pub fn period(&self) -> NonZeroU32 {
        self.period
    }

    #[inline]
    pub fn sample(&self, sample_offset: u32) -> f32 {
        let t = sample_offset;
        let p = self.period().get();

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

    #[test]
    fn lfo_square_period_one() {
        assert_eq!(lfo_square(0, 1), 0.0);
        assert_eq!(lfo_square(1, 1), 0.0);
        assert_eq!(lfo_square(2, 1), 0.0);
        assert_eq!(lfo_square(3, 1), 0.0);
    }

    fn assert_close(lhs: f32, rhs: f32) {
        const EPS: f32 = 1e-6;

        assert!((lhs - rhs).abs() <= EPS)
    }

    #[test]
    fn lfo_square_period_halftime() {
        assert_eq!(lfo_square(22049, 44100), 0.0);
        assert_eq!(lfo_square(22050, 44100), 1.0);
        assert_eq!(lfo_square(44100, 44100), 0.0);
    }

    #[test]
    fn lfo_sinusoidal_period_one() {
        assert_close(lfo_sinusoidal(0, 1), 0.0);
        assert_close(lfo_sinusoidal(1, 1), 0.0);
        assert_close(lfo_sinusoidal(2, 1), 0.0);
        assert_close(lfo_sinusoidal(3, 1), 0.0);
    }

    #[test]
    fn lfo_sinusoidal_period_halftime() {
        // We leave some wiggle room for floats, that's why the halftime sample is at 22000.
        assert!(lfo_sinusoidal(22000, 44100) < 1.0);
        assert_close(lfo_sinusoidal(22050, 44100), 1.0);
        assert_close(lfo_sinusoidal(44100, 44100), 0.0);
    }

    #[test]
    fn lfo_triangle_period_one() {
        assert_close(lfo_triangle(0, 1), 0.0);
        assert_close(lfo_triangle(1, 1), 0.0);
        assert_close(lfo_triangle(2, 1), 0.0);
        assert_close(lfo_triangle(3, 1), 0.0);
    }

    #[test]
    fn lfo_triangle_period_halftime() {
        // The triangle wave is a simple shape so we expect more precision.
        assert!(lfo_triangle(22049, 44100) < 1.0);
        assert_close(lfo_triangle(22050, 44100), 1.0);
        assert_close(lfo_triangle(44100, 44100), 0.0);
    }

    #[test]
    fn lfo_saw_period_one() {
        assert_close(lfo_saw(0, 1), 0.0);
        assert_close(lfo_saw(1, 1), 0.0);
        assert_close(lfo_saw(2, 1), 0.0);
        assert_close(lfo_saw(3, 1), 0.0);
    }

    #[test]
    fn lfo_saw_period_halftime() {
        assert!(lfo_saw(22049, 44100) < 0.5);
        assert_close(lfo_saw(22050, 44100), 0.5);
        assert_close(lfo_saw(44100, 44100), 0.0);
    }

    #[test]
    fn lfo_saw_increasing() {
        let mut last = f32::NEG_INFINITY;

        for t in 0..1000 {
            let new = lfo_saw(t, 1000);

            assert!(last < new);

            last = new;
        }
    }
}
