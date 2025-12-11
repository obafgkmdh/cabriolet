use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub enum LabelTimely<const TIME: u64> {}

#[derive(Clone, Debug)]
pub enum LabelNonIdem {}

pub trait Label {
    type MetaData<T>;
}

pub trait AtMostAsIdemAs<T>: Label {}
impl<T: Label> AtMostAsIdemAs<T> for T {} // reflexive property

impl<const TIME: u64> Label for LabelTimely<TIME> {
    type MetaData<T> = Instant;
}
impl Label for LabelNonIdem {
    type MetaData<T> = ();
}

impl<const TIME: u64> AtMostAsIdemAs<LabelTimely<TIME>> for LabelNonIdem {}

trait Contains<T> {
    fn new(item: T) -> Self;
    fn unwrap_unchecked(self) -> T;
}

#[derive(Debug)]
pub struct Labeled<T, L>
where
    L: Label,
{
    val: T,
    metadata: L::MetaData<T>,
}

impl<T> Contains<T> for Labeled<T, LabelNonIdem> {
    fn new(item: T) -> Self {
        Self {
            val: item,
            metadata: (),
        }
    }

    fn unwrap_unchecked(self) -> T {
        self.val
    }
}

impl<T, const TIME: u64> Contains<T> for Labeled<T, LabelTimely<TIME>> {
    fn new(item: T) -> Self {
        Self {
            val: item,
            metadata: Instant::now() + Duration::from_millis(TIME),
        }
    }

    fn unwrap_unchecked(self) -> T {
        let now = Instant::now();
        let expiry = self.metadata;
        if now < expiry {
            self.val
        } else {
            unimplemented!("tried to unwrap expired value");
        }
    }
}

#[allow(private_bounds)]
impl<T, L: Label> Labeled<T, L> where Labeled<T, L>: Contains<T> {
    pub fn new(item: T) -> Self {
        <Self as Contains<T>>::new(item)
    }

    pub fn unwrap_checked<Lp>(self) -> T
    where
        Lp: AtMostAsIdemAs<L>,
    {
        self.unwrap_unchecked()
    }

    pub fn endorse_idempotent(self) -> T {
        self.val
    }
}
