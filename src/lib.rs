#![cfg_attr(not(test), no_std)]
extern crate alloc;

#[cfg(test)]
extern crate self as ptask;

mod task;
pub use task::{into_waker, spawn};

#[cfg(test)]
mod tests;
