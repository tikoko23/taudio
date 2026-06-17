//! Graph based audio processing pipelines.
//!
//! # The Pipeline
//!
//! The [`Pipeline`] type acts as the _recipe_ to create outputs from a list of inputs, and is thus
//! immutable once created. To change options which do not affect the pipeline's layout,
//! [`PipelineTemplate`]s can be used.
//!
//! # Graph Representation
//!
//! An atomic unit of producing and processing audio samples is represented as a _node_.
//! Built-in audio sources (producers), and audio sinks (consumers) can be found
//! [here](crate::sources) and [here](crate::sinks).
//!
//! Every node can have 0 or more input and output _channels_. A channel is an independent array of
//! samples provided for the node to read from or write into. It can be thought as a plug coming
//! into or out of a node.
//!
//! The graph representation lets the user of this module to express which inputs of which nodes
//! belong to which outputs of which nodes (i.e. connects nodes together via the plugs).
//!
//! A node in a [`Pipeline`] or a [`PipelineBuilder`] can be referenced by [`NodeId`]s and
//! [`NodeHandle`]s. Such handles are returned from builder's functions (such as
//! [`PipelineBuilder::add_source`]).
//!
//! A [`NodeId`] is a type-erased handle to a node in a pipeline. Its outputs can be accessed with
//! [`NodeId::output`]. In order to access the inner node value, downcasting on
//! [`AudioNode`] and its derived traits' objects must be used.
//!
//! A [`NodeHandle`] however is a typed handle to a node in a pipeline. Its outputs can be accessed
//! in a type-safe manner using [`NodeHandle::port`]. In order to access the inner node value,
//! [`Pipeline::resolve_handle`] can be used. A [`NodeHandle`]'s type can be erased using [`NodeHandle::id`].
//!
//! An output channel of a node in a [`PipelineBuilder`] can be referenced using a [`NodeOutput`].
//! A node output can be constructed with [`NodeId::output`].
//!
//! # Sources, Processors and Sinks
//!
//! A node which has no inputs, but multiple outputs is modeled as a _source_. Such nodes will
//! implement [`AudioSource`].
//!
//! A node which has multiple inputs and multiple outputs (possibly different amounts) is modeled
//! as a _processor_. Such nodes will implement [`AudioProcessor`].
//!
//! A node which has multiple inputs but no outputs is modeled as a _sink_. Such nodes will
//! implement [`AudioSink`].
//!
//! # Example
//!
//! This section will guide you through synthesizing a sine wave and reading the samples
//! using a pipeline.
//!
//! We will use an [`Osc`] to sample a sine wave, and a [`SampleSink`] to create 16-bit samples.
//!
//! ```
//! # fn main() -> Result<(), taudio::err::AudioError> {
//! use taudio::{
//!     sources::Osc,
//!     sinks::SampleSink,
//!     waveform,
//!     sample,
//!     pipeline::PipelineBuilder,
//!     automation::{
//!         AutomationTimeline,
//!         Parameter,
//!     },
//! };
//!
//! let oscillator = Osc::new(waveform::Sine, 440.0, 1.0, 1);
//!
//! let sample_sink = SampleSink::new(sample::Int16);
//!
//! let mut builder = PipelineBuilder::default();
//!
//! let osc_handle = builder.add_source(oscillator)?;
//! let sink_handle = builder.add_sink([osc_handle.output(0)], sample_sink)?;
//!
//! let mut pipeline = builder.build()?;
//!
//! // Sample one second worth of audio.
//! pipeline.sample(44100, &AutomationTimeline::new())?;
//!
//! let sink = pipeline.resolve_handle_mut(&sink_handle);
//!
//! // Get the first channel.
//! let data = sink.take().next().unwrap();
//!
//! // ...
//! let _ = data;
//! # Ok(())
//! # }
//! ```
//!
//!
//! [`Osc`]: crate::sources::Osc
//! [`SampleSink`]: crate::sinks::SampleSink

use std::{cell::RefCell, fmt::Display, marker::PhantomData, ops::Deref};

use crate::{
    automation::AutomationTimeline,
    buffer::{AudioBuffer, SampleChannels},
    err::AudioError,
    id::{IdContainer, NumericId},
    incremental_id,
    node::{
        AudioNode, AudioProcessor, AudioProcessorInfo, AudioSink, AudioSinkInfo, AudioSource,
        AudioSourceInfo, SamplingContext,
    },
};

