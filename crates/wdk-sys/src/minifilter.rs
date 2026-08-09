// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! Direct FFI bindings to File System Minifilter APIs from the Windows Driver
//! Kit (WDK)
//!
//! This module contains all bindings to functions, constants, methods,
//! constructors and destructors for Filter Manager headers. Types are not
//! included in this module, but are available in the top-level `wdk_sys`
//! module.

pub use bindings::*;

use crate::types::UCHAR;

// Macros with C casts in their expansion are not supported by bindgen, so they
// must be manually ported: https://github.com/rust-lang/rust-bindgen/issues/316
/// Sentinel value that terminates the `FLT_OPERATION_REGISTRATION` array
/// pointed to by `FLT_REGISTRATION::OperationRegistration`
pub const IRP_MJ_OPERATION_END: UCHAR = 0x80;

#[allow(
    missing_docs,
    reason = "most items in the WDK headers have no inline documentation, so bindgen is unable to \
              generate documentation for their bindings"
)]
mod bindings {
    #[allow(
        clippy::wildcard_imports,
        reason = "the underlying c code relies on all type definitions being in scope, which \
                  results in the bindgen generated code relying on the generated types being in \
                  scope as well"
    )]
    use crate::types::*;

    include!(concat!(env!("OUT_DIR"), "/minifilter.rs"));
}
