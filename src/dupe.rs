use crate::id::{IdContainer, IndexById};

/// A variant of [`Clone`] for types which can't always be cloned.
pub trait Dupe: Sized {
    #[inline]
    fn dupe(&self) -> Option<Self> {
        None
    }
}

impl<T: Dupe> Dupe for Vec<T> {
    fn dupe(&self) -> Option<Self> {
        let mut new = Vec::with_capacity(self.len());

        for item in self {
            new.push(item.dupe()?);
        }

        Some(new)
    }
}

impl<T: Dupe, const N: usize> Dupe for [T; N] {
    fn dupe(&self) -> Option<Self> {
        let opt = std::array::from_fn(|i| self[i].dupe());

        for item in &opt {
            if item.is_none() {
                return None;
            }
        }

        Some(opt.map(|x| x.unwrap()))
    }
}

impl<T: Dupe> Dupe for Option<T> {
    fn dupe(&self) -> Option<Self> {
        self.as_ref().map(Dupe::dupe)
    }
}

impl<T: Dupe + IndexById> Dupe for IdContainer<T> {
    fn dupe(&self) -> Option<Self> {
        self.as_inner().dupe().map(IdContainer::new)
    }
}
