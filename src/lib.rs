#![cfg_attr(not(test), no_std)]
extern crate alloc;

#[cfg(test)]
extern crate self as ptask;

use core::future::Future;

mod task;
pub use task::ptask;

/// Spawns a new task to run the provided future.
#[inline(always)]
pub fn spawn<Fut: Future<Output = ()> + Send + 'static>(task: Fut) {
    ptask(task).wake()
}

#[cfg(test)]
mod tests;
