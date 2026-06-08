use std::ops::{Deref, DerefMut, Index, IndexMut};

use crate::id::{IndexById, IndexByIdMut, NumericId};

/// A container adapter which allows the use of [`NumericId`]s as indices.
///
/// # Example
/// ```rust
/// # use taudio::id::IdContainer;
/// # use taudio::id::incremental_id;
///
/// incremental_id! {
///     #[derive(Debug, Clone, Copy)]
///     pub struct Id(u32) impl { NumericId };
/// }
///
/// fn main() {
///     let mut strings = IdContainer::new(vec![]);
///
///     // The `push_id` function is only for `Vec`.
///     let xyz_id: Id = strings.push_id("xyz");
///     let foo_id: Id = strings.push_id("foo");
///
///     // This works for any container.
///     assert_eq!(strings[xyz_id], "xyz");
///     assert_eq!(strings[foo_id], "foo");
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdContainer<A: IndexById>(A);

impl<A: IndexById> IdContainer<A> {
    #[inline]
    pub fn new(a: A) -> Self {
        Self(a)
    }

    #[inline]
    pub fn into_inner(self) -> A {
        self.0
    }

    #[inline]
    pub fn as_inner(&self) -> &A {
        &self.0
    }

    #[inline]
    pub fn as_inner_mut(&mut self) -> &mut A {
        &mut self.0
    }

    pub fn iter_with_id<'a, I: NumericId>(
        &'a self,
    ) -> impl Iterator<Item = (I, <&'a A as IntoIterator>::Item)>
    where
        &'a A: IntoIterator,
    {
        self.as_inner()
            .into_iter()
            .enumerate()
            .map(|(i, v)| (I::from_index(i), v))
    }

    pub fn iter_with_id_mut<'a, I: NumericId>(
        &'a mut self,
    ) -> impl Iterator<Item = (I, <&'a mut A as IntoIterator>::Item)>
    where
        &'a mut A: IntoIterator,
    {
        self.as_inner_mut()
            .into_iter()
            .enumerate()
            .map(|(i, v)| (I::from_index(i), v))
    }
}

impl<A: IndexById + Index<usize>> Index<usize> for IdContainer<A> {
    type Output = <A as Index<usize>>::Output;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl<A: IndexById + IndexMut<usize>> IndexMut<usize> for IdContainer<A> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl<I: NumericId, A: IndexById> Index<I> for IdContainer<A> {
    type Output = <A as IndexById>::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.0.index_by_id(index)
    }
}

impl<I: NumericId, A: IndexByIdMut> IndexMut<I> for IdContainer<A> {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.0.index_by_id_mut(index)
    }
}

impl<A: IndexById> Deref for IdContainer<A> {
    type Target = A;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<A: IndexById> DerefMut for IdContainer<A> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<A: IndexById> AsRef<A> for IdContainer<A> {
    #[inline]
    fn as_ref(&self) -> &A {
        &self.0
    }
}

impl<A: IndexById> AsMut<A> for IdContainer<A> {
    #[inline]
    fn as_mut(&mut self) -> &mut A {
        &mut self.0
    }
}

impl<A: IndexById> From<A> for IdContainer<A> {
    #[inline]
    fn from(value: A) -> Self {
        Self(value)
    }
}

impl<A> Default for IdContainer<A>
where
    A: IndexById + Default,
{
    #[inline]
    fn default() -> Self {
        Self(A::default())
    }
}

impl<A> IntoIterator for IdContainer<A>
where
    A: IndexById + IntoIterator,
{
    type Item = A::Item;
    type IntoIter = A::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T> IdContainer<Vec<T>> {
    pub fn push_id<I: NumericId>(&mut self, value: T) -> I {
        let new_idx = self.len();
        self.push(value);

        I::from_index(new_idx)
    }

    #[inline]
    pub fn next_id<I: NumericId>(&self) -> I {
        I::from_index(self.len())
    }
}

#[cfg(test)]
mod test {
    use crate::{
        id::{IdContainer, IdSequence, NumericId},
        incremental_id,
    };

    incremental_id! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct TestId(u32) impl { NumericId };
    }

    macro_rules! id {
        ($n:literal) => {
            TestId::from_index($n - 1)
        };
    }

    #[test]
    fn vec_wrapper() {
        let a = IdContainer::new(vec![1, 2, 3, 4]);

        let mut seq: IdSequence<TestId> = IdSequence::default();
        let ids: [_; 4] = std::array::from_fn(|_| seq.next_id());

        assert_eq!(a[ids[0]], a[0]);
        assert_eq!(a[ids[1]], a[1]);
        assert_eq!(a[ids[2]], a[2]);
        assert_eq!(a[ids[3]], a[3]);
    }

    #[test]
    fn iter_by_id() {
        let a = IdContainer::new(vec![1, 2, 3]);
        let iter_result: Vec<_> = a.iter_with_id::<TestId>().map(|(id, &v)| (id, v)).collect();

        assert_eq!(iter_result, [(id!(1), 1), (id!(2), 2), (id!(3), 3)]);
    }
}
