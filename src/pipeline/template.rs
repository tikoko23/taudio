use std::{collections::HashSet, num::NonZeroU32};

use smallvec::smallvec;

use crate::{
    err::AudioError,
    id::{IdContainer, IncrementalId, NumericId},
    pipeline::{
        BufferAssignment, BufferId, NodeId, Pipeline, PipelineAudioNode, PipelineBuilder,
        PipelineOpts, graph::Graph,
    },
};

/// Represents a pipeline graph which has been checked to be valid. [`Pipeline`] instances can be
/// constructed using [`From`].
///
/// This allows reusing the same pipeline structure with different [`PipelineOpts`].
#[derive(Debug, Clone)]
pub struct PipelineTemplate {
    pub(crate) opts: PipelineOpts,
    pub(crate) topological_order: Vec<NodeId>,
    pub(crate) nodes: IdContainer<Vec<PipelineAudioNode>>,
    pub(crate) buffer_assignments: Vec<BufferAssignment>,
    pub(crate) num_buffers: usize,
}

impl PipelineTemplate {
    #[inline]
    pub fn try_from_builder(b: PipelineBuilder) -> Result<Self, AudioError> {
        b.try_into()
    }

    /// Constructs a pipeline from this template.
    ///
    /// If you want to reuse the template, consider calling [`PipelineTemplate::construct`]
    /// on a [`Clone`]d value
    #[inline]
    pub fn construct(self) -> Pipeline {
        self.into()
    }

    #[inline]
    pub fn get_opts(&self) -> &PipelineOpts {
        &self.opts
    }

    #[inline]
    pub fn set_opts(&mut self, opts: PipelineOpts) {
        self.opts = opts;
    }
}

impl TryFrom<PipelineBuilder> for PipelineTemplate {
    type Error = AudioError;

    fn try_from(builder: PipelineBuilder) -> Result<Self, Self::Error> {
        let mut edges = vec![];
        let mut num_outputs = IdContainer::new(vec![0; builder.nodes.len()]);

        for (id, node) in builder.nodes.iter_with_id() {
            for out in node.inputs() {
                num_outputs[out.node] += 1;
                edges.push((out.node, id));
            }
        }

        for (id, node) in builder.nodes.iter_with_id::<NodeId>() {
            let expected_outputs = match node {
                PipelineAudioNode::Source { src_info, .. } => src_info.num_outputs,
                PipelineAudioNode::Processor { proc_info, .. } => proc_info.num_outputs,
                PipelineAudioNode::Sink { .. } => 0,
            };

            if expected_outputs != num_outputs[id] {
                return Err(AudioError::MismatchedChannels {
                    node_name: node.as_common().name().into(),
                    got: num_outputs[id],
                    expected: expected_outputs,
                });
            }
        }

        let graph = Graph::from_edges(edges);

        // SAFETY: The current builder API doesn't allow cycles because
        // each node's inputs are submitted before a handle to the node
        // is given to the caller and inputs cannot be modified later.
        let topological_order = unsafe { graph.toposort_acyclic() };

        let result = assign_buffers(&graph, &topological_order, &num_outputs, &builder.nodes);

        Ok(PipelineTemplate {
            nodes: builder.nodes,
            opts: builder.opts,
            buffer_assignments: result.assignments,
            num_buffers: result.num_buffers,
            topological_order,
        })
    }
}

struct AssignmentResult {
    assignments: Vec<BufferAssignment>,
    num_buffers: usize,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Interval {
    Start {
        from_node: NodeId,
        to_node: NodeId,
        output_index: u32,
        input_index: u32,
    },
    End {
        from_node: NodeId,
        output_index: u32,
    },
}

/// Simulates sequential buffer allocations by assining a [`BufferId`] (which would normally be a
/// pointer) for each one.
#[derive(Debug)]
struct BufferIdAllocator {
    unused: HashSet<BufferId>,
    free: BufferId,
}

impl BufferIdAllocator {
    fn new() -> Self {
        Self {
            unused: HashSet::new(),
            free: BufferId::FIRST,
        }
    }

    fn alloc(&mut self) -> BufferId {
        if let Some(&reused) = self.unused.iter().next() {
            self.unused.remove(&reused);
            return reused;
        }

        let id = self.free;

        self.free = id.next();

        id
    }

