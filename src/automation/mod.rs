//! Dynamic parameters changing over time.
//!
//! # Automation
//! An _automation_ is a parameter of an audio processing step which changes over time
//! according to pre-determined instructions. The parameters' value can be within `0.0..=1.0`.
//!
//! A single automation's values over time are determined by an [`AutomationTrack`].
//! An automation track consists of [`AutomationClip`]s with specific start / end times.
//!
//! Multiple tracks can be grouped together using an [`AutomationTimeline`]. This is necessary
//! to use automations with the [pipeline API](crate::pipeline).
//!
//! Audio nodes can query different automations with [`SamplingContext::automations`] while
//! sampling. It's up to the node implementation to make sense of the normalized value, although
//! this is idiomatically controlled by parameter mappings (see [`Parameter`] and [`Mapping`]).
//!
//! [`SamplingContext::automations`]: crate::node::SamplingContext::automations

mod clip;
mod parameter;
mod timeline;
mod track;

pub use clip::*;
pub use parameter::*;
pub use timeline::*;
pub use track::*;
