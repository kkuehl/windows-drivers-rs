// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! `ExAllocateFromLookasideListEx` and `ExFreeToLookasideListEx`, which cannot
//! be bound as plain externs without raising every consumer's minimum OS to
//! Windows 11 22H2.
//!
//! # The problem these solve
//!
//! `km/wdm.h:27592` is `#if (NTDDI_VERSION >= NTDDI_WIN10_NI)`. Above it the
//! two routines are `NTKERNELAPI` declarations (`km/wdm.h:27599` and `:27606`);
//! below it, in the `#else` at `km/wdm.h:27612-27700`, the WDK supplies
//! `FORCEINLINE` bodies instead. `NTDDI_WIN10_NI` is Windows 11 22H2 / Windows
//! Server 2025.
//!
//! MSVC honours the `FORCEINLINE` and emits no import, which is why a C driver
//! built against a current WDK still loads on Windows 10. bindgen cannot inline
//! a `FORCEINLINE`, so it emits ordinary externs — and an extern is an import
//! the kernel's image loader must resolve **before `DriverEntry` is entered**.
//! An image importing these two therefore fails to load on every build of
//! Windows 10 and on Server 2016, 2019 and 2022, with no trace and nothing in
//! any log attributing it.
//!
//! Measured on a downstream minifilter built against this crate: the two were
//! the highest gate in its import table, and the shipping C++ driver it is a
//! port of imports neither. So the asymmetry is an artefact of binding
//! generation rather than a design choice, and nothing in the Rust source of
//! either crate says so.
//!
//! `build.rs` blocklists the two functions from the `ntddk` bindings and
//! `ntddk.rs` re-exports these in their place, so consumers need no change.
//!
//! # How they behave
//!
//! Each resolves its own routine once, through `MmGetSystemRoutineAddress`, and
//! caches the result. That is the same pattern the WDK documents for optional
//! DDIs and the same one `ExInitializeDriverRuntime` uses internally for
//! `NtQuerySystemInformation` (`km/wdm.h:50917`).
//!
//! - **Resolved** — the kernel is Windows 11 22H2 or later, which is every
//!   kernel a consumer of this crate could previously load on at all. The
//!   exported routine is called, so behaviour is byte for byte what it was.
//! - **Unresolved** — the kernel is older. The lookaside list degenerates to a
//!   100%-miss cache: every allocation goes to `L.AllocateEx` and every free to
//!   `L.FreeEx`, which are the callbacks `ExInitializeLookasideListEx`
//!   installed and are exactly what the WDK's own `FORCEINLINE` body calls on a
//!   miss (`km/wdm.h:27650-27654` and `:27692-27694`). Correct, and slower by
//!   one pool round trip per allocation.
//!
//! # What is deliberately not done, and why
//!
//! The `FORCEINLINE` bodies also keep an `SLIST` cache in
//! `Lookaside->L.ListHead`, and this does not reproduce it. Restoring it needs
//! `ExQueryDepthSList`, which on x64 is
//! `#define ExQueryDepthSList(_listhead_) (_listhead_)->Depth`
//! (`km/wdm.h:27422`) over a `SLIST_HEADER` whose x64 layout is a union of
//! bitfields — `HeaderX64.Depth:16` — so a Rust transcription has to read a
//! bitfield through a union rather than a named field. That is the one step in
//! the transcription that can silently corrupt a system-wide structure, it
//! cannot be exercised without a pre-NI machine, and what it buys is *speed on
//! kernels a consumer cannot currently load on at all*. So it is correctly
//! sequenced after this change rather than bundled into it, and the counters
//! below are maintained so that `!lookaside` in a debugger still shows the miss
//! rate that would motivate it.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    ntddk::{KeGetCurrentIrql, MmGetSystemRoutineAddress},
    types::{PLOOKASIDE_LIST_EX, PVOID, UNICODE_STRING, USHORT, WCHAR},
};

/// Sentinel for "not looked up yet" in the caches below.
///
/// Zero cannot be a routine address, so it is free to mean "unknown".
const UNRESOLVED: usize = 0;