    fn free(&mut self, id: BufferId) {
        self.unused.insert(id);
    }
}

/// Assigns buffers for node inputs/outputs.
///
/// # Details
/// This works very much like the classic "maximum intervals" problem
/// but we do some extra book-keeping in [`BufferIdAllocator`] to solve this.
fn assign_buffers(
    graph: &Graph,
    topological_order: &[NodeId],
    num_outputs: &IdContainer<Vec<usize>>,
    nodes: &IdContainer<Vec<PipelineAudioNode>>,
) -> AssignmentResult {
    let mut node_to_topo_index = IdContainer::new(vec![0; graph.len()]);

    for (index, &node) in topological_order.iter().enumerate() {
        node_to_topo_index[node] = index;
    }

    let mut intervals = Vec::with_capacity(graph.count_edges());

    for (start_time, &node) in topological_order.iter().enumerate() {
        for next_node in graph.neighbors(node) {
            let inputs = nodes[next_node].inputs();

            let input_index = inputs
                .iter()
                .position(|x| x.node == node)
                .expect("input must exist");

            intervals.push((
                start_time,
                Interval::Start {
                    from_node: node,
                    to_node: next_node,
                    output_index: inputs[input_index].output_index,
                    input_index: input_index as u32,
                },
            ));

            intervals.push((
                node_to_topo_index[next_node],
                Interval::End {
                    from_node: node,
                    output_index: inputs[input_index].output_index,
                },
            ));
        }
    }

    intervals.sort_unstable();

    let mut buffers = IdContainer::new(Vec::with_capacity(nodes.len()));

    for (id, node) in nodes.iter_with_id::<NodeId>() {
        let n_inputs = node.inputs().len();
        let n_outputs = num_outputs[id];

        let dummy_assignment = BufferAssignment {
            inputs: smallvec![BufferId(NonZeroU32::MAX); n_inputs],
            outputs: smallvec![BufferId(NonZeroU32::MAX); n_outputs],
        };

        buffers.push(dummy_assignment);
    }

    let mut buf_alloc = BufferIdAllocator::new();

    for (_, interval) in intervals {
        match interval {
            Interval::Start {
                from_node,
                to_node,
                output_index,
                input_index,
            } => {
                let buf_id = buf_alloc.alloc();

                buffers[from_node].outputs[output_index as usize] = buf_id;
                buffers[to_node].inputs[input_index as usize] = buf_id;
            }
            Interval::End {
                from_node,
                output_index,
            } => {
                let buf_id = buffers[from_node].outputs[output_index as usize];

                buf_alloc.free(buf_id);
            }
        }
    }

    let num_buffers = buf_alloc.free.as_index();

    debug_assert_eq!(buf_alloc.unused.len(), num_buffers);

    AssignmentResult {
        assignments: buffers.into_inner(),
        num_buffers,
    }
}

#[cfg(test)]
mod test {
    use crate::{
        err::AudioError,
        id::NumericId,
        node::{
            AudioNode, AudioProcessor, AudioProcessorCfg, AudioProcessorInfo, AudioSink,
            AudioSinkCfg, AudioSinkInfo, AudioSource, AudioSourceCfg, AudioSourceInfo,
            SamplingContext,
        },
        pipeline::{BufferId, PipelineBuilder, PipelineOpts},
    };

    #[derive(Debug, Clone)]
    struct TestSource(usize);

    #[derive(Debug, Clone)]
    struct TestProcessor(usize, usize);

    #[derive(Debug, Clone)]
    struct TestSink(usize);

    impl AudioNode for TestSource {
        fn name(&self) -> &str {
            "@test:source"
        }
    }

    impl AudioNode for TestProcessor {
        fn name(&self) -> &str {
            "@test:processor"
        }
    }

    impl AudioNode for TestSink {
        fn name(&self) -> &str {
            "@test:sink"
        }
    }

    impl AudioSource for TestSource {
        fn setup(&mut self, cfg: &AudioSourceCfg) -> Result<AudioSourceInfo, AudioError> {
            let _ = cfg;

            Ok(AudioSourceInfo {
                num_outputs: self.0,
            })
        }

        fn sample(
            &mut self,
            _ctx: &SamplingContext,
            _output: &mut crate::buffer::SampleChannels<'_>,
        ) -> Result<(), AudioError> {
            Ok(())
        }
    }

    impl AudioProcessor for TestProcessor {
        fn setup(&mut self, cfg: &AudioProcessorCfg) -> Result<AudioProcessorInfo, AudioError> {
            AudioError::expect_channels(self.0..=self.0, cfg.num_inputs)?;

            Ok(AudioProcessorInfo {
                num_outputs: self.1,
            })
        }

        fn sample(
            &mut self,
            _ctx: &SamplingContext,
            _input: &crate::buffer::SampleChannels<'_>,
            _output: &mut crate::buffer::SampleChannels<'_>,
        ) -> Result<(), AudioError> {
            Ok(())
        }
    }

    impl AudioSink for TestSink {
        fn setup(&mut self, cfg: &AudioSinkCfg) -> Result<AudioSinkInfo, AudioError> {
            AudioError::expect_channels(self.0..=self.0, cfg.num_inputs)?;

            Ok(AudioSinkInfo {})
        }

        fn sample(
            &mut self,
            _ctx: &SamplingContext,
            _input: &crate::buffer::SampleChannels<'_>,
        ) -> Result<(), AudioError> {
            Ok(())
        }
    }

    macro_rules! bid {
        ($n:literal) => {
            BufferId::from_index($n - 1)
        };
    }

    #[test]
    fn linear_ok() -> Result<(), AudioError> {
        let opts = PipelineOpts {
            sample_rate: 44100.try_into().unwrap(),
            ..Default::default()
        };

        let mut builder = PipelineBuilder::new(opts);

        let source = builder.add_source(TestSource(1))?;
        let proc = builder.add_processor([source.output(0)], TestProcessor(1, 1))?;
        let _ = builder.add_sink([proc.output(0)], TestSink(1))?;

        let template = builder.into_template()?;

        eprintln!("{template:#?}");

        assert_eq!(template.opts.sample_rate, 44100.try_into().unwrap());
        assert_eq!(template.num_buffers, 2);

        assert_eq!(template.buffer_assignments[0].inputs.as_slice(), &[]);
        assert_eq!(
            template.buffer_assignments[0].outputs.as_slice(),
            &[bid!(1)]
        );

        assert_eq!(template.buffer_assignments[1].inputs.as_slice(), &[bid!(1)]);
        assert_eq!(
            template.buffer_assignments[1].outputs.as_slice(),
            &[bid!(2)]
        );

        assert_eq!(template.buffer_assignments[2].inputs.as_slice(), &[bid!(2)]);
        assert_eq!(template.buffer_assignments[2].outputs.as_slice(), &[]);

        Ok(())
    }
}
