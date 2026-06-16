use std::ops::Range;

use crate::{Real, automation::AutomationClip};

#[derive(Debug, Clone)]
struct ClipData {
    clip: AutomationClip,
    range: Range<Real>,
}

impl ClipData {
    pub fn duration(&self) -> Real {
        self.range.end - self.range.start
    }
}

fn sample_clip(clip: &ClipData, offset: Real) -> Real {
    if clip.range.end - clip.range.start == 0.0 {
        return clip.clip.sample(0.0);
    }

    let clamp = Real::clamp(offset, clip.range.start, clip.range.end);
    let local = clamp - clip.range.start;

    clip.clip.sample(local)
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

impl FromIterator<(Range<Real>, AutomationClip)> for AutomationTrack {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (Range<Real>, AutomationClip)>,
    {
        let mut track = AutomationTrack::new();

        let iter = iter.into_iter();

        track.clips.reserve(iter.size_hint().0);

        for (range, clip) in iter {
            track.add_clip(clip, range);
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
    pub fn clips(&self) -> impl Iterator<Item = (Range<Real>, &AutomationClip)> {
        self.clips.iter().map(|c| (c.range.clone(), &c.clip))
    }

    /// Returns an iterator over the clips in this track and their occupied ranges.
    ///
    /// Mutating the clip objects is safe because the occupied range is not determined by
    /// the clip itself.
    pub fn clips_mut(&mut self) -> impl Iterator<Item = (Range<Real>, &mut AutomationClip)> {
        self.clips
            .iter_mut()
            .map(|c| (c.range.clone(), &mut c.clip))
    }

    /// Adds a new clip to the end of the track.
    ///
    /// # Panics
    /// - Panics if the
    /// - Panics if the new track violates the [overlap constraint](AutomationTrack#overlaps).
    /// - Panics if the new track violates the [time ordering constraint](AutomationTrack#time-ordering).
    pub fn add_clip(&mut self, clip: AutomationClip, span: Range<Real>) {
        assert!(span.start.is_finite(), "clip start time must be finite");
        assert!(span.start >= 0.0, "clip start time must be positive");

        assert!(span.end.is_finite(), "clip start time must be finite");
        assert!(span.end >= 0.0, "clip start time must be positive");

        assert!(span.end >= span.start);

        if let Some(last) = self.clips.last() {
            assert!(
                last.range.end <= span.start,
                "clips must not overlap nor be out of time order"
            );
        }

        self.clips.push(ClipData { clip, range: span });
    }

    /// # Panics
    /// - Panics if the index is out of range.
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
    /// - Panics if the new duration is non-finite or negative.
    /// - Panics if the index is out of range.
    /// - Panics if the [overlap constraint](AutomationClip#overlaps) is violated.
    pub fn resize_clip(&mut self, index: usize, new_duration: Real) {
        assert!(
            new_duration.is_finite() && new_duration >= 0.0,
            "duration cannot be non-finite or negative"
        );

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
    /// - Panics if the new offset is non-finite or negative.
    /// - Panics if the index is out of range.
    /// - Panics if the [overlap constraint](AutomationClip#overlaps) is violated.
    pub fn reposition_clip(&mut self, index: usize, new_offset: Real) {
        assert!(
            new_offset.is_finite() && new_offset >= 0.0,
            "offset cannot be non-finite or negative"
        );

        let new_position = self.clips.partition_point(|c| c.range.end <= new_offset);

        if let Some(next) = self.clips.get(new_position + 1) {
            let this_clip = &self.clips[index];

            assert!(
                next.range.start > new_offset + this_clip.duration(),
                "new position would overlap with the next clip"
            );

            let correction = new_position > index;

            let mut clip = self.clips.remove(index);
            clip.range = new_offset..(new_offset + clip.duration());

            self.clips.insert(new_position - correction as usize, clip);
        } else {
            let mut clip = self.clips.remove(index);
            clip.range = new_offset..(new_offset + clip.duration());

            self.clips.push(clip);
        }
    }

    /// Returns the clip in which the given sample offset lies, or [`None`].
    pub fn query_clip(&self, offset: Real) -> Option<&AutomationClip> {
        assert!(
            offset.is_finite() && offset >= 0.0,
            "offset cannot be non-finite or negative"
        );

        let split_index = self.clips.partition_point(|c| c.range.end <= offset);

        if split_index >= self.clips.len() {
            return None;
        }

        let candidate = &self.clips[split_index];

        if candidate.range.contains(&offset) {
            Some(&candidate.clip)
        } else {
            None
        }
    }

    /// Queries the value of the track at the given sample offset.
    ///
    /// If there are no clips, the fallback value is returned.
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
    pub fn query_value(&self, offset: Real, fallback: Real) -> Real {
        assert!(
            offset.is_finite() && offset >= 0.0,
            "offset cannot be non-finite or negative"
        );

        let Some(first) = self.clips.first() else {
            return fallback;
        };

        let late_clip_index = self.clips.partition_point(|c| c.range.start <= offset);

        if late_clip_index == 0 {
            sample_clip(first, first.range.start)
        } else {
            let clip = &self.clips[late_clip_index - 1];

            sample_clip(clip, offset)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::automation::{Lfo, LfoShape};

    use super::*;

    fn make_clip() -> AutomationClip {
        AutomationClip::Constant(0.5)
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

        track.add_clip(make_clip(), 10.0..15.0);
        assert_eq!(track.len(), 1);

        track.add_clip(make_clip(), 15.0..25.0);
        assert_eq!(track.len(), 2);

        let clips: Vec<_> = track.clips().collect();
        assert_eq!(clips[0].0, 10.0..15.0);
        assert_eq!(clips[1].0, 15.0..25.0);
    }

    #[test]
    #[should_panic(expected = "clips must not overlap nor be out of time order")]
    fn push_clip_overlap_panics() {
        let mut track = AutomationTrack::new();
        track.add_clip(make_clip(), 10.0..20.0);
        track.add_clip(make_clip(), 15.0..25.0);
    }

    #[test]
    #[should_panic(expected = "clips must not overlap nor be out of time order")]
    fn push_clip_out_of_order_panics() {
        let mut track = AutomationTrack::new();
        track.add_clip(make_clip(), 20.0..30.0);
        track.add_clip(make_clip(), 0.0..5.0);
    }

    #[test]
    fn remove_and_pop_clip() {
        let mut track = AutomationTrack::new();
        track.add_clip(make_clip(), 10.0..15.0);
        track.add_clip(make_clip(), 20.0..25.0);
        track.add_clip(make_clip(), 30.0..35.0);

        track.remove_clip(1);
        assert_eq!(track.len(), 2);

        let popped = track.pop_clip();
        assert!(popped.is_some());
        assert_eq!(track.len(), 1);
    }

    #[test]
    fn resize_clip_success() {
        let mut track = AutomationTrack::new();
        track.add_clip(make_clip(), 10.0..15.0);
        track.add_clip(make_clip(), 20.0..25.0);

        track.resize_clip(0, 8.0);

        let ranges: Vec<_> = track.clips().map(|(r, _)| r).collect();
        assert_eq!(ranges[0], 10.0..18.0);
    }

    #[test]
    #[should_panic(expected = "new range would overlap with the next")]
    fn resize_clip_overlap_panics() {
        let mut track = AutomationTrack::new();
        track.add_clip(make_clip(), 10.0..15.0);
        track.add_clip(make_clip(), 20.0..25.0);

        track.resize_clip(0, 15.0);
    }

    #[test]
    fn reposition_clip_success() {
        let mut track = AutomationTrack::new();
        track.add_clip(make_clip(), 10.0..15.0);
        track.add_clip(make_clip(), 20.0..25.0);
        track.add_clip(make_clip(), 30.0..35.0);

        // Move B to sit between A and C more tightly
        track.reposition_clip(1, 16.0);

        let ranges: Vec<_> = track.clips().map(|(r, _)| r).collect();
        assert_eq!(ranges[0], 10.0..15.0);
        assert_eq!(ranges[1], 16.0..21.0);
        assert_eq!(ranges[2], 30.0..35.0);
    }

    #[test]
    fn reposition_clip_reorder() {
        let mut track = AutomationTrack::new();
        track.add_clip(make_clip(), 10.0..15.0);
        track.add_clip(make_clip(), 20.0..25.0);
        track.add_clip(make_clip(), 30.0..35.0);

        track.reposition_clip(1, 35.0);

        let ranges: Vec<_> = track.clips().map(|(r, _)| r).collect();
        assert_eq!(ranges[0], 10.0..15.0);
        assert_eq!(ranges[1], 30.0..35.0);
        assert_eq!(ranges[2], 35.0..40.0);
    }

    macro_rules! assert_matches {
        ($expr:expr, $pat:pat) => {
            assert!(matches!($expr, $pat));
        };
    }

    #[test]
    fn query_clip() {
        let mut track = AutomationTrack::new();
        track.add_clip(AutomationClip::Constant(0.23), 10.0..20.0);
        track.add_clip(AutomationClip::Constant(0.67), 30.0..40.0);

        // Inside first clip
        assert_matches!(track.query_clip(15.0), Some(AutomationClip::Constant(0.23)));

        // Inside gap
        assert!(track.query_clip(25.0).is_none());

        // Inside second clip
        assert_matches!(track.query_clip(35.0), Some(AutomationClip::Constant(0.67)));

        // Before first clip
        assert!(track.query_clip(5.0).is_none());

        assert_matches!(track.query_clip(10.0), Some(AutomationClip::Constant(0.23)));

        assert!(track.query_clip(20.0).is_none());
    }

    fn assert_close(lhs: Real, rhs: Real) {
        const EPS: Real = 1e-9;

        assert!((lhs - rhs).abs() <= EPS)
    }

    #[test]
    fn sample() {
        let track = AutomationTrack::from_iter([
            (10.0..20.0, AutomationClip::Constant(0.23)),
            (30.0..40.0, AutomationClip::Constant(0.67)),
        ]);

        assert_eq!(track.query_value(0.0, Real::NAN), 0.23);
        assert_eq!(track.query_value(15.0, Real::NAN), 0.23);
        assert_eq!(track.query_value(25.0, Real::NAN), 0.23);
        assert_eq!(track.query_value(35.0, Real::NAN), 0.67);
        assert_eq!(track.query_value(45.0, Real::NAN), 0.67);
    }
}