mod builder;
mod graph;
mod template;

pub use builder::*;
use smallvec::SmallVec;
pub use template::*;

incremental_id! {
    /// Untyped handle to a node in a [`Pipeline`].
    ///
    /// You may want to use a [`NodeHandle`] for type-safety if applicable.
    ///
    /// Reference the outputs of this node with [`NodeId::output`].
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[repr(transparent)]
    pub struct NodeId(u32) impl { NumericId };
}

impl Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08X}", self.0.get())
    }
}

incremental_id! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[repr(transparent)]
    pub(crate) struct BufferId(u32) impl { NumericId };
}

impl NodeId {
    /// Returns a handle for the given output of a node.
    pub fn output(self, n: u32) -> NodeOutput {
        NodeOutput {
            node: self,
            output_index: n,
        }
    }
}

/// Typed handle to a node in a [`Pipeline`].
///
/// You can create an untyped handle for the same node with [`NodeHandle::id`].
///
/// This is a typed wrapper around [`NodeId`] with stricter safety guarantees.
/// Concrete-typing also allows the use of compile-time checked and more ergonomic
/// node output access. See [`NodeHandle::port`] for more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeHandle<T: AudioNode> {
    id: NodeId,
    _marker: PhantomData<T>,
}

impl<T: AudioNode> Deref for NodeHandle<T> {
    type Target = NodeId;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl<T: AudioNode> NodeHandle<T> {
    #[inline]
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Get a handle to an output of this node.
    ///
    /// This is a type safe variant of [`NodeId::output`].
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # use taudio::{sources::{Osc, OscPort}, waveform, pipeline::{PipelineBuilder, PipelineOpts}};
    /// #
    /// let osc = Osc::new(waveform::Sine, 440.0, 1.0, 2);
    ///
    /// let mut builder = PipelineBuilder::new(PipelineOpts::default());
    ///
    /// let osc_handle = builder.add_source(osc)?;
    ///
    /// let osc_output_left = osc_handle.port(OscPort::Output(0));
    /// let osc_output_right = osc_handle.port(OscPort::Output(1));
    /// #
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn port(&self, output: impl IntoNodeOutputIndex<T>) -> NodeOutput {
        self.output(output.into_node_output_index())
    }
}

/// Description of how a port is mapped to a channel index.
pub trait IntoNodeOutputIndex<T: AudioNode> {
    /// Maps the represented port of the value to a channel index.
    ///
    /// See [`NodeHandle::port`] for details.
    fn into_node_output_index(self) -> u32;
}

#[derive(Debug, Clone)]
pub(crate) enum PipelineAudioNode {
    Source {
        id: NodeId,
        node: Box<dyn AudioSource>,
        src_info: AudioSourceInfo,
    },
    Processor {
        id: NodeId,
        node: Box<dyn AudioProcessor>,
        inputs: Vec<NodeOutput>,
        proc_info: AudioProcessorInfo,
    },
    Sink {
        id: NodeId,
        node: Box<dyn AudioSink>,
        inputs: Vec<NodeOutput>,
        #[allow(unused)]
        sink_info: AudioSinkInfo,
    },
}

impl PipelineAudioNode {
    pub fn as_common(&self) -> &dyn AudioNode {
        match self {
            Self::Source { node, .. } => node.as_ref(),
            Self::Processor { node, .. } => node.as_ref(),
            Self::Sink { node, .. } => node.as_ref(),
        }
    }

    pub fn as_common_mut(&mut self) -> &mut dyn AudioNode {
        match self {
            Self::Source { node, .. } => node.as_mut(),
            Self::Processor { node, .. } => node.as_mut(),
            Self::Sink { node, .. } => node.as_mut(),
        }
    }

    pub fn inputs(&self) -> &[NodeOutput] {
        match self {
            Self::Source { .. } => &[],
            Self::Processor { inputs, .. } => inputs,
            Self::Sink { inputs, .. } => inputs,
        }
    }

