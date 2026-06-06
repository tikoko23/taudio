use std::ops::{Index, IndexMut};

use crate::id::NumericId;

/// A trait which allows a container to be indexable by a numeric id.
///
/// To use [`NumericId`]s with containers that implement [`Index`], see [`IdContainer`].
/// For a mutable version, see [`IndexByIdMut`].
///
/// [`IdContainer`]: crate::id::IdContainer
pub trait IndexById {
    type Output: ?Sized;

    fn index_by_id<I: NumericId>(&self, id: I) -> &Self::Output;
}

/// Mutable version of [`IndexById`].
pub trait IndexByIdMut: IndexById {
    fn index_by_id_mut<I: NumericId>(&mut self, id: I) -> &mut <Self as IndexById>::Output;
}

impl<A: Index<usize>> IndexById for A {
    type Output = <A as Index<usize>>::Output;

    #[inline]
    fn index_by_id<I: NumericId>(&self, id: I) -> &Self::Output {
        &self[id.as_index()]
    }
}

impl<A: IndexMut<usize>> IndexByIdMut for A {
    #[inline]
    fn index_by_id_mut<I: NumericId>(&mut self, id: I) -> &mut Self::Output {
        &mut self[id.as_index()]
    }
}
