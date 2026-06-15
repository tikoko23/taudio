use std::ops::Range;

use crate::automation::AutomationClip;

#[derive(Debug, Clone)]
struct ClipData {
    clip: AutomationClip,
    range: Range<u64>,
}

impl ClipData {
    pub fn duration(&self) -> u64 {
        self.range.end - self.range.start
    }
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

impl FromIterator<(Range<u64>, AutomationClip)> for AutomationTrack {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (Range<u64>, AutomationClip)>,
    {
        let mut track = AutomationTrack::new();

        let iter = iter.into_iter();

        track.clips.reserve(iter.size_hint().0);

        for (range, clip) in iter {
            track.add_clip(clip, range.start, range.end - range.start);
        }

        track
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

    /// Returns an iterator over the clips in this track and their occupied ranges.
    pub fn clips(&self) -> impl Iterator<Item = (Range<u64>, &AutomationClip)> {
        self.clips.iter().map(|c| (c.range.clone(), &c.clip))
    }

    /// Returns an iterator over the clips in this track and their occupied ranges.
    ///
    /// Mutating the clip objects is safe because the occupied range is not determined by
    /// the clip itself.
    pub fn clips_mut(&mut self) -> impl Iterator<Item = (Range<u64>, &mut AutomationClip)> {
        self.clips
            .iter_mut()
            .map(|c| (c.range.clone(), &mut c.clip))
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

    /// # Panics
    /// Panics if the index is out of range.
    #[inline]
    pub fn remove_clip(&mut self, index: usize) -> AutomationClip {
        self.clips.remove(index).clip
    }

    /// Removes the last clip from the end of the track.
    #[inline]
    pub fn pop_clip(&mut self) -> Option<AutomationClip> {
        self.clips.pop().map(|c| c.clip)
    }

    /// Resizes a clip from the end (i.e. moves the endpoint).
    ///
    /// # Panics
    /// Panics if the index is out of range.
    /// Panics if the [overlap constraint](AutomationClip#overlaps) is violated.
    pub fn resize_clip(&mut self, index: usize, new_duration: u64) {
        let next_clip = self.clips.get_mut(index + 1);

        match next_clip {
            Some(next_clip) => {
                let next_start = next_clip.range.start;
                let range = &mut self.clips[index].range;

                if range.start + new_duration > next_start {
                    panic!(
                        "new range would overlap with the next (would end at {end}, next starts at {next_start})",
                        end = range.start + new_duration
                    );
                }

                range.end = range.start + new_duration;
            }
            None => {
                let range = &mut self.clips[index].range;
                range.end = range.start + new_duration;
            }
        }
    }

    /// Repositions a clip from its starting point.
    ///
    /// # Panics
    /// Panics if the index is out of range.
    /// Panics if the [overlap constraint](AutomationClip#overlaps) is violated.
    pub fn reposition_clip(&mut self, index: usize, new_sample_offset: u64) {
        let new_position = self
            .clips
            .partition_point(|c| c.range.end <= new_sample_offset);

        if let Some(next) = self.clips.get(new_position + 1) {
            let this_clip = &self.clips[index];

            assert!(
                next.range.start > new_sample_offset + this_clip.duration(),
                "new position would overlap with the next clip"
            );

            let correction = new_position > index;

            let mut clip = self.clips.remove(index);
            clip.range = new_sample_offset..(new_sample_offset + clip.duration());

            self.clips.insert(new_position - correction as usize, clip);
        } else {
            let mut clip = self.clips.remove(index);
            clip.range = new_sample_offset..(new_sample_offset + clip.duration());

            self.clips.push(clip);
        }
    }

    /// Returns the clip in which the given sample offset lies, or [`None`].
    pub fn query_clip(&self, sample_offset: u64) -> Option<&AutomationClip> {
        let split_index = self.clips.partition_point(|c| c.range.end < sample_offset);

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

        let clip_index = self.clips.partition_point(|c| c.range.end < sample_offset);

        if clip_index >= self.clips.len() {
            return sample_clip(last, sample_offset);
        }

        let clip = &self.clips[clip_index];

        if clip.range.start > sample_offset {
            todo!()
        }

        dbg!(sample_offset);
        dbg!(clip_index);

        sample_clip(clip, sample_offset)
    }
}

#[cfg(test)]
mod test {
    use std::num::NonZero;

    use crate::automation::{Lfo, LfoShape};

    use super::*;

    fn make_clip() -> AutomationClip {
        Lfo::new(LfoShape::Saw, NonZero::new(64).unwrap()).into()
    }

    #[test]
    fn new_and_empty() {
        let track = AutomationTrack::new();
        assert!(track.is_empty());
        assert_eq!(track.len(), 0);
    }

