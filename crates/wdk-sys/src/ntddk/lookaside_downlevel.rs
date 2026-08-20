// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! `ExAllocateFromLookasideListEx` and `ExFreeToLookasideListEx`, which cannot
//! be bound as plain externs without raising every consumer's minimum OS to
//! Windows 11 22H2.
//!
//! # The problem these solve
//!
//! `10.0.26100.0/km/wdm.h:26324` is `#if (NTDDI_VERSION >= NTDDI_WIN10_NI)`.
//! Above it the two routines are `NTKERNELAPI` declarations
//! (`10.0.26100.0/km/wdm.h:26331` and `:26338`); below it, in the `#else` at
//! `:26343-26433`, the WDK supplies `FORCEINLINE` bodies instead.
//! `NTDDI_WIN10_NI` is Windows 11 22H2 / Windows Server 2025.
//!
//! Every WDK citation in this file names the header version it was read from,
//! because the line numbers move by roughly a thousand lines between WDKs: the
//! same gate is at `10.0.22621.0/km/wdm.h:25341`. An unqualified `km/wdm.h:NNN`
//! resolves to unrelated code against any other kit, which for this file is a
//! hazard rather than an inconvenience — the justification below is the only
//! thing standing between a reader and "simplifying" these two back into plain
//! externs, which would silently restore the load floor this module exists to
//! remove.
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
//! caches the result. That is the pattern Microsoft documents for calling a DDI
//! that may be absent on an older kernel while keeping the image loadable on
//! it.
//!
//! - **Resolved** — the kernel is Windows 11 22H2 or later, which is every
//!   kernel a consumer of this crate could previously load on at all. The
//!   exported routine is called, so behaviour is byte for byte what it was.
//! - **Unresolved** — the kernel is older. The lookaside list degenerates to a
//!   100%-miss cache: every allocation goes to `L.AllocateEx` and every free to
//!   `L.FreeEx`, which are the callbacks `ExInitializeLookasideListEx`
//!   installed and are exactly what the WDK's own `FORCEINLINE` body calls on a
//!   miss (`10.0.26100.0/km/wdm.h:26382-26385` and `:26424`). Correct, and
//!   slower by one pool round trip per allocation.
//!
//! # What is deliberately not done, and why
//!
//! The `FORCEINLINE` bodies also keep an `SLIST` cache in
//! `Lookaside->L.ListHead`, and this does not reproduce it, so the miss arm
//! above is taken on every operation rather than only on a genuine miss.
//!
//! Two reasons previously given for deferring that do not survive checking, and
//! are corrected here rather than deleted so that the next reader does not
//! re-derive them:
//!
//! - It is **not** true that restoring the cache requires reading a bitfield
//!   through a union. `#define ExQueryDepthSList(_listhead_)
//!   (_listhead_)->Depth` is the **x86** arm: it sits in the `#else` of
//!   `#if !defined(_X86_)` at `10.0.26100.0/km/wdm.h:26104`, with the macro at
//!   `:26154`. On x64, with `_NTDDK_`/`_NTIFS_` defined as a kernel driver
//!   build defines them (`:26106`), `ExQueryDepthSList` is an ordinary
//!   `NTKERNELAPI USHORT ExQueryDepthSList(PSLIST_HEADER)` export at
//!   `:26108-26112` — which is why the generated `ntddk` bindings already
//!   contain it, as they do `ExpInterlockedPopEntrySList` and
//!   `ExpInterlockedPushEntrySList` (`:26197` and `:26203`, the two the
//!   `InterlockedPop/PushEntrySList` macros expand to at `:26184-26188`). No
//!   union access is involved.
//! - It is **not** true that the cache would only buy speed on kernels a
//!   consumer cannot load on anyway. That was the position *before* this
//!   module; removing these two imports is what made the pre-NI window
//!   loadable. Measured on the downstream minifilter this was written for: no
//!   symbol left in its release import table is declared inside an
//!   `NTDDI_VERSION >= NTDDI_WIN10_NI` gate, so its floor is now set by
//!   `ExAllocatePool2` (Windows 10 2004) and the pre-NI window it can load on
//!   spans Windows 10 2004 through 22H1 **and Windows Server 2022**. The miss
//!   arm is therefore live on mainstream supported kernels, on that driver's
//!   hottest path.
//!
//! What remains true is that the SLIST arm cannot be exercised without a pre-NI
//! machine, and that a wrong push or pop corrupts the list's own free chain. So
//! it is still sequenced after this change rather than bundled into it — but on
//! that ground alone. The counters below are maintained so that `!lookaside` in
//! a debugger shows the miss rate that motivates it.

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

/// `ExAllocateFromLookasideListEx`, as `10.0.26100.0/km/wdm.h:26326-26333`
/// declares it.
type AllocateFromLookaside = unsafe extern "C" fn(PLOOKASIDE_LIST_EX) -> PVOID;

/// `ExFreeToLookasideListEx`, as `10.0.26100.0/km/wdm.h:26335-26341` declares
/// it.
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
/// (`10.0.26100.0/km/wdm.h:13974`) and both lookaside routines are
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
/// `10.0.26100.0/km/wdm.h:26326-26333` when the kernel exports it, and the
/// miss arm of `:26374-26389` when it does not. See the module documentation for
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

    // `10.0.26100.0/km/wdm.h:26378` and `:26381`. Both counters are `ULONG` and the WDK
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
    // `10.0.26100.0/km/wdm.h:26382-26385`
    let allocate = unsafe { list.__bindgen_anon_4.AllocateEx };

    let Some(allocate) = allocate else {
        // `ExInitializeLookasideListEx` installs a default when the caller passes
        // none, so this is unreachable for an initialised list. Returning null rather
        // than calling through it is what keeps that unreachability from being a
        // bugcheck if it is ever wrong -- a failed lookaside allocation is a status
        // every caller already handles.
        return core::ptr::null_mut();
    };

    // `10.0.26100.0/km/wdm.h:26382-26385`, argument for argument.
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
/// `10.0.26100.0/km/wdm.h:26335-26341` when the kernel exports it, and the
/// miss arm of `:26419-26431` when it does not.
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

    // `10.0.26100.0/km/wdm.h:26421` and `:26423`.
    list.TotalFrees = list.TotalFrees.wrapping_add(1);

    // SAFETY: `FreeMisses` and `FreeHits` are the two arms of one anonymous union
    // and both are `ULONG`
    unsafe {
        list.__bindgen_anon_3.FreeMisses = list.__bindgen_anon_3.FreeMisses.wrapping_add(1);
    }

    // SAFETY: `FreeEx` and `Free` are the two arms of one anonymous union;
    // `ExInitializeLookasideListEx` fills the `Ex` form, which is the arm the WDK's
    // own body calls with two arguments at `10.0.26100.0/km/wdm.h:26424`
    let free = unsafe { list.__bindgen_anon_5.FreeEx };

    let Some(free) = free else {
        // Unreachable for an initialised list, as in the allocate path. Leaking the
        // block is the only safe response: the alternative is guessing which pool it
        // came from and calling `ExFreePool` on a block the kernel may own.
        return;
    };

    // `10.0.26100.0/km/wdm.h:26424`.
    // SAFETY: `free` is the non-null callback the kernel installed in this
    // descriptor, and `Entry` came from the matching allocate per this function's
    // contract
    unsafe { free(Entry, Lookaside) };
}
