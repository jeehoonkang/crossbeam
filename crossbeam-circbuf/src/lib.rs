//! Concurrent queues based on circular buffer.
//!
//! Currently, this crate provides the following flavors of queues:
//!
//! - bounded/unbounded SPSC (single-producer single-consumer)
//! - bounded/unbounded SPMC (single-producer multiple-consumer)

#![warn(missing_docs, missing_debug_implementations)]

extern crate crossbeam_epoch as epoch;
extern crate crossbeam_utils as utils;

mod buffer;

#[doc(hidden)] // for doc-tests
pub mod sp;
#[doc(hidden)] // for doc-tests
pub mod mp;

pub use sp::mc as spmc;
pub use sp::sc as spsc;
pub use mp::mc as mpmc;

/// The return type for `try_recv` methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryRecv<T> {
    /// Received a value.
    Data(T),
    /// Not received a value because the buffer is empty.
    Empty,
    /// Lost the race to a concurrent operation. Try again.
    Retry,
}

impl<T> TryRecv<T> {
    /// Applies a function to the content of `TryRecv::Data`.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> TryRecv<U> {
        match self {
            TryRecv::Data(v) => TryRecv::Data(f(v)),
            TryRecv::Empty => TryRecv::Empty,
            TryRecv::Retry => TryRecv::Retry,
        }
    }
}
