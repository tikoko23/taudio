use std::fmt::Debug;

use crate::{
    Real,
    automation::{CurveMapping, IntoParameter, Parameter},
    buffer::SampleChannels,
    err::AudioError,
    node::{AudioNode, AudioSource, AudioSourceCfg, AudioSourceInfo, SamplingContext},
    pipeline::IntoNodeOutputIndex,
    waveform::WaveSource,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OscPort {
    /// The waveform output of the oscillator node.
    Output(u32),
}

impl<W> IntoNodeOutputIndex<Osc<W>> for OscPort
where
    W: WaveSource + Debug + Clone + 'static,
{
    fn into_node_output_index(self) -> u32 {
        match self {
            Self::Output(n) => n,
        }
    }
}

/// An oscillator which produces samples of a specific frequency and amplitude.
///
/// # Example
/// ```
/// # use taudio::sources::Osc;
/// use taudio::{waveform, automation::Parameter};
///
/// let osc = Osc::new(waveform::Sine, 440.0, 1.0, 1);
/// ```
#[derive(Debug, Clone)]
pub struct Osc<W: WaveSource + Debug + Clone + 'static> {
    source: W,
    freq: Parameter<Real, CurveMapping>,
    amp: Parameter<Real, CurveMapping>,
    phase: Real,
    num_outputs: usize,
}

impl<W: WaveSource + Debug + Clone + 'static> Osc<W> {
    #[inline]
    pub fn new(
        source: W,
        freq: impl IntoParameter<Real, CurveMapping>,
        amp: impl IntoParameter<Real, CurveMapping>,
        num_outputs: usize,
    ) -> Self {
        Self {
            phase: 0.0,
            source,
            freq: freq.into_parameter(),
            amp: amp.into_parameter(),
            num_outputs,
        }
    }
}

impl<W: WaveSource + Debug + Clone + 'static> AudioNode for Osc<W> {
    fn name(&self) -> &str {
        "@builtin:osc"
    }
}

impl<W: WaveSource + Debug + Clone + 'static> AudioSource for Osc<W> {
    fn setup(&mut self, cfg: &AudioSourceCfg) -> Result<AudioSourceInfo, AudioError> {
        let _ = cfg;

        Ok(AudioSourceInfo {
            num_outputs: self.num_outputs,
        })
    }

    fn sample(
        &mut self,
        ctx: &SamplingContext,
        output: &mut SampleChannels<'_>,
    ) -> Result<(), AudioError> {
        for sample in 0..ctx.batch_size() {
            let t = ctx.time_of(sample);
            let f = self.freq.sample(t, ctx.automations());
            let a = self.amp.sample(t, ctx.automations());

            let out = a * self.source.sample(1.0, self.phase);

            self.phase += f / ctx.get_samples_per_second() as Real;
            self.phase %= 1.0;

            for i in 0..self.num_outputs {
                let mut chan = output.get_channel_mut(i);

                chan[sample] = out;
            }
        }

        Ok(())
    }
}