/// Sentinel for "looked up, and this kernel does not export it".
///
/// One is not a valid code address on any supported platform — kernel-mode code
/// lives in the upper half of the address space — so it cannot collide with a
/// real resolution. Distinguishing this from [`UNRESOLVED`] is what stops a
/// down-level kernel being re-probed on every allocation.
const ABSENT: usize = 1;

/// `NTDDI_VERSION >= NTDDI_WIN10_NI` build of `ExAllocateFromLookasideListEx`,
/// or [`UNRESOLVED`]/[`ABSENT`].
static ALLOCATE_FROM_LOOKASIDE: AtomicUsize = AtomicUsize::new(UNRESOLVED);

/// As [`ALLOCATE_FROM_LOOKASIDE`], for `ExFreeToLookasideListEx`.
static FREE_TO_LOOKASIDE: AtomicUsize = AtomicUsize::new(UNRESOLVED);

/// `ExAllocateFromLookasideListEx`, as `km/wdm.h:27596-27600` declares it.
type AllocateFromLookaside = unsafe extern "C" fn(PLOOKASIDE_LIST_EX) -> PVOID;

/// `ExFreeToLookasideListEx`, as `km/wdm.h:27603-27608` declares it.
type FreeToLookaside = unsafe extern "C" fn(PLOOKASIDE_LIST_EX, PVOID);

/// `"ExAllocateFromLookasideListEx"` as UTF-16, for
/// `MmGetSystemRoutineAddress`.
///
/// A `static` rather than a local because `MmGetSystemRoutineAddress` takes a
/// `UNICODE_STRING` that points *at* the buffer and reads it during the call; a
/// local would be fine for that, but a `static` is also what makes the
/// descriptor cheap to build and impossible to get wrong.
static ALLOCATE_NAME: [WCHAR; 29] = wide(b"ExAllocateFromLookasideListEx");

/// `"ExFreeToLookasideListEx"` as UTF-16.
static FREE_NAME: [WCHAR; 23] = wide(b"ExFreeToLookasideListEx");

/// Widens an ASCII routine name at compile time.
///
/// A `const fn` so the two names above are data in the image rather than work
/// at run time, and so a non-ASCII byte is a build failure rather than a lookup
/// that silently misses.
const fn wide<const N: usize>(name: &[u8]) -> [WCHAR; N] {
    assert!(name.len() == N, "buffer length should match the name");

    let mut wide = [0; N];
    let mut index = 0;
    while index < N {
        assert!(name[index] < 0x80, "a routine name should be ASCII");
        wide[index] = name[index] as WCHAR;
        index += 1;
    }

    wide
}

/// Builds the `UNICODE_STRING` `MmGetSystemRoutineAddress` wants over `name`.
///
/// The `Length` is a byte count, which is what a `UNICODE_STRING` counts, and
/// the buffer is deliberately not NUL terminated: a `UNICODE_STRING` is
/// explicitly counted.
fn name_of(name: &'static [WCHAR]) -> UNICODE_STRING {
    // `name` is one of the two `static`s above, both far shorter than
    // `USHORT::MAX`, so this cannot truncate.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "both callers pass a `static` of fewer than 30 elements, so the byte count is \
                  under 60 and cannot exceed `USHORT::MAX`"
    )]
    let length_in_bytes = (name.len() * size_of::<WCHAR>()) as USHORT;

    UNICODE_STRING {
        Length: length_in_bytes,
        MaximumLength: length_in_bytes,
        Buffer: name.as_ptr().cast_mut(),
    }
}

