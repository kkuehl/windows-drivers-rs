// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! Fallible-allocation shared ownership.
//!
//! # Why this is not just a re-export
//!
//! [`alloc::sync::Arc::new`] allocates infallibly: on failure it reaches
//! [`alloc::alloc::handle_alloc_error`], which in a driver is `KeBugCheckEx`.
//! The fallible constructor `Arc::try_new` is still unstable — it sits behind
//! `allocator_api` — so on a stable toolchain there is *no* way to construct an
//! [`alloc::sync::Arc`] that reports allocation failure to its caller.
//!
//! The near-miss workarounds do not work either, and it is worth recording why
//! so they are not retried:
//!
//! * `Arc::from(try_box(value)?)` still allocates. `From<Box<T>>` has to build
//!   an `ArcInner` to hold the strong and weak counts and move the value into
//!   it, and that second allocation is infallible.
//! * Reserving a one-element [`alloc::vec::Vec`] with
//!   [`alloc::vec::Vec::try_reserve_exact`] and calling `into_boxed_slice` is
//!   not reliable either. `try_reserve_exact` is documented to be allowed to
//!   over-allocate, and when capacity exceeds length `into_boxed_slice` shrinks
//!   — an infallible reallocation.
//!
//! So [`Arc`] here is a purpose-built strong-count-only pointer. It is
//! deliberately a small fraction of [`alloc::sync::Arc`]: the whole point is
//! that a reviewer can read all of it.
//!
//! # Differences from [`alloc::sync::Arc`]
//!
//! * [`Arc::try_new`] replaces `new`, and reports failure.
//! * `T` must be [`Sized`]. `Arc<[T]>` and `Arc<dyn Trait>` need fat-pointer
//!   handling that no current consumer needs.
//! * There is no `Weak`, and so no reference cycle collection. A cycle leaks.
//! * There is no `get_mut` or `make_mut`. Interior mutability is the caller's
//!   job — wrap the value in a lock.

use core::{
    fmt,
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering, fence},
};

use crate::fallible::{AllocError, try_box};

/// The heap allocation an [`Arc`] and all of its clones share.
struct Inner<T> {
    /// Number of live [`Arc`] handles pointing at this allocation.
    strong: AtomicUsize,
    /// The shared value.
    value: T,
}

/// A thread-safe reference-counting pointer whose construction can fail.
///
/// This is the fallible-allocation counterpart to [`alloc::sync::Arc`], for use
/// in driver code where an infallible allocation is a bugcheck path. See the
/// [module documentation](self) for why a re-export will not do and for the
/// list of intentional differences.
///
/// # Examples
///
/// ```
/// use wdk_alloc::{fallible::AllocError, sync::Arc};
///
/// let shared = Arc::try_new(41u32)?;
/// let alias = Arc::clone(&shared);
///
/// assert_eq!(*alias + 1, 42);
/// assert_eq!(Arc::strong_count(&shared), 2);
/// assert!(Arc::ptr_eq(&shared, &alias));
/// # Ok::<(), AllocError>(())
/// ```
pub struct Arc<T> {
    /// Pointer to the shared allocation. Owns one unit of `Inner::strong`.
    ptr: NonNull<Inner<T>>,
}

impl<T> Arc<T> {
    /// Allocates a new `Arc` holding `value`.
    ///
    /// # Errors
    ///
    /// Returns [`AllocError`] if the allocation failed, rather than
    /// bugchecking.
    pub fn try_new(value: T) -> Result<Self, AllocError> {
        let inner = try_box(Inner {
            strong: AtomicUsize::new(1),
            value,
        })?;

        // `Box::leak` hands out the allocation without freeing it and yields a
        // reference, so the raw pointer is obtained without an `unsafe` block.
        // `Drop` below is what reclaims it once the strong count reaches zero.
        Ok(Self {
            ptr: NonNull::from(alloc::boxed::Box::leak(inner)),
        })
    }

    /// Returns the number of live handles sharing this allocation.
    ///
    /// This is a snapshot. Another thread may clone or drop a handle before the
    /// caller can act on the value, so it is only meaningful for diagnostics or
    /// when the caller already knows no other thread holds a handle.
    #[must_use]
    pub fn strong_count(this: &Self) -> usize {
        this.inner().strong.load(Ordering::Relaxed)
    }

