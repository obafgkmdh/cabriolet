use std::marker::PhantomData;

// #[derive(Clone)]
// pub enum LabelIdem {}

#[derive(Clone, Debug)]
pub enum LabelTimely {}

#[derive(Clone, Debug)]
pub enum LabelNonIdem {}

pub trait Label {}

pub trait AtMostAsIdemAs<T>: Label {}
impl<T: Label> AtMostAsIdemAs<T> for T {} // reflexive property

impl Label for LabelTimely {}
impl Label for LabelNonIdem {}

impl AtMostAsIdemAs<LabelTimely> for LabelNonIdem {}

#[derive(Debug)]
pub struct Labeled<T, L>
where
    L: Label,
{
    val: T,
    _pd: PhantomData<L>,
}

impl<T, L: Label> Labeled<T, L> {
    pub fn new(item: T) -> Self {
        Self {
            val: item,
            _pd: PhantomData,
        }
    }

    pub fn unwrap_checked<Lp>(self) -> T
    where
        Lp: AtMostAsIdemAs<L>,
    {
        self.unwrap_unchecked()
    }

    fn unwrap_unchecked(self) -> T {
        self.val
    }

    pub fn endorse_idempotent(self) -> T {
        self.val
    }
}
