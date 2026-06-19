use std::error::Error;

use taudio::{
    sample::{self, TypedSample},
    wav,
    waveform::{self, Saw},
};

fn main() -> Result<(), Box<dyn Error>> {
    const SAMPLE_RATE: u32 = 44100;

    let mut samples = vec![];
    let mut phase = 0.0;

    for _ in 0..SAMPLE_RATE {
        let s = waveform::osc(&mut Saw, &mut phase, SAMPLE_RATE, 220.0);
        let quantized = sample::Int16::into_typed(s);

        samples.push(quantized);
    }

    wav::dump("saw220hz.wav", SAMPLE_RATE, [samples.as_ref()])?;

    Ok(())
}
