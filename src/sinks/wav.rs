use std::io::{Seek, Write};

use crate::sample::Sample;

pub struct WavSink<S: Sample, W: Write + Seek> {
    sample: S,
    writer: W,
}

impl<S: Sample, W: Write + Seek> WavSink<S, W> {
    pub fn new(sample: S, writer: W) -> Self {
        Self { sample, writer }
    }
}
