use crate::{Real, consts::TAU};

/// A wave source which can be sampled at an arbitrary time position and frequency.
pub trait WaveSource {
    /// Samples at the given time point with the provided frequency.
    fn sample(&mut self, freq: Real, time: Real) -> Real;
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
    fn sample(&mut self, freq: Real, time: Real) -> Real {
        Real::sin(TAU * freq * time)
    }
}

impl WaveSource for Triangle {
    // TODO: find a simpler expression
    fn sample(&mut self, freq: Real, time: Real) -> Real {
        1.0 - Real::abs(2.0 - 4.0 * ((freq * time + 0.25) % 1.0))
    }
}

impl WaveSource for Square {
    // TODO: find a simpler expression
    fn sample(&mut self, freq: Real, time: Real) -> Real {
        2.0 * Real::floor(2.0 * (freq * time % 1.0)) - 1.0
    }
}

impl WaveSource for Saw {
    // TODO: find a simpler expression
    fn sample(&mut self, freq: Real, time: Real) -> Real {
        (2.0 * freq * time + 1.0) % 2.0 - 1.0
    }
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

        assert_close(wave.sample(1.0, 0.0), 0.0);
        assert_close(wave.sample(1.0, 0.25), 1.0);
        assert_close(wave.sample(1.0, 0.5), 0.0);
        assert_close(wave.sample(1.0, 0.75), -1.0);
        assert_close(wave.sample(1.0, 1.0), 0.0);
    }

    #[test]
    fn triangle_key_points() {
        let mut wave = Triangle;

        assert_close(wave.sample(1.0, 0.0), 0.0);
        assert_close(wave.sample(1.0, 0.25), 1.0);
        assert_close(wave.sample(1.0, 0.5), 0.0);
        assert_close(wave.sample(1.0, 0.75), -1.0);
        assert_close(wave.sample(1.0, 1.0), 0.0);
    }

    #[test]
    fn square_key_points() {
        let mut wave = Square;

        assert_close(wave.sample(1.0, 0.0), -1.0);
        assert_close(wave.sample(1.0, 0.25), -1.0);
        assert_close(wave.sample(1.0, 0.49), -1.0);

        assert_close(wave.sample(1.0, 0.51), 1.0);
        assert_close(wave.sample(1.0, 0.75), 1.0);
        assert_close(wave.sample(1.0, 0.99), 1.0);
    }

    #[test]
    fn saw_key_points() {
        let mut wave = Saw;

        assert_close(wave.sample(1.0, 0.0), 0.0);
        assert_close(wave.sample(1.0, 0.25), 0.5);
        assert_close(wave.sample(1.0, 0.5), -1.0);
        assert_close(wave.sample(1.0, 0.75), -0.5);
        assert_close(wave.sample(1.0, 1.0), 0.0);
    }

    #[test]
    fn waves_are_periodic() {
        let mut sine = Sine;
        let mut triangle = Triangle;
        let mut square = Square;
        let mut saw = Saw;

        let t = 0.12345;
        let f = 7.0;
        let period = 1.0 / f;

        assert_close(sine.sample(f, t), sine.sample(f, t + period));

        assert_close(triangle.sample(f, t), triangle.sample(f, t + period));

        assert_close(square.sample(f, t), square.sample(f, t + period));

        assert_close(saw.sample(f, t), saw.sample(f, t + period));
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
                sine.sample(3.7, t),
                triangle.sample(3.7, t),
                square.sample(3.7, t),
                saw.sample(3.7, t),
            ] {
                assert!(((-1.0 - EPS)..=(1.0 + EPS)).contains(&value));
            }
        }
    }
}
