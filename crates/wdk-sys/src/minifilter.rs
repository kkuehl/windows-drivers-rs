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

use crate::{
    constants::{FLT_PORT_CONNECT, STANDARD_RIGHTS_ALL},
    types::{ACCESS_MASK, UCHAR},
};

// Macros with C casts in their expansion are not supported by bindgen, so they
// must be manually ported: https://github.com/rust-lang/rust-bindgen/issues/316
/// Sentinel value that terminates the `FLT_OPERATION_REGISTRATION` array
/// pointed to by `FLT_REGISTRATION::OperationRegistration`
pub const IRP_MJ_OPERATION_END: UCHAR = 0x80;

// Object-like macros whose expansion references other identifiers are not
// emitted by bindgen either, so the access mask a minifilter passes to
// `FltBuildDefaultSecurityDescriptor` must be composed by hand. It is derived
// from the generated constants rather than written out as a literal so that it
// cannot drift from `fltKernel.h`.
/// Access mask granting full access to a filter communication port, for use as
/// the `DesiredAccess` of
/// [`FltBuildDefaultSecurityDescriptor`](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/fltkernel/nf-fltkernel-fltbuilddefaultsecuritydescriptor).
pub const FLT_PORT_ALL_ACCESS: ACCESS_MASK = FLT_PORT_CONNECT | STANDARD_RIGHTS_ALL;

// `FltGetRequestorProcessId` is defined in fltKernel.h but bindgen does not
// generate it, possibly because it's an inline function or has attributes that
// bindgen skips. Minifilters need this to identify which process originated an
// I/O operation, so it's manually ported here.
//
// SAFETY: This function is safe to call with any valid FLT_CALLBACK_DATA pointer.
// It returns the process ID (ULONG) of the thread that originated the operation.
unsafe extern "C" {
    /// Returns the process ID of the thread that originated the I/O operation
    /// represented by the given callback data.
    ///
    /// # Parameters
    /// * `CallbackData` - Pointer to the callback data for the I/O operation
    ///
    /// # Returns
    /// The process ID (PID) as a `ULONG` (u32)
    ///
    /// # Safety
    /// The caller must ensure `CallbackData` is a valid pointer to
    /// `FLT_CALLBACK_DATA`.
    pub fn FltGetRequestorProcessId(CallbackData: *mut crate::types::FLT_CALLBACK_DATA)
        -> crate::types::ULONG;
}

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