    /// Returns `true` if both handles point at the same allocation.
    #[must_use]
    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        core::ptr::eq(this.ptr.as_ptr(), other.ptr.as_ptr())
    }

    /// Borrows the shared allocation.
    const fn inner(&self) -> &Inner<T> {
        // SAFETY: `self.ptr` was produced from a `Box` in `try_new` and is never
        // reassigned. This handle owns one unit of the strong count, and the
        // allocation is only freed when that count reaches zero, so the pointee
        // is live for at least `&self`.
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner().value
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        // `Relaxed` is sufficient for an increment: the new handle is derived from an
        // existing one, so the allocation is already known to be live and no data
        // written before the clone needs to be published by this operation.
        // Ordering is only required on the decrement in `Drop`, which is where
        // the value is read for the last time.
        //
        // The count is not checked for overflow, which is where this differs from
        // `alloc::sync::Arc` -- it aborts, and a driver has no abort, only a bugcheck.
        // Overflowing requires 2^64 increments that are never paired with a decrement
        // (`core::mem::forget` in a loop); at one increment per nanosecond that is over
        // five hundred years of uninterrupted uptime, so it is not reachable in the
        // lifetime of a booted kernel.
        self.inner().strong.fetch_add(1, Ordering::Relaxed);
        Self { ptr: self.ptr }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        // `Release` pairs with the `Acquire` fence below so that everything this thread
        // did with the value happens-before the thread that observes the count
        // reaching zero runs the destructor.
        if self.inner().strong.fetch_sub(1, Ordering::Release) != 1 {
            return;
        }

        // This thread took the count from one to zero, so it is the sole remaining
        // owner. The fence makes every other thread's prior use of the value
        // visible here before the destructor runs, which is what stops the drop
        // from racing a reader.
        fence(Ordering::Acquire);

        // SAFETY: the strong count reached zero, so no other handle exists and none can
        // be created (`Clone` requires a live handle). The pointer came from
        // `Box::leak` on a `Box<Inner<T>>` in `try_new`, so reconstituting that same
        // `Box` reclaims it with the layout it was allocated with, and dropping
        // it runs `T`'s destructor exactly once.
        drop(unsafe { alloc::boxed::Box::from_raw(self.ptr.as_ptr()) });
    }
}

// SAFETY: `Arc<T>` hands out `&T` to every thread holding a handle, and moving
// a handle to another thread can move `T`'s destructor there too, so sharing
// requires `T: Sync` and transferring requires `T: Send` -- the same bounds
// `alloc::sync::Arc` carries. The strong count itself is an `AtomicUsize`, so
// the refcounting is race-free independently of `T`.
unsafe impl<T: Send + Sync> Send for Arc<T> {}

// SAFETY: see the `Send` impl above; `&Arc<T>` allows another thread to clone
// the handle and read `T`, which is exactly what `T: Send + Sync` licenses.
unsafe impl<T: Send + Sync> Sync for Arc<T> {}

impl<T: fmt::Debug> fmt::Debug for Arc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T> AsRef<T> for Arc<T> {
    fn as_ref(&self) -> &T {
        self
    }
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::AtomicUsize;

    use super::*;

    /// Increments a shared counter when dropped, so tests can prove the
    /// destructor runs exactly once.
    struct DropWitness<'a>(&'a AtomicUsize);

    impl Drop for DropWitness<'_> {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn try_new_then_deref_yields_the_value() {
        let shared = Arc::try_new(0xFEED_u32).expect("small allocation should succeed");
        assert_eq!(*shared, 0xFEED);
    }

    #[test]
    fn clone_shares_one_allocation() {
        let shared = Arc::try_new(7_u32).expect("small allocation should succeed");
        let alias = Arc::clone(&shared);

        assert!(Arc::ptr_eq(&shared, &alias));
        assert_eq!(Arc::strong_count(&shared), 2);
        // Both handles must observe writes through the same allocation.
        assert_eq!(*shared, *alias);
    }

    #[test]
    fn dropping_a_clone_leaves_the_original_usable() {
        let shared = Arc::try_new(9_u32).expect("small allocation should succeed");
        let alias = Arc::clone(&shared);
        assert_eq!(Arc::strong_count(&shared), 2);

        drop(alias);

        assert_eq!(Arc::strong_count(&shared), 1);
        assert_eq!(*shared, 9);
    }

    #[test]
    fn value_is_dropped_once_when_the_last_handle_goes() {
        let drops = AtomicUsize::new(0);

        {
            let shared = Arc::try_new(DropWitness(&drops)).expect("small allocation");
            let alias = Arc::clone(&shared);
            let third = Arc::clone(&alias);
            assert_eq!(Arc::strong_count(&shared), 3);
            // Nothing may be dropped while any handle is alive.
            assert_eq!(drops.load(Ordering::SeqCst), 0);
            drop(alias);
            drop(third);
            assert_eq!(drops.load(Ordering::SeqCst), 0);
        }

        // Exactly once -- not zero (a leak) and not three (a double free).
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn distinct_allocations_are_not_ptr_eq() {
        let first = Arc::try_new(1_u32).expect("small allocation should succeed");
        let second = Arc::try_new(1_u32).expect("small allocation should succeed");

        // Equal values, different allocations.
        assert_eq!(*first, *second);
        assert!(!Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn holds_a_zero_sized_value() {
        // A ZST takes `try_box`'s non-allocating path; the refcounting must still work,
        // since `Inner<()>` is itself non-zero-sized because of the count.
        let shared = Arc::try_new(()).expect("zero-sized allocation cannot fail");
        let alias = Arc::clone(&shared);
        assert_eq!(Arc::strong_count(&shared), 2);
        drop(alias);
        assert_eq!(Arc::strong_count(&shared), 1);
    }

    #[test]
    fn debug_forwards_to_the_value() {
        let shared = Arc::try_new(5_u32).expect("small allocation should succeed");
        assert_eq!(alloc::format!("{shared:?}"), "5");
    }
}
