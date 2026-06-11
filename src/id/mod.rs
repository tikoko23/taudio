mod container;
mod index;
mod macros;

pub use container::*;
pub use index::*;
pub use macros::*;

pub trait IncrementalId {
    const FIRST: Self;

    fn next(&self) -> Self;
}

/// Allows an [`IncrementalId`] to be used as an index for containers which implement [`IndexById`].
///
/// To use [`NumericId`] types as indices for external types, see [`IdContainer`].
pub trait NumericId: IncrementalId {
    fn as_index(&self) -> usize;
    fn from_index(index: usize) -> Self;
}

/// Continuously yields incremental ids in the order [`IncrementalId::next`] provides them.
#[derive(Debug, Clone)]
pub struct IdSequence<I: IncrementalId> {
    next: I,
}

impl<I: IncrementalId> Default for IdSequence<I> {
    #[inline]
    fn default() -> Self {
        Self { next: I::FIRST }
    }
}

impl<I: IncrementalId> IdSequence<I> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_id(&mut self) -> I {
        let mut next = self.next.next();

        std::mem::swap(&mut next, &mut self.next);

        next
    }
}
