//! Triplify: capture any tabular data as raw RDF first, map later.

pub mod emit;
pub mod formula;
pub mod git;
pub mod iri;
pub mod reader;
pub mod source;
pub mod triplify;
pub mod vocab;
pub mod walk;
pub mod writer;

pub use triplify::{Summary, Triplify};
