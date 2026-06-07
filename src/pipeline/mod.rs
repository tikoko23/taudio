use std::cell::RefCell;

use crate::{
    buffer::{AudioBuffer, SampleChannels},
    err::AudioError,
    id::IdContainer,
    incremental_id,
    node::{
        AudioNodeCommon, AudioProcessor, AudioProcessorInfo, AudioSink, AudioSinkInfo, AudioSource,
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
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[repr(transparent)]
    pub struct NodeId(u32) impl { NumericId };
}

incremental_id! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[repr(transparent)]
    pub(crate) struct BufferId(u32) impl { NumericId };
}

impl NodeId {
    pub fn output(self, n: u32) -> NodeOutput {
        NodeOutput {
            node: self,
            output_index: n,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum PipelineAudioNode {
    Source {
        node: Box<dyn AudioSource>,
        src_info: AudioSourceInfo,
    },
    Processor {
        node: Box<dyn AudioProcessor>,
        inputs: Vec<NodeOutput>,
        proc_info: AudioProcessorInfo,
    },
    Sink {
        node: Box<dyn AudioSink>,
        inputs: Vec<NodeOutput>,
        #[allow(unused)]
        sink_info: AudioSinkInfo,
    },
}

impl PipelineAudioNode {
    pub fn as_common(&self) -> &dyn AudioNodeCommon {
        match self {
            Self::Source { node, .. } => node.as_ref(),
            Self::Processor { node, .. } => node.as_ref(),
            Self::Sink { node, .. } => node.as_ref(),
        }
    }

    pub fn inputs(&self) -> &[NodeOutput] {
        match self {
            Self::Source { .. } => &[],
            Self::Processor { inputs, .. } => inputs,
            Self::Sink { inputs, .. } => inputs,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BufferAssignment {
    pub(crate) inputs: SmallVec<[BufferId; 16]>,
    pub(crate) outputs: SmallVec<[BufferId; 16]>,
}

#[derive(Debug)]
pub struct Pipeline {
    buffers: IdContainer<Vec<RefCell<AudioBuffer>>>,
    opts: PipelineOpts,
    nodes: Vec<PipelineAudioNode>,
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

        let ordered_nodes = template
            .topological_order
            .into_iter()
            .map(|id| unordered_nodes[id].take().unwrap())
            .collect();

        Self {
            opts: template.opts,
            buffer_assignments: template.buffer_assignments,
            output_bufs: vec![],
            nodes: ordered_nodes,
            current_sample_offset: 0,
            buffers,
        }
    }
}

impl Pipeline {
    /// # Panics
    /// Panics if the requested sample count is greater than the pipeline sample count.
    pub fn sample(&mut self, n_samples: u32) -> Result<(), AudioError> {
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
            };

            match node {
                PipelineAudioNode::Source { node, src_info: _ } => {
                    node.sample(&context, &mut output_buffers)?
                }

                PipelineAudioNode::Processor {
                    node,
                    inputs: _,
                    proc_info: _,
                } => node.sample(&context, &input_buffers, &mut output_buffers)?,

                PipelineAudioNode::Sink {
                    node,
                    inputs: _,
                    sink_info: _,
                } => node.sample(&context, &input_buffers)?,
            }
        }

        Ok(())
    }
}
