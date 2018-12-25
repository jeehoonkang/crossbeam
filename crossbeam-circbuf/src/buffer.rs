use std::cell::UnsafeCell;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::mem;

/// A slot in buffer.
#[derive(Debug)]
pub struct Slot<T> {
    index: AtomicUsize,
    data: UnsafeCell<mem::ManuallyDrop<T>>,
}

/// A buffer that holds values in a queue.
///
/// This is just a buffer---dropping an instance of this struct will *not* drop the internal values.
#[derive(Debug)]
pub struct Buffer<T> {
    /// Pointer to the allocated memory.
    ptr: *mut Slot<T>,

    /// Capacity of the buffer. Always a power of two.
    cap: usize,
}

impl<T> Buffer<T> {
    /// Allocates a new buffer with the specified capacity.
    pub fn new(cap: usize) -> Self {
        // `cap` should be a power of two.
        debug_assert_eq!(cap, cap.next_power_of_two());

        // Creates a buffer.
        let mut v = Vec::<Slot<T>>::with_capacity(cap);
        let ptr = v.as_mut_ptr();
        mem::forget(v);

        // Marks all entries invalid.
        unsafe {
            for i in 0..cap {
                // Index `i + 1` for the `i`-th entry is invalid; only the indexes of the form `i +
                // N * cap` is valid.
                (*ptr.offset(i as isize)).index = AtomicUsize::new(i + 1);
            }
        }

        Buffer { ptr, cap }
    }
}

impl<T> Drop for Buffer<T> {
    fn drop(&mut self) {
        unsafe { 
            drop(Vec::from_raw_parts(self.ptr, 0, self.cap));
        }
    }
}

impl<T> Buffer<T> {
    pub fn cap(&self) -> usize {
        self.cap
    }

    /// Returns a pointer to the slot at the specified `index`.
    pub unsafe fn at(&self, index: usize) -> *mut Slot<T> {
        // `array.size()` is always a power of two.
        self.ptr.offset((index & (self.cap - 1)) as isize)
    }

    /// Reads a value from the specified `index`.
    ///
    /// Returns `Some(v)` if `v` is at `index`; or `None` if there's no valid value for `index`.
    ///
    /// Using this concurrently with a `write` is technically speaking UB due to data races.  We
    /// should be using relaxed accesses, but that would cost too much performance.  Hence, as a
    /// HACK, we use volatile accesses instead.  Experimental evidence shows that this works.
    pub unsafe fn read(&self, index: usize) -> Option<mem::ManuallyDrop<T>> {
        let slot = self.at(index);

        // Reads the index with `Acquire`.
        let i = (*slot).index.load(Ordering::Acquire);

        // If the index in the buffer mismatches with the queried index, there's no valid value.
        if index != i {
            return None;
        }

        // Returns the value.
        Some((*slot).data.get().read_volatile())
    }

    /// Reads a value from the specified `index` without checking the index.
    ///
    /// Returns the value at `index` regardless or whether it's valid or not.
    ///
    /// Using this concurrently with a `write` is technically speaking UB due to data races.  We
    /// should be using relaxed accesses, but that would cost too much performance.  Hence, as a
    /// HACK, we use volatile accesses instead.  Experimental evidence shows that this works.
    pub unsafe fn read_unchecked(&self, index: usize) -> mem::ManuallyDrop<T> {
        let slot = self.at(index);

        // Returns the value.
        (*slot).data.get().read_volatile()
    }

    /// Writes `value` into the specified `index`.
    ///
    /// Using this concurrently with another `read` or `write` is technically
    /// speaking UB due to data races.  We should be using relaxed accesses, but
    /// that would cost too much performance.  Hence, as a HACK, we use volatile
    /// accesses instead.  Experimental evidence shows that this works.
    pub unsafe fn write(&self, index: usize, value: T) {
        let slot = self.at(index);

        // Writes the value.
        (*slot).data.get().write_volatile(mem::ManuallyDrop::new(value));

        // Writes the index with `Release`.
        (*slot).index.store(index, Ordering::Release);
    }
}