    #[test]
    fn push_clip_success() {
        let mut track = AutomationTrack::new();

        assert!(track.is_empty());

        track.add_clip(make_clip(), 10, 5);
        assert_eq!(track.len(), 1);

        track.add_clip(make_clip(), 15, 10);
        assert_eq!(track.len(), 2);

        let clips: Vec<_> = track.clips().collect();
        assert_eq!(clips[0].0, 10..15);
        assert_eq!(clips[1].0, 15..25);
    }

    #[test]
    #[should_panic(expected = "clips must not overlap nor be out of time order")]
    fn push_clip_overlap_panics() {
        let mut track = AutomationTrack::new();
        track.add_clip(make_clip(), 10, 10);
        track.add_clip(make_clip(), 15, 10);
    }

    #[test]
    #[should_panic(expected = "clips must not overlap nor be out of time order")]
    fn push_clip_out_of_order_panics() {
        let mut track = AutomationTrack::new();
        track.add_clip(make_clip(), 20, 10);
        track.add_clip(make_clip(), 0, 5);
    }

    #[test]
    fn remove_and_pop_clip() {
        let mut track = AutomationTrack::new();
        track.add_clip(make_clip(), 10, 5);
        track.add_clip(make_clip(), 20, 5);
        track.add_clip(make_clip(), 30, 5);

        track.remove_clip(1);
        assert_eq!(track.len(), 2);

        let popped = track.pop_clip();
        assert!(popped.is_some());
        assert_eq!(track.len(), 1);
    }

    #[test]
    fn resize_clip_success() {
        let mut track = AutomationTrack::new();
        track.add_clip(make_clip(), 10, 5);
        track.add_clip(make_clip(), 20, 5);

        track.resize_clip(0, 8);

        let ranges: Vec<_> = track.clips().map(|(r, _)| r).collect();
        assert_eq!(ranges[0], 10..18);
    }

    #[test]
    #[should_panic(expected = "new range would overlap with the next")]
    fn resize_clip_overlap_panics() {
        let mut track = AutomationTrack::new();
        track.add_clip(make_clip(), 10, 5);
        track.add_clip(make_clip(), 20, 5);

        track.resize_clip(0, 15);
    }

    #[test]
    fn reposition_clip_success() {
        let mut track = AutomationTrack::new();
        track.add_clip(make_clip(), 10, 5);
        track.add_clip(make_clip(), 20, 5);
        track.add_clip(make_clip(), 30, 5);

        // Move B to sit between A and C more tightly
        track.reposition_clip(1, 16);

        let ranges: Vec<_> = track.clips().map(|(r, _)| r).collect();
        assert_eq!(ranges[0], 10..15);
        assert_eq!(ranges[1], 16..21);
        assert_eq!(ranges[2], 30..35);
    }

    #[test]
    fn reposition_clip_reorder() {
        let mut track = AutomationTrack::new();
        track.add_clip(make_clip(), 10, 5);
        track.add_clip(make_clip(), 20, 5);
        track.add_clip(make_clip(), 30, 5);

        track.reposition_clip(1, 35);

        let ranges: Vec<_> = track.clips().map(|(r, _)| r).collect();
        assert_eq!(ranges[0], 10..15);
        assert_eq!(ranges[1], 30..35);
        assert_eq!(ranges[2], 35..40);
    }

    #[test]
    fn query_clip() {
        let mut track = AutomationTrack::new();
        track.add_clip(make_clip(), 10, 10);
        track.add_clip(make_clip(), 30, 10);

        // Inside first clip
        assert!(track.query_clip(15).is_some());

        // Inside gap
        assert!(track.query_clip(25).is_none());

        // Inside second clip
        assert!(track.query_clip(35).is_some());

        // Before first clip
        assert!(track.query_clip(5).is_none());

        assert!(track.query_clip(10).is_some());
        assert!(track.query_clip(20).is_none());
    }

    fn assert_close(lhs: f32, rhs: f32) {
        const EPS: f32 = 1e-6;

        assert!((lhs - rhs).abs() <= EPS)
    }

    #[test]
    fn sample() {
        let track = AutomationTrack::from_iter([
            (
                0..10,
                Lfo::new(LfoShape::Saw, 10.try_into().unwrap()).into(),
            ),
            (
                20..30,
                Lfo::new(LfoShape::Square, 10.try_into().unwrap()).into(),
            ),
        ]);

        for t in 0..10 {
            assert_close(track.query_value(t), t as f32 / 10.0);
        }

        for t in 11..20 {
            assert_close(track.query_value(t), 1.0);
        }

        for t in 20..25 {
            assert_eq!(track.query_value(t), 0.0);
        }

        for t in 25..30 {
            assert_eq!(track.query_value(t), 1.0);
        }
    }
}
