pub type Real = f64;

/// Constants for the [`Real`] type
pub mod consts {
    pub use std::f64::consts::*;
}

pub mod buffer;
pub mod err;
pub mod id;
pub mod node;
pub mod pipeline;
pub mod sample;
pub mod wav;
pub mod waveform;

pub mod sinks;
pub mod sources;
