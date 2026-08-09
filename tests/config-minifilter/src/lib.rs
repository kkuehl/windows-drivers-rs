// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! Compile-time checks that the `minifilter` API subset of `wdk-sys` generates
//! the Filter Manager routines a File System Minifilter driver needs, with
//! signatures that accept the generated types.
//!
//! These live in the library rather than in `tests/` because naming a `Flt*`
//! routine adds a real import on `FLTMGR.SYS`, which is a kernel image: the
//! `#[test]` harness produces a user-mode executable that would fail to load
//! with `STATUS_DLL_NOT_FOUND`. Coercing each routine to an explicitly written
//! `fn` type in a `const` gets the same signature check from `rustc` with no
//! runtime component at all, so a bindgen regression that drops a routine or
//! changes an argument type is a build failure.

/// Signatures of the Filter Manager routines a minifilter cannot avoid calling.
/// Written out longhand so that they are checked against `fltKernel.h` via the
/// generated bindings rather than inferred from them.
mod filter_manager_routine_signatures {
    use wdk_sys::{
        FLT_FILE_NAME_OPTIONS,
        FLT_REGISTRATION,
        NTSTATUS,
        PDRIVER_OBJECT,
        PFLT_CALLBACK_DATA,
        PFLT_FILE_NAME_INFORMATION,
        PFLT_FILTER,
        minifilter,
    };

    const _FLT_REGISTER_FILTER: unsafe extern "C" fn(
        PDRIVER_OBJECT,
        *const FLT_REGISTRATION,
        *mut PFLT_FILTER,
    ) -> NTSTATUS = minifilter::FltRegisterFilter;

    const _FLT_START_FILTERING: unsafe extern "C" fn(PFLT_FILTER) -> NTSTATUS =
        minifilter::FltStartFiltering;

    const _FLT_UNREGISTER_FILTER: unsafe extern "C" fn(PFLT_FILTER) =
        minifilter::FltUnregisterFilter;

    const _FLT_GET_FILE_NAME_INFORMATION: unsafe extern "C" fn(
        PFLT_CALLBACK_DATA,
        FLT_FILE_NAME_OPTIONS,
        *mut PFLT_FILE_NAME_INFORMATION,
    ) -> NTSTATUS = minifilter::FltGetFileNameInformation;

    const _FLT_PARSE_FILE_NAME_INFORMATION: unsafe extern "C" fn(
        PFLT_FILE_NAME_INFORMATION,
    ) -> NTSTATUS = minifilter::FltParseFileNameInformation;

    const _FLT_RELEASE_FILE_NAME_INFORMATION: unsafe extern "C" fn(PFLT_FILE_NAME_INFORMATION) =
        minifilter::FltReleaseFileNameInformation;
}
