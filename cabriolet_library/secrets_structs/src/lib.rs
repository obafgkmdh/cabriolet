use std::marker::PhantomData;

// #[derive(Clone)]
// pub enum LabelIdem {}

#[derive(Clone)]
pub enum LabelTimely {}

#[derive(Clone)]
pub enum LabelNonIdem {}

pub trait Label {}

pub trait AtLeastAsIdemAs<T>: Label {}
impl<T: Label> AtLeastAsIdemAs<T> for T {} // reflexive property

// impl Label for LabelIdem {}
impl Label for LabelTimely {}
impl Label for LabelNonIdem {}

impl AtLeastAsIdemAs<LabelNonIdem> for LabelTimely {}
// impl AtLeastAsIdemAs<LabelNonIdem> for LabelIdem {}
// impl AtLeastAsIdemAs<LabelTimely> for LabelIdem {}

pub struct Labeled<T, L> where L: Label {
    val: T,
    _pd: PhantomData<L>,
}
