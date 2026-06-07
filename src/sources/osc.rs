use std::fmt::Debug;

use crate::{
    Real,
    buffer::SampleChannels,
    dupe::Dupe,
    err::AudioError,
    node::{AudioNode, AudioSource, AudioSourceCfg, AudioSourceInfo, SamplingContext},
    waveform::WaveSource,
};

#[derive(Debug, Clone)]
pub struct Osc<W: WaveSource + Debug + Clone + 'static> {
    source: W,
    freq: Real,
    amplitude: Real,
    num_outputs: usize,
}

impl<W: WaveSource + Debug + Clone> Dupe for Osc<W> {
    fn dupe(&self) -> Option<Self> {
        Some(self.clone())
    }
}

impl<W: WaveSource + Debug + Clone + 'static> Osc<W> {
    #[inline]
    pub fn new(source: W, freq: Real, amplitude: Real, num_outputs: usize) -> Self {
        Self {
            source,
            freq,
            amplitude,
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
            let a = self.amplitude * self.source.sample(self.freq, t);

            for i in 0..self.num_outputs {
                let mut chan = output.get_channel_mut(i);

                chan[sample] = a;
            }
        }

        Ok(())
    }
}