/// Resolves `name` through `MmGetSystemRoutineAddress`, caching the answer in
/// `cache`, and returns the address or [`ABSENT`].
///
/// # Why the IRQL is tested
///
/// `MmGetSystemRoutineAddress` is `_IRQL_requires_max_(PASSIVE_LEVEL)`
/// (`km/wdm.h:14671`) and both lookaside routines are
/// `_IRQL_requires_max_(DISPATCH_LEVEL)`, so a caller may legitimately be above
/// `PASSIVE_LEVEL`. Probing there would be the IRQL violation; taking the
/// down-level path for that one call is merely slow, and a later call at
/// `PASSIVE_LEVEL` populates the cache for good.
///
/// This is what makes the shims self-contained: there is no initialisation a
/// consumer has to remember to call from `DriverEntry`, and therefore no way to
/// get an unresolved-pointer call by forgetting it.
fn resolve(cache: &AtomicUsize, name: &'static [WCHAR]) -> usize {
    let cached = cache.load(Ordering::Relaxed);
    if cached != UNRESOLVED {
        return cached;
    }

    // SAFETY: `KeGetCurrentIrql` reads the current processor's IRQL and takes no
    // arguments
    let irql = unsafe { KeGetCurrentIrql() };

    // `PASSIVE_LEVEL` is 0 (`km/wdm.h`), and `crate::constants::PASSIVE_LEVEL` is a
    // `u32` while `KIRQL` is a `UCHAR`, so the comparison is written against 0
    // rather than through a cast that could hide a widening.
    if irql != 0 {
        return ABSENT;
    }

    let mut routine_name = name_of(name);

    // SAFETY: `routine_name` is a valid `UNICODE_STRING` over a `static` buffer
    // that outlives the call, and the IRQL test above establishes the
    // `PASSIVE_LEVEL` precondition
    let address = unsafe { MmGetSystemRoutineAddress(&raw mut routine_name) };

    let resolved = if address.is_null() {
        ABSENT
    } else {
        address as usize
    };

    // `Relaxed` is sufficient: the value published is a plain integer with no
    // memory it guards, two threads resolving concurrently compute the same
    // answer, and a store lost to a race merely costs one more lookup.
    cache.store(resolved, Ordering::Relaxed);

    resolved
}

/// Removes (pops) the first entry from the specified lookaside list.
///
/// `km/wdm.h:27596-27600` when the kernel exports it, and the miss arm of
/// `km/wdm.h:27619-27657` when it does not. See the module documentation for
/// why this is not a plain extern.
///
/// # Safety
///
/// As `ExAllocateFromLookasideListEx`: `Lookaside` must point to a
/// `LOOKASIDE_LIST_EX` initialised by `ExInitializeLookasideListEx` and not yet
/// deleted, and the caller must be at `IRQL <= DISPATCH_LEVEL`.
#[expect(
    non_snake_case,
    reason = "bindings in this crate retain their original C names, and this one replaces a \
              bindgen-generated extern of the same name"
)]
#[must_use]
pub unsafe fn ExAllocateFromLookasideListEx(Lookaside: PLOOKASIDE_LIST_EX) -> PVOID {
    let resolved = resolve(&ALLOCATE_FROM_LOOKASIDE, &ALLOCATE_NAME);

    if resolved != ABSENT {
        // SAFETY: `resolved` is neither `UNRESOLVED` nor `ABSENT`, so it is the address
        // `MmGetSystemRoutineAddress` returned for `ExAllocateFromLookasideListEx`,
        // whose declared signature is `AllocateFromLookaside`
        let routine = unsafe { core::mem::transmute::<usize, AllocateFromLookaside>(resolved) };

        // SAFETY: the caller's contract is this routine's contract, unchanged
        return unsafe { routine(Lookaside) };
    }

    // SAFETY: `Lookaside` points to a live `LOOKASIDE_LIST_EX` per this function's
    // contract, so `L` is a valid `GENERAL_LOOKASIDE_POOL`
    let list = unsafe { &mut (*Lookaside).L };

    // `km/wdm.h:27643` and `:27649`. Both counters are `ULONG` and the WDK
    // increments them unsynchronized too -- they are diagnostics for `!lookaside`,
    // not accounting -- so a wrap is the WDK's behaviour and not a defect here.
    list.TotalAllocates = list.TotalAllocates.wrapping_add(1);

    // SAFETY: `AllocateMisses` and `AllocateHits` are the two arms of the same
    // anonymous union and both are `ULONG`, so reading and writing either is valid
    // whichever the kernel last wrote
    unsafe {
        list.__bindgen_anon_2.AllocateMisses = list.__bindgen_anon_2.AllocateMisses.wrapping_add(1);
    }

    // SAFETY: `AllocateEx` and `Allocate` are the two arms of one anonymous union;
    // `ExInitializeLookasideListEx` is documented to fill the `Ex` form, which is
    // also the arm the WDK's own `FORCEINLINE` body calls with four arguments at
    // `km/wdm.h:27650-27654`
    let allocate = unsafe { list.__bindgen_anon_4.AllocateEx };

    let Some(allocate) = allocate else {
        // `ExInitializeLookasideListEx` installs a default when the caller passes
        // none, so this is unreachable for an initialised list. Returning null rather
        // than calling through it is what keeps that unreachability from being a
        // bugcheck if it is ever wrong -- a failed lookaside allocation is a status
        // every caller already handles.
        return core::ptr::null_mut();
    };

    // `km/wdm.h:27650-27654`, argument for argument.
    // SAFETY: `allocate` is the non-null callback the kernel installed in this
    // descriptor, and the four arguments are the descriptor's own `Type`, `Size`
    // and `Tag` plus the descriptor itself, which is what the WDK passes
    unsafe {
        allocate(
            list.Type,
            list.Size as crate::types::SIZE_T,
            list.Tag,
            Lookaside,
        )
    }
}

