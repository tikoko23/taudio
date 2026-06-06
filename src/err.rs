use std::{
    error::Error,
    ops::{Bound, RangeBounds},
};
use thiserror::Error;

use crate::Real;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AudioError {
    #[error("{0} is not a valid frequency")]
    InvalidFrequency(Real),

    #[error("{0} is not a valid sample rate")]
    InvalidSampleRate(u32),

    #[error("Too few channels: Expected at least {min}, got {got}")]
    TooFewChannels { got: u32, min: u32 },

    #[error("Too many channels: Expected at most {max}, got {got}")]
    TooManyChannels { got: u32, max: u32 },

    #[error("Cycled dependencies")]
    Cycle,

    #[error("Mismatched channels: Expected {expected} outputs for {node_name}, got {got}")]
    MismatchedChannels {
        node_name: String,
        got: u32,
        expected: u32,
    },

    #[error(transparent)]
    Boxed(#[from] Box<dyn Error>),
}

impl AudioError {
    /// Returns [`AudioError::TooFewChannels`] if `got` is below `expected`,
    /// [`AudioError::TooManyChannels`] if `got` is above `expected` and [`Ok`]
    /// if `got` is within `expected`.
    pub fn expect_channels<R: RangeBounds<u32>>(expected: R, got: u32) -> Result<(), AudioError> {
        if expected.contains(&got) {
            return Ok(());
        }

        match expected.start_bound() {
            Bound::Included(&value) if got < value => {
                return Err(AudioError::TooFewChannels { got, min: value });
            }
            Bound::Excluded(&value) if got < value => {
                return Err(AudioError::TooFewChannels {
                    got,
                    min: value + 1,
                });
            }
            _ => {}
        }

        match expected.end_bound() {
            Bound::Included(&value) if got > value => {
                return Err(AudioError::TooManyChannels { got, max: value });
            }
            Bound::Excluded(&value) if got > value => {
                return Err(AudioError::TooManyChannels {
                    got,
                    max: value - 1,
                });
            }
            _ => {}
        }

        unreachable!("all of the above must catch any number")
    }
}

#[cfg(test)]
mod test {
    use crate::err::AudioError;

    macro_rules! assert_matches {
        ($expr:expr, $pat:pat) => {
            assert!(matches!($expr, $pat))
        };
    }

    #[test]
    fn expect_channels() {
        assert_matches!(AudioError::expect_channels(.., 23), Ok(()));

        assert_matches!(
            AudioError::expect_channels(1..=4, 5).unwrap_err(),
            AudioError::TooManyChannels { got: 5, max: 4 }
        );

        assert_matches!(
            AudioError::expect_channels(1..4, 5).unwrap_err(),
            AudioError::TooManyChannels { got: 5, max: 3 }
        );

        assert_matches!(
            AudioError::expect_channels(1..4, 0).unwrap_err(),
            AudioError::TooFewChannels { got: 0, min: 1 }
        );
    }
}
