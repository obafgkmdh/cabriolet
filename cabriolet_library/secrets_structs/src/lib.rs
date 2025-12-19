#![feature(generic_const_exprs)]

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use futures::future::BoxFuture;

pub type TimelyClosure<T> = Arc<dyn Fn() -> BoxFuture<'static, T> + Send + Sync>;

#[derive(Clone, Debug)]
pub enum LabelTimely<const TIME: u64> {}

#[derive(Clone, Debug)]
pub enum LabelNonIdem {}

pub trait Label {
    type MetaData<T>: Clone + Send;
}

pub trait AtMostAsIdemAs<T>: Label {}
//impl<T: Label> AtMostAsIdemAs<T> for T {} // reflexive property

impl<const TIME: u64> Label for LabelTimely<TIME> {
    type MetaData<T> = (Instant, TimelyClosure<T>);
}
impl Label for LabelNonIdem {
    type MetaData<T> = ();
}

impl<L: Label> AtMostAsIdemAs<L> for LabelNonIdem {}
impl<const T1: u64, const T2: u64> AtMostAsIdemAs<LabelTimely<T2>> for LabelTimely<T1> where
    [(); T1 as usize - T2 as usize]: Sized
{
}

// TODO: ideally, this wouldn't be pub
#[async_trait]
pub trait Contains<T> {
    type CreationArgs;

    fn new(args: Self::CreationArgs) -> Self;

    async unsafe fn unwrap_unchecked(&mut self) -> T;
}

#[derive(Clone)]
pub struct Labeled<T, L>
where
    T: Clone + Send,
    L: Label,
{
    val: Option<T>,
    metadata: L::MetaData<T>,
}

#[async_trait]
impl<T: Clone + Send> Contains<T> for Labeled<T, LabelNonIdem> {
    type CreationArgs = T;

    fn new(val: Self::CreationArgs) -> Self {
        Self {
            val: Some(val),
            metadata: (),
        }
    }

    async unsafe fn unwrap_unchecked(&mut self) -> T {
        self.val
            .clone()
            .expect("NonIdem should always have a value")
    }
}

#[async_trait]
impl<T: Clone + Send, const TIME: u64> Contains<T> for Labeled<T, LabelTimely<TIME>> {
    type CreationArgs = TimelyClosure<T>;

    fn new(create_fn: Self::CreationArgs) -> Self {
        Self {
            val: None,
            metadata: (Instant::now() + Duration::from_millis(TIME), create_fn),
        }
    }

    async unsafe fn unwrap_unchecked(&mut self) -> T {
        let now = Instant::now();
        let (expiry, ref create_fn) = self.metadata;
        if now < expiry && self.val.is_some() {
            self.val.clone().unwrap()
        } else {
            let val = create_fn().await;
            self.val = Some(val.clone());

            val
        }
    }
}

#[allow(private_bounds)]
impl<T, L> Labeled<T, L>
where
    T: Clone + Send,
    L: Label,
    Labeled<T, L>: Contains<T>,
{
    pub fn new(args: <Self as Contains<T>>::CreationArgs) -> Self {
        <Self as Contains<T>>::new(args)
    }

    pub async fn unwrap_checked<Lp>(&mut self) -> T
    where
        Lp: AtMostAsIdemAs<L>,
    {
        unsafe { self.unwrap_unchecked() }.await
    }

    pub async fn endorse_idempotent(mut self) -> T {
        self.unwrap_checked::<LabelNonIdem>().await
    }
}
