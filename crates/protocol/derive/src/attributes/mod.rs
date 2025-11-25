//! Module containing the [AttributesBuilder] trait implementations.
//!
//! [AttributesBuilder]: crate::traits::AttributesBuilder

mod bitset;
mod stateful;

pub use bitset::BitSetError;
pub use stateful::StatefulAttributesBuilder;
