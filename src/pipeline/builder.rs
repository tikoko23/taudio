use std::num::NonZero;

use crate::{
    err::AudioError,
    id::IdContainer,
    node::{
        AudioProcessor, AudioProcessorCfg, AudioSink, AudioSinkCfg, AudioSource, AudioSourceCfg,
    },
    pipeline::{NodeId, Pipeline, PipelineAudioNode, PipelineTemplate},
};

#[derive(Debug, Clone, Copy)]
pub struct NodeOutput {
    pub(crate) node: NodeId,
    pub(crate) output_index: u32,
}

pub trait NodeInputs {
    fn get_inputs(self) -> Vec<NodeOutput>;
}

impl NodeInputs for Vec<NodeOutput> {
    #[inline]
    fn get_inputs(self) -> Vec<NodeOutput> {
        self
    }
}

impl NodeInputs for &[NodeOutput] {
    #[inline]
    fn get_inputs(self) -> Vec<NodeOutput> {
        self.to_vec()
    }
}

impl<const N: usize> NodeInputs for [NodeOutput; N] {
    #[inline]
    fn get_inputs(self) -> Vec<NodeOutput> {
        self.to_vec()
    }
}

impl<const N: usize> NodeInputs for &[NodeOutput; N] {
    #[inline]
    fn get_inputs(self) -> Vec<NodeOutput> {
        self.to_vec()
    }
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct PipelineOpts {
    pub sample_rate: NonZero<u32>,
}

impl Default for PipelineOpts {
    #[inline]
    fn default() -> Self {
        PipelineOpts {
            sample_rate: NonZero::new(44100).unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct PipelineBuilder {
    pub(super) nodes: IdContainer<Vec<PipelineAudioNode>>,
    pub(super) opts: PipelineOpts,
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self {
            nodes: vec![].into(),
            opts: PipelineOpts::default(),
        }
    }
}

impl PipelineBuilder {
    /// # Panics
    /// Panics if the sample rate in the pipeline options is invalid, see
    /// [`PipelineOpts::is_valid_sample_rate`].
    #[inline]
    pub fn new(opts: PipelineOpts) -> Self {
        Self {
            nodes: vec![].into(),
            opts,
        }
    }

    #[inline]
    pub fn add_source<S: AudioSource>(&mut self, src: S) -> Result<NodeId, AudioError> {
        self.add_source_boxed(Box::new(src))
    }

    pub fn add_source_boxed(
        &mut self,
        mut src: Box<dyn AudioSource>,
    ) -> Result<NodeId, AudioError> {
        let src_info = src.setup(&AudioSourceCfg {
            sample_rate: self.opts.sample_rate.get(),
        })?;

        Ok(self.nodes.push_id(PipelineAudioNode::Source {
            node: src,
            src_info,
        }))
    }

    #[inline]
    pub fn add_processor<V, P>(&mut self, inputs: V, proc: P) -> Result<NodeId, AudioError>
    where
        V: NodeInputs,
        P: AudioProcessor,
    {
        self.add_processor_boxed(inputs, Box::new(proc))
    }

    pub fn add_processor_boxed<V>(
        &mut self,
        inputs: V,
        mut proc: Box<dyn AudioProcessor>,
    ) -> Result<NodeId, AudioError>
    where
        V: NodeInputs,
    {
        let inputs = inputs.get_inputs();

        let proc_info = proc.setup(&AudioProcessorCfg {
            sample_rate: self.opts.sample_rate.get(),
            num_inputs: inputs.len() as u32,
        })?;

        Ok(self.nodes.push_id(PipelineAudioNode::Processor {
            node: proc,
            inputs,
            proc_info,
        }))
    }

    #[inline]
    pub fn add_sink<V, S>(&mut self, inputs: V, sink: S) -> Result<NodeId, AudioError>
    where
        V: NodeInputs,
        S: AudioSink,
    {
        self.add_sink_boxed(inputs, Box::new(sink))
    }

    pub fn add_sink_boxed<V>(
        &mut self,
        inputs: V,
        mut sink: Box<dyn AudioSink>,
    ) -> Result<NodeId, AudioError>
    where
        V: NodeInputs,
    {
        let inputs = inputs.get_inputs();

        let sink_info = sink.setup(&AudioSinkCfg {
            num_inputs: inputs.len() as u32,
            sample_rate: self.opts.sample_rate.get(),
        })?;

        Ok(self.nodes.push_id(PipelineAudioNode::Sink {
            node: sink,
            inputs,
            sink_info,
        }))
    }

    #[inline]
    pub fn into_template(self) -> Result<PipelineTemplate, AudioError> {
        self.try_into()
    }

    /// Constructs a [`Pipeline`] from this builder.
    ///
    /// If you need to create copies of the same pipeline without going through [`PipelineBuilder`]
    /// every time, see [`PipelineBuilder::into_template`].
    #[inline]
    pub fn build(self) -> Result<Pipeline, AudioError> {
        self.into_template().map(|t| t.construct())
    }
}
