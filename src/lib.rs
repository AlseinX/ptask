#![cfg_attr(not(test), no_std)]
extern crate alloc;

#[cfg(test)]
extern crate self as ptask;

#[cfg(not(test))]
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::{future::Future, pin::Pin};

mod task;

/// Spawns a new task to run the provided future.
#[inline(always)]
pub fn spawn<Fut: Future<Output = ()> + Send + 'static>(task: Fut) {
    spawn_pinned(Box::pin(task))
}

/// Spawns a new task to run the provided pinned dynamic future.
#[cfg_attr(feature = "inlining", inline(always))]
pub fn spawn_pinned(task: Pin<Box<dyn Future<Output = ()> + Send>>) {
    Arc::new(task::Task::new(task)).dispatch();
}

#[cfg(test)]
mod tests;