/// Inserts (pushes) the specified entry into the specified lookaside list.
///
/// `km/wdm.h:27603-27608` when the kernel exports it, and the miss arm of
/// `km/wdm.h:27662-27700` when it does not.
///
/// # Safety
///
/// As `ExFreeToLookasideListEx`: `Lookaside` must point to a live
/// `LOOKASIDE_LIST_EX`, `Entry` must be a block obtained from that same list,
/// and the caller must be at `IRQL <= DISPATCH_LEVEL`.
#[expect(
    non_snake_case,
    reason = "bindings in this crate retain their original C names, and this one replaces a \
              bindgen-generated extern of the same name"
)]
pub unsafe fn ExFreeToLookasideListEx(Lookaside: PLOOKASIDE_LIST_EX, Entry: PVOID) {
    let resolved = resolve(&FREE_TO_LOOKASIDE, &FREE_NAME);

    if resolved != ABSENT {
        // SAFETY: `resolved` is the address `MmGetSystemRoutineAddress` returned for
        // `ExFreeToLookasideListEx`, whose declared signature is `FreeToLookaside`
        let routine = unsafe { core::mem::transmute::<usize, FreeToLookaside>(resolved) };

        // SAFETY: the caller's contract is this routine's contract, unchanged
        unsafe { routine(Lookaside, Entry) };
        return;
    }

    // SAFETY: `Lookaside` points to a live `LOOKASIDE_LIST_EX` per this function's
    // contract
    let list = unsafe { &mut (*Lookaside).L };

    // `km/wdm.h:27687` and `:27690`.
    list.TotalFrees = list.TotalFrees.wrapping_add(1);

    // SAFETY: `FreeMisses` and `FreeHits` are the two arms of one anonymous union
    // and both are `ULONG`
    unsafe {
        list.__bindgen_anon_3.FreeMisses = list.__bindgen_anon_3.FreeMisses.wrapping_add(1);
    }

    // SAFETY: `FreeEx` and `Free` are the two arms of one anonymous union;
    // `ExInitializeLookasideListEx` fills the `Ex` form, which is the arm the WDK's
    // own body calls with two arguments at `km/wdm.h:27692-27694`
    let free = unsafe { list.__bindgen_anon_5.FreeEx };

    let Some(free) = free else {
        // Unreachable for an initialised list, as in the allocate path. Leaking the
        // block is the only safe response: the alternative is guessing which pool it
        // came from and calling `ExFreePool` on a block the kernel may own.
        return;
    };

    // `km/wdm.h:27692-27694`.
    // SAFETY: `free` is the non-null callback the kernel installed in this
    // descriptor, and `Entry` came from the matching allocate per this function's
    // contract
    unsafe { free(Entry, Lookaside) };
}
