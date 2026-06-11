use std::num::NonZero;

use crate::{
    err::AudioError,
    id::IdContainer,
    node::{
        AudioProcessor, AudioProcessorCfg, AudioSink, AudioSinkCfg, AudioSource, AudioSourceCfg,
    },
    pipeline::{NodeId, Pipeline, PipelineAudioNode, PipelineTemplate},
};

/// Handle to a node's specific output channel.
#[derive(Debug, Clone, Copy)]
pub struct NodeOutput {
    pub(crate) node: NodeId,
    pub(crate) output_index: u32,
}

/// Defines how a value must be converted to a list of [`NodeOutput`]s.
///
/// The returned value is owned because the [`PipelineBuilder`] needs to own them.
/// This prevents redundant clones and allocations in some cases (see the [`NodeInputs`] implementation for [`Vec`]).
pub trait NodeInputs {
    /// Converts the type into a list of node ouputs.
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

/// Represents the configurable aspects which don't depend on the nodes of a pipeline.
///
/// See [`PipelineTemplate`] to learn why this was created.
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
    #[inline]
    pub fn new(opts: PipelineOpts) -> Self {
        Self {
            nodes: vec![].into(),
            opts,
        }
    }

    /// Adds a source to the pipeline.
    ///
    /// Consider using [`PipelineBuilder::add_source_boxed`] if your source is already heap-allocated,
    /// or is a trait object.
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

        let id = self.nodes.next_id();

        Ok(self.nodes.push_id(PipelineAudioNode::Source {
            node: src,
            src_info,
            id,
        }))
    }

    /// Adds a processor to the pipeline.
    ///
    /// Consider using [`PipelineBuilder::add_processor_boxed`] if your processor is already heap-allocated,
    /// or is a trait object.
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
            num_inputs: inputs.len(),
        })?;

        let id = self.nodes.next_id();

        Ok(self.nodes.push_id(PipelineAudioNode::Processor {
            node: proc,
            inputs,
            proc_info,
            id,
        }))
    }

    /// Adds a sink to the pipeline.
    ///
    /// Consider using [`PipelineBuilder::add_sink_boxed`] if your sink is already heap-allocated,
    /// or is a trait object.
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
            num_inputs: inputs.len(),
            sample_rate: self.opts.sample_rate.get(),
        })?;

        let id = self.nodes.next_id();

        Ok(self.nodes.push_id(PipelineAudioNode::Sink {
            node: sink,
            inputs,
            sink_info,
            id,
        }))
    }

    /// Converts this builder into a template which can be reinstantiated multiple times
    /// with different [`PipelineOpts`].
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
