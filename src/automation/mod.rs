mod clip;
mod timeline;
mod track;

pub use clip::*;
pub use timeline::*;
pub use track::*;

/// Normalizes a value into seconds according to the given sample rate.
#[inline]
pub fn normalize(sample_rate: u64, value: u64) -> f64 {
    (value as f64) / (sample_rate as f64)
}

/// Calculates the new sample offset of a value from the old sample rate.
pub fn rerate(old_sample_rate: u64, new_sample_rate: u64, value: u64) -> u64 {
    let old_sample_rate = old_sample_rate as f64;
    let new_sample_rate = new_sample_rate as f64;

    let conversion_factor = new_sample_rate / old_sample_rate;

    (value as f64 * conversion_factor) as u64
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn normalize_zero() {
        assert_eq!(normalize(48000, 0), 0.0);
    }

    #[test]
    fn normalize_one_second() {
        assert_eq!(normalize(48000, 48000), 1.0);
    }

    #[test]
    fn normalize_sub_second() {
        assert_eq!(normalize(48000, 24000), 0.5);
    }

    #[test]
    fn normalize_44100() {
        assert_eq!(normalize(44100, 44100), 1.0);
    }

    #[test]
    fn normalize_large_value() {
        let hours: u64 = 106_751_991;
        let samples = hours * 48000 * 3600;
        let seconds = normalize(48000, samples);
        assert!((seconds - (hours * 3600) as f64).abs() < 1.0);
    }

    #[test]
    fn rerate_zero() {
        assert_eq!(rerate(44100, 48000, 0), 0);
    }

    #[test]
    fn rerate_same_rate() {
        assert_eq!(rerate(48000, 48000, 48000), 48000);
    }

    #[test]
    fn rerate_upsample_44100_to_48000() {
        let result = rerate(44100, 48000, 44100);
        assert_eq!(result, 48000);
    }

    #[test]
    fn rerate_downsample_48000_to_44100() {
        let result = rerate(48000, 44100, 48000);
        assert_eq!(result, 44100);
    }

    #[test]
    fn rerate_upsample_half_second() {
        let result = rerate(44100, 48000, 22050);
        assert_eq!(result, 24000);
    }

    #[test]
    fn rerate_double_sample_rate() {
        assert_eq!(rerate(24000, 48000, 24000), 48000);
    }

    #[test]
    fn rerate_half_sample_rate() {
        assert_eq!(rerate(48000, 24000, 48000), 24000);
    }

    #[test]
    fn rerate_preserves_duration_in_seconds() {
        // Whatever the rates, normalized seconds should match before and after
        let original = 88200u64; // 2 seconds at 44100
        let resampled = rerate(44100, 48000, original);
        let before = normalize(44100, original);
        let after = normalize(48000, resampled);
        assert!((before - after).abs() < 1e-9);
    }
}
