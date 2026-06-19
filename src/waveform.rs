use crate::{Real, consts::TAU};

/// A wave source which can be sampled at an arbitrary time position and frequency.
pub trait WaveSource {
    /// Samples at the given phase.
    ///
    /// The phase parameter is expected to be in `0.0..=1.0` but there are no guarantees
    /// about the actual value passed in.
    ///
    /// Implementations should output values within `-1.0..=1.0` but this isn't strictly
    /// required.
    fn sample(&mut self, phase: Real) -> Real;
}

/// A sine wave.
#[derive(Debug, Clone, Copy)]
pub struct Sine;

/// A triangle wave.
#[derive(Debug, Clone, Copy)]
pub struct Triangle;

/// A square wave.
///
/// Note that unlike other built-in waves, this wave *does not* output 0 when sampled at 0
/// because a perfect square wave has no 0 points.
#[derive(Debug, Clone, Copy)]
pub struct Square;

/// A saw wave.
///
/// Note that just like other built-in waves, this wave outputs 0 when sampled at 0.
#[derive(Debug, Clone, Copy)]
pub struct Saw;

impl WaveSource for Sine {
    fn sample(&mut self, phase: Real) -> Real {
        Real::sin(TAU * phase)
    }
}

impl WaveSource for Triangle {
    fn sample(&mut self, phase: Real) -> Real {
        1.0 - f64::abs(1.0 - (f64::abs(4.0 * phase - 3.0) - 1.0))
    }
}

impl WaveSource for Square {
    fn sample(&mut self, phase: Real) -> Real {
        2.0 * Real::round(phase) - 1.0
    }
}

impl WaveSource for Saw {
    fn sample(&mut self, phase: Real) -> Real {
        2.0 * phase - 1.0
    }
}

/// Sample the wave with the given frequency while updating phase information.
///
/// ```
/// use taudio::waveform::{self, Sine};
///
/// let mut phase = 0.0;
/// let mut samples = vec![];
///
/// for _ in 0..44100 {
///     let s = waveform::osc(&mut Sine, &mut phase, 44100, 440.0);
///
///     samples.push(s);
/// }
/// ```
pub fn osc<W: WaveSource>(
    waveform: &mut W,
    phase: &mut Real,
    samples_per_second: u32,
    freq: Real,
) -> Real {
    let value = waveform.sample(*phase);

    *phase += freq / (samples_per_second as Real);
    *phase %= 1.0;

    value
}

#[cfg(test)]
mod test {
    use super::*;

    const EPS: Real = 1e-6;

    fn assert_close(a: Real, b: Real) {
        assert!((a - b).abs() < EPS, "{a} != {b}");
    }

    #[test]
    fn sine_key_points() {
        let mut wave = Sine;

        assert_close(wave.sample(0.0), 0.0);
        assert_close(wave.sample(0.25), 1.0);
        assert_close(wave.sample(0.5), 0.0);
        assert_close(wave.sample(0.75), -1.0);
        assert_close(wave.sample(1.0), 0.0);
    }

    #[test]
    fn triangle_key_points() {
        let mut wave = Triangle;

        assert_close(wave.sample(0.0), 0.0);
        assert_close(wave.sample(0.25), 1.0);
        assert_close(wave.sample(0.5), 0.0);
        assert_close(wave.sample(0.75), -1.0);
        assert_close(wave.sample(1.0), 0.0);
    }

    #[test]
    fn square_key_points() {
        let mut wave = Square;

        assert_close(wave.sample(0.0), -1.0);
        assert_close(wave.sample(0.25), -1.0);
        assert_close(wave.sample(0.49), -1.0);

        assert_close(wave.sample(0.51), 1.0);
        assert_close(wave.sample(0.75), 1.0);
        assert_close(wave.sample(0.99), 1.0);
    }

    #[test]
    fn saw_key_points() {
        let mut wave = Saw;

        assert_close(wave.sample(0.0), -1.0);
        assert_close(wave.sample(0.25), -0.5);
        assert_close(wave.sample(0.5), 0.0);
        assert_close(wave.sample(0.75), 0.5);
        assert_close(wave.sample(1.0), 1.0);
    }

    #[test]
    fn outputs_are_in_range() {
        let mut sine = Sine;
        let mut triangle = Triangle;
        let mut square = Square;
        let mut saw = Saw;

        for i in 0..10_000 {
            let t = i as Real * 0.0001;

            for value in [
                sine.sample(t),
                triangle.sample(t),
                square.sample(t),
                saw.sample(t),
            ] {
                assert!(((-1.0 - EPS)..=(1.0 + EPS)).contains(&value));
            }
        }
    }
}
