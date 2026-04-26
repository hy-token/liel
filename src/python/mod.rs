/// Python bindings module for liel.
///
/// This module re-exports all PyO3 types and the `open()` function that together
/// form the public Python API surface of the liel graph database.  The actual
/// type definitions live in the `types` sub-module; this file exists so that
/// the crate can refer to `crate::python::types::*` while keeping the module
/// hierarchy tidy.
pub mod types;
