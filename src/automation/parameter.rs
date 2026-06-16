use crate::{
    Real,
    automation::{AutomationId, AutomationTimeline},
};

/// A controllable parameter with a configurable mapping.
#[derive(Debug, Clone, Copy)]
pub enum Parameter<T, M>
where
    T: Copy,
    M: Mapping<Value = T>,
{
    Automated { id: AutomationId, mapping: M },
    Constant(T),
}

impl<T, M> Parameter<T, M>
where
    T: Copy,
    M: Mapping<Value = T>,
{
    /// Evaluates the parameter at the given time point.
    ///
    /// This will skip a timeline query if the parameter is a constant.
    ///
    /// # Panics
    /// Panics if the inner call to [`AutomationTimeline::query_value`] panics.
    pub fn sample(&self, time: Real, timeline: &AutomationTimeline) -> T {
        match self {
            Self::Constant(x) => *x,
            Self::Automated { id, mapping } => {
                let a = timeline.query_value(*id, time);

                mapping.map(a)
            }
        }
    }
}

/// Description of how a parameter should be mapped to a concrete value.
pub trait Mapping {
    /// The concrete value type.
    type Value: Copy;

    /// Returns the value that `0` is mapped to.
    #[inline]
    fn zero(&self) -> Self::Value {
        let (lo, _) = self.endpoints();

        lo
    }

    /// Returns the value that `1` is mapped to.
    #[inline]
    fn one(&self) -> Self::Value {
        let (_, hi) = self.endpoints();

        hi
    }

    fn endpoints(&self) -> (Self::Value, Self::Value);

    /// Maps a floating point number in `0.0..=1.0` to a concrete value.
    fn map(&self, x: Real) -> Self::Value;
}

/// Continuous mapping types.
///
/// ```
/// # use taudio::{Real, automation::{CurveMapping, Mapping}};
/// #
/// // Use parameter mappings to generate note frequencies.
/// let mapping = CurveMapping::Exp2(220.0, 440.0);
/// let note_names = [
///     "A3", "A#3", "B3", "C4", "C#4", "D4",
///     "D#4", "E4", "F4", "F#4", "G4", "G#4", "A4"
/// ];
///
/// for (semitone, name) in note_names.into_iter().enumerate() {
///     let normalized = semitone as Real / 12.0;
///     let hz = mapping.map(normalized);
///
///     println!("{name:3} is {hz:.3} Hz");
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub enum CurveMapping {
    /// Linearly interpolates between the two values.
    Linear(Real, Real),

    /// Interpolates between the two values with an exponential curve.
    Exp2(Real, Real),
}

impl Mapping for CurveMapping {
    type Value = Real;

    fn endpoints(&self) -> (Real, Real) {
        match *self {
            Self::Linear(a, b) => (a, b),
            Self::Exp2(a, b) => (a, b),
        }
    }

    fn map(&self, x: Real) -> Real {
        match *self {
            Self::Linear(a, b) => (b - a) * x + a,
            Self::Exp2(a, b) => (b / a).powf(x) * a,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        Real,
        automation::{CurveMapping, Mapping},
    };

    static NOTE_FREQUENCIES: [Real; 13] = [
        220.0,
        233.08188075904496,
        246.94165062806206,
        261.6255653005986,
        277.1826309768721,
        293.6647679174076,
        311.1269837220809,
        329.6275569128699,
        349.2282314330039,
        369.9944227116344,
        391.99543598174927,
        415.3046975799451,
        440.0,
    ];

    fn assert_close(x: Real, y: Real) {
        const EPS: Real = 1e-9;

        assert!((x - y).abs() <= EPS);
    }

    #[test]
    fn mapping_exp() {
        let m = CurveMapping::Exp2(220.0, 440.0);

        for (i, freq) in NOTE_FREQUENCIES.into_iter().enumerate() {
            let norm = i as Real / 12.0;
            assert_close(m.map(norm), freq);
        }
    }
}
