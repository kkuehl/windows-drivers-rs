// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

pub use bindings::*;

#[allow(missing_docs)]
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[rustversion::attr(
    any(
        all(not(nightly), since(1.88)),
        all(nightly, since(2025-04-25)),
    ),
    allow(unnecessary_transmutes)
)]
#[allow(unsafe_op_in_unsafe_fn)]
#[allow(clippy::cast_lossless)]
#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_possible_wrap)]
#[allow(clippy::cognitive_complexity)]
#[allow(clippy::doc_markdown)]
#[allow(clippy::default_trait_access)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[rustversion::attr(
    any(
        all(not(nightly), before(1.74)),
        all(nightly, before(2023-09-13)),
    ),
    allow(clippy::incorrect_clone_impl_on_copy_type)
)]
#[rustversion::attr(
    any(
        all(not(nightly), since(1.74)),
        all(nightly, since(2023-09-13)),
    ),
    allow(clippy::non_canonical_clone_impl)
)]
#[allow(clippy::missing_const_for_fn)]
#[allow(clippy::missing_safety_doc)]
#[allow(clippy::module_name_repetitions)]
#[allow(clippy::multiple_unsafe_ops_per_block)]
#[allow(clippy::must_use_candidate)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[allow(clippy::ptr_as_ptr)]
#[allow(clippy::ptr_offset_with_cast)]
#[rustversion::attr(
    any(
        all(not(nightly), since(1.77)),
        all(nightly, since(2024-01-11)),
    ),
    allow(clippy::pub_underscore_fields)
)]
#[rustversion::attr(
    any(
        all(not(nightly), since(1.78)),
        all(nightly, since(2024-02-09)),
    ),
    allow(clippy::ref_as_ptr)
)]
#[allow(clippy::semicolon_if_nothing_returned)]
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
#[allow(clippy::transmute_ptr_to_ptr)]
#[allow(clippy::undocumented_unsafe_blocks)]
#[allow(clippy::unnecessary_cast)]
#[allow(clippy::unreadable_literal)]
#[allow(clippy::used_underscore_binding)]
#[allow(clippy::useless_transmute)]
#[allow(clippy::use_self)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/types.rs"));
}

// MDL (Memory Descriptor List) types from wdm.h
//
// These types are used by the MDL functions in ntddk.rs. The MDL structure itself
// is opaque to drivers (only the kernel accesses its fields), so we define it as
// an opaque type.

/// Opaque structure describing a range of virtual memory pages.
///
/// Drivers should not access MDL fields directly. Use the MDL functions
/// (IoAllocateMdl, MmProbeAndLockPages, etc.) to manipulate MDLs.
#[repr(C)]
pub struct MDL {
    _opaque: [u8; 0],
}

/// Pointer to an MDL.
pub type PMDL = *mut MDL;

/// Specifies the type of access for which pages should be locked.
///
/// Used with `MmProbeAndLockPages`.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LOCK_OPERATION {
    /// Lock pages for read access.
    IoReadAccess = 0,
    /// Lock pages for write access.
    IoWriteAccess = 1,
    /// Lock pages for both read and write access.
    IoModifyAccess = 2,
}

// Page priority constants for MmGetSystemAddressForMdlSafe
/// Normal page priority (typical case).
pub const NORMAL_PAGE_PRIORITY: ULONG = 16;
/// Low page priority (can fail under memory pressure).
pub const LOW_PAGE_PRIORITY: ULONG = 0;
/// High page priority (should not fail except in extreme cases).
pub const HIGH_PAGE_PRIORITY: ULONG = 32;
