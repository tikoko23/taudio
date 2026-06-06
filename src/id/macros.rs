/// A macro which allows easy creation of [`IncrementalId`] types.
///
/// # Example
///
/// ```rust,no_run
/// # use taudio::id::incremental_id;
///
/// incremental_id! {
///     #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
///     pub struct Id(u32) impl { NumericId };
/// }
/// ```
///
/// The `impl` block is completely optional. If provided, it must contain a comma seperated
/// (trailing allowed) list of supported trait names given below.
///
/// - [`NumericId`]: The created type implements [`NumericId`] and maps each internal number to
///   one less (because internally [`NonZero`] is used)
///
/// # Details
/// Internalyl a [`NonZero`] is used to allow for niche optimization, thus the type provided as
/// the "inner type" must be compatible with [`NonZero`].
///
/// [`IncrementalId`]: crate::id::IncrementalId
/// [`NumericId`]: crate::id::NumericId
/// [`NonZero`]: std::num::NonZero
#[macro_export]
macro_rules! incremental_id {
    (
        $(#[$meta:meta])*
        $vis:vis struct $Name:ident($inner_vis:vis $T:ty) $(
            impl { $($extra_features:ident),* $(,)? }
        )?;
    ) => {
        $(#[$meta])*
        $vis struct $Name($inner_vis ::std::num::NonZero<$T>);

        impl $crate::id::IncrementalId for $Name {
            const FIRST: Self = Self(::std::num::NonZero::new(1).unwrap());

            fn next(&self) -> Self {
                ::core::debug_assert!(self.0.get() != <$T>::MAX);

                $Name(::std::num::NonZero::<$T>::new(self.0.get().saturating_add(1)).unwrap())
            }
        }

        $(
            $crate::id::incremental_id!(@munch(extra_features) [$Name, $T] $($extra_features),* ,);
        )?
    };

    (@munch(extra_features) [$Name:ident, $T:ty] NumericId, $($tail:tt)*) => {
        impl $crate::id::NumericId for $Name {
            #[inline]
            fn as_index(&self) -> usize {
                self.0.get() as usize - 1
            }

            #[inline]
            fn from_index(index: usize) -> Self {
                // The use of saturating add saves a branch which would otherwise be used for
                // the unrealistic case that index is `usize::MAX`.
                Self(::std::num::NonZero::new(index.saturating_add(1) as $T).unwrap())
            }
        }

        $crate::id::incremental_id!(@munch(extra_features) [$Name, $T] $($tail)*);
    };

    (@munch(extra_features) [$Name:ident, $T:ty] $(,)?) => {};
}

pub use incremental_id;

// Guards against the removal of previously existing trait features.
#[allow(dead_code)]
mod internal {
    incremental_id! {
        #[derive(Debug)]
        struct Id0(u32) impl { NumericId };
    }
}
