#![allow(dead_code)]

use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicUsize, Ordering};

pub struct LockCell<T: ?Sized> {
    ticket: AtomicUsize,

    release: AtomicUsize,

    val: UnsafeCell<T>,
}
unsafe impl<T: ?Sized> Sync for LockCell<T> {}

impl<T> LockCell<T> {
    pub const fn new(val: T) -> Self {
        LockCell {
            val: UnsafeCell::new(val),
            ticket: AtomicUsize::new(0),
            release: AtomicUsize::new(0),
        }
    }
}

impl<T: ?Sized> LockCell<T> {
    #[track_caller]
    pub fn lock(&self) -> LockCellGuard<T> {
        // Get a ticket
        let ticket = self.ticket.fetch_add(1, Ordering::SeqCst);

        let mut i = 0;

        // Spin while our ticket doesn't match the release
        while self.release.load(Ordering::SeqCst) != ticket {
            if i == 1_000_000 {
                panic!("Waited too long to lock!");
            }

            spin_loop();
            i += 1;
        }

        // At this point we have exclusive access
        LockCellGuard { cell: self }
    }
}

pub struct LockCellGuard<'a, T: ?Sized> {
    cell: &'a LockCell<T>,
}

impl<'a, T: ?Sized> LockCellGuard<'a, T> {
    pub unsafe fn release_lock(&self) {
        // Release the lock
        self.cell.release.fetch_add(1, Ordering::SeqCst);
    }
}

impl<'a, T: ?Sized> Drop for LockCellGuard<'a, T> {
    fn drop(&mut self) {
        // Release the lock
        self.cell.release.fetch_add(1, Ordering::SeqCst);
    }
}

impl<'a, T: ?Sized> Deref for LockCellGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.cell.val.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for LockCellGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.cell.val.get() }
    }
}