    pub fn id(&self) -> NodeId {
        match self {
            Self::Source { id, .. } => *id,
            Self::Processor { id, .. } => *id,
            Self::Sink { id, .. } => *id,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BufferAssignment {
    pub(crate) inputs: SmallVec<[BufferId; 16]>,
    pub(crate) outputs: SmallVec<[BufferId; 16]>,
}

/// Represents a complete, instantiated and ready to use audio processing pipeline.
///
/// [`PipelineBuilder`] can be used to create [`Pipeline`]s.
#[derive(Debug)]
pub struct Pipeline {
    buffers: IdContainer<Vec<RefCell<AudioBuffer>>>,
    opts: PipelineOpts,
    nodes: Vec<PipelineAudioNode>,
    id_to_node_index: IdContainer<Vec<usize>>,
    buffer_assignments: Vec<BufferAssignment>,
    output_bufs: Vec<BufferId>,
    current_sample_offset: u64,
}

impl From<PipelineTemplate> for Pipeline {
    fn from(template: PipelineTemplate) -> Self {
        let n_bufs = template.num_buffers;
        let mut buffers = IdContainer::new(Vec::with_capacity(n_bufs));
        let sample_count = template.opts.sample_rate.get() as usize;

        for _ in 0..n_bufs {
            let buffer = unsafe { AudioBuffer::new_uninit(sample_count) };
            buffers.push(RefCell::new(buffer));
        }

        let mut unordered_nodes: IdContainer<_> = template
            .nodes
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>()
            .into();

        let ordered_nodes: Vec<_> = template
            .topological_order
            .into_iter()
            .map(|id| unordered_nodes[id].take().unwrap())
            .collect();

        let mut id_to_node_index = IdContainer::new(vec![usize::MAX; unordered_nodes.len()]);

        for (i, node) in ordered_nodes.iter().enumerate() {
            id_to_node_index[node.id()] = i;
        }

        Self {
            opts: template.opts,
            buffer_assignments: template.buffer_assignments,
            output_bufs: vec![],
            nodes: ordered_nodes,
            current_sample_offset: 0,
            id_to_node_index,
            buffers,
        }
    }
}

impl Pipeline {
    pub fn sources(&self) -> impl Iterator<Item = (NodeId, &dyn AudioSource)> {
        self.nodes.iter().filter_map(|node| match node {
            PipelineAudioNode::Source { node, id, .. } => Some((*id, node.as_ref())),
            _ => None,
        })
    }

    pub fn sinks(&self) -> impl Iterator<Item = (NodeId, &dyn AudioSink)> {
        self.nodes.iter().filter_map(|node| match node {
            PipelineAudioNode::Sink { id, node, .. } => Some((*id, node.as_ref())),
            _ => None,
        })
    }

    pub fn sources_mut(&mut self) -> impl Iterator<Item = (NodeId, &mut dyn AudioSource)> {
        self.nodes.iter_mut().filter_map(|node| match node {
            PipelineAudioNode::Source { node, id, .. } => Some((*id, node.as_mut())),
            _ => None,
        })
    }

    pub fn sinks_mut(&mut self) -> impl Iterator<Item = (NodeId, &mut dyn AudioSink)> {
        self.nodes.iter_mut().filter_map(|node| match node {
            PipelineAudioNode::Sink { id, node, .. } => Some((*id, node.as_mut())),
            _ => None,
        })
    }

    pub fn get_node(&self, id: NodeId) -> Option<&dyn AudioNode> {
        let idx = *self.id_to_node_index.get(id.as_index())?;

        Some(self.nodes[idx].as_common())
    }

    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut dyn AudioNode> {
        let idx = *self.id_to_node_index.get(id.as_index())?;

        Some(self.nodes[idx].as_common_mut())
    }

    pub fn get_source(&self, id: NodeId) -> Option<&dyn AudioSource> {
        let idx = *self.id_to_node_index.get(id.as_index())?;

        match &self.nodes[idx] {
            PipelineAudioNode::Source { node, .. } => Some(node.as_ref()),
            _ => None,
        }
    }

    pub fn get_source_mut(&mut self, id: NodeId) -> Option<&mut dyn AudioSource> {
        let idx = *self.id_to_node_index.get(id.as_index())?;

        match &mut self.nodes[idx] {
            PipelineAudioNode::Source { node, .. } => Some(node.as_mut()),
            _ => None,
        }
    }

    pub fn get_processor(&self, id: NodeId) -> Option<&dyn AudioProcessor> {
        let idx = *self.id_to_node_index.get(id.as_index())?;

        match &self.nodes[idx] {
            PipelineAudioNode::Processor { node, .. } => Some(node.as_ref()),
            _ => None,
        }
    }

    pub fn get_processor_mut(&mut self, id: NodeId) -> Option<&mut dyn AudioProcessor> {
        let idx = *self.id_to_node_index.get(id.as_index())?;

        match &mut self.nodes[idx] {
            PipelineAudioNode::Processor { node, .. } => Some(node.as_mut()),
            _ => None,
        }
    }

    pub fn get_sink(&self, id: NodeId) -> Option<&dyn AudioSink> {
        let idx = *self.id_to_node_index.get(id.as_index())?;

        match &self.nodes[idx] {
            PipelineAudioNode::Sink { node, .. } => Some(node.as_ref()),
            _ => None,
        }
    }

    pub fn get_sink_mut(&mut self, id: NodeId) -> Option<&mut dyn AudioSink> {
        let idx = *self.id_to_node_index.get(id.as_index())?;

        match &mut self.nodes[idx] {
            PipelineAudioNode::Sink { node, .. } => Some(node.as_mut()),
            _ => None,
        }
    }

    /// Returns a reference to a node identified by a typed node handle.
    ///
    /// # Panics
    /// Panics if the given handle does not point to a valid node or the handle's type
    /// does not match with the node's type. Note that this is not possible unless handles
    /// from different pipelines are mixed.
    pub fn resolve_handle<T: AudioNode>(&self, handle: &NodeHandle<T>) -> &T {
        self.get_node(handle.id())
            .expect("node with the given handle does not exist")
            .downcast_ref()
            .expect("handle type doesn't match node type")
    }

    /// Returns a reference to a node identified by a typed node handle.
    ///
    /// # Panics
    /// Panics if the given handle does not point to a valid node or the handle's type
    /// does not match with the node's type. Note that this is not possible unless handles
    /// from different pipelines are mixed.
    pub fn resolve_handle_mut<T: AudioNode>(&mut self, handle: &NodeHandle<T>) -> &mut T {
        self.get_node_mut(handle.id())
            .expect("node with the given handle does not exist")
            .downcast_mut()
            .expect("handle type doesn't match the node type")
    }

    /// Samples from the pipeline.
    ///
    /// If the given automation timeline differs in sucessive calls, its a logic error.
    /// Pass in a default timeline to ignore automations.
    ///
    /// # Panics
    /// Panics if the requested sample count is greater than the pipeline sample count.
    pub fn sample(
        &mut self,
        n_samples: u32,
        automations: &AutomationTimeline,
    ) -> Result<(), AudioError> {
        if self.opts.sample_rate.get() < n_samples {
            panic!("number of requested samples exceeded pipeline sample rate");
        }

        self.output_bufs.clear();

        for (i, node) in self.nodes.iter_mut().enumerate() {
            let assignment = &self.buffer_assignments[i];

            let input_buffers = SampleChannels {
                num_samples: n_samples as usize,
                buffers: &self.buffers,
                channels: &assignment.inputs,
            };

            let mut output_buffers = SampleChannels {
                num_samples: n_samples as usize,
                buffers: &self.buffers,
                channels: &assignment.outputs,
            };

            let context = SamplingContext {
                sample_rate: self.opts.sample_rate.get(),
                batch_begin: self.current_sample_offset,
                num_samples: n_samples,
                automations,
            };

            match node {
                PipelineAudioNode::Source {
                    node,
                    src_info: _,
                    id: _,
                } => node.sample(&context, &mut output_buffers)?,

                PipelineAudioNode::Processor {
                    node,
                    inputs: _,
                    proc_info: _,
                    id: _,
                } => node.sample(&context, &input_buffers, &mut output_buffers)?,

                PipelineAudioNode::Sink {
                    node,
                    inputs: _,
                    sink_info: _,
                    id: _,
                } => node.sample(&context, &input_buffers)?,
            }
        }

        self.current_sample_offset += n_samples as u64;

        Ok(())
    }
}
