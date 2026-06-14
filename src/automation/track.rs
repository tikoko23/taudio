use std::ops::Range;

use crate::automation::AutomationClip;

#[derive(Debug, Clone)]
struct ClipData {
    clip: AutomationClip,
    range: Range<u64>,
}

fn sample_clip(clip: &ClipData, offset: u64) -> f32 {
    debug_assert!(offset >= clip.range.start);

    let local = offset - clip.range.start;

    debug_assert!(local <= u32::MAX.into());

    clip.clip.sample(local as u32)
}

/// Groups automation clips of one parameter ordered by time.
///
/// # Constraints
/// The [`AutomationTrack`] structure employs some constraints on the added clips
/// which enables it to keep fast lookup times.
///
/// These constraints are enforced by the public API.
///
/// ## Overlaps
/// Clips stored in this track are constrained to not have any overlap.
///
/// ## Time Ordering
/// All clips must be in ascending time order. That is for all clips except the first,
/// the last sample offest occupied by the clip before them must be earlier than its own
/// starting offset.
#[derive(Debug, Clone)]
pub struct AutomationTrack {
    clips: Vec<ClipData>,
}

impl Default for AutomationTrack {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl AutomationTrack {
    #[inline]
    pub fn new() -> Self {
        Self { clips: vec![] }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.clips.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.clips.len()
    }

    /// # Panics
    /// Panics if the index is out of range.
    #[inline]
    pub fn remove_clip(&mut self, index: usize) -> AutomationClip {
        self.clips.remove(index).clip
    }

    /// Returns an iterator over the clips in this track and their occupied ranges.
    pub fn clips(&self) -> impl Iterator<Item = (Range<u64>, &AutomationClip)> {
        self.clips.iter().map(|c| (c.range.clone(), &c.clip))
    }

    /// Adds a new clip to the end of the track.
    ///
    /// # Panics
    /// Panics if the new track violates the [overlap constraint](AutomationTrack#overlaps).
    /// Panics if the new track violates the [time ordering constraint](AutomationTrack#time-ordering).
    pub fn add_clip(
        &mut self,
        clip: AutomationClip,
        begin_sample_offset: u64,
        duration_in_samples: u64,
    ) {
        if let Some(last) = self.clips.last() {
            assert!(
                last.range.end <= begin_sample_offset,
                "clips must not overlap nor be out of time order"
            );
        }

        let end = begin_sample_offset + duration_in_samples;

        self.clips.push(ClipData {
            clip,
            range: begin_sample_offset..end,
        });
    }

    /// Returns the clip in which the given sample offset lies, or [`None`].
    pub fn query_clip(&self, sample_offset: u64) -> Option<&AutomationClip> {
        let split_index = self
            .clips
            .partition_point(|c| c.range.start > sample_offset);

        if split_index >= self.clips.len() {
            return None;
        }

        let candidate = &self.clips[split_index];

        if candidate.range.contains(&sample_offset) {
            Some(&candidate.clip)
        } else {
            None
        }
    }

    /// Queries the value of the track at the given sample offset.
    ///
    /// If there are no clips, 0 is returned.
    ///
    /// If the queried sample offset is earlier than the first clip, the first value
    /// of the first clip is returned. Otherwise, the last value of the pervious clip
    /// is returned. Refer to the diagram below for an example situation.
    ///
    /// ```text
    /// Time (t):   10           20           30           40           50
    ///             .------------.            .------------.
    ///             |   Clip A   |            |   Clip B   |
    ///           | '------------'  |         '------------'     |
    ///           ^         ^       ^                ^           ^
    ///           Q0        Q1      Q2               Q3          Q4
    /// ```
    ///
    /// - `Q0` returns the value at `t = 10`.
    /// - `Q1` returns the corresponding value in Clip A.
    /// - `Q2` returns the value at `t = 20`.
    /// - `Q3` returns the corresponding value in Clip B.
    /// - `Q4` returns the value at `t = 40`.
    pub fn query_value(&self, sample_offset: u64) -> f32 {
        let Some(last) = self.clips.last() else {
            return 0.0;
        };

        let clip_index = self
            .clips
            .partition_point(|c| c.range.start > sample_offset);

        if clip_index >= self.clips.len() {
            sample_clip(last, sample_offset)
        } else {
            let clip = &self.clips[clip_index];

            sample_clip(clip, sample_offset)
        }
    }
}
