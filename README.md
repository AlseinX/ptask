# Parasitic Task

Parasitic tasks are tasks that are not scheduled by any executor, but are instead directly dispatched at once on the thread that invokes the waker.

This makes it possible to process asynchronous results just-in-place, without the need for queueing the subsequent operations on a separate thread. This is useful for situations where response latency is a critical issue, such as in high-frequency trading applications.

## Usage

Simply run a parasitic task with `ptask::spawn` which is immediately dispatched on the current thread, until it `await`s for some future that has not yet completed, typically receiving from an asynchronous channel.

Note that the task is ref-counted with `Arc` internally, as the asynchonous operations that are `await`ed must be holding the waker that keeps the task alive. Thus, the sender could be regarded as the owner of the task that is `await`ing for the receiver. Keep in mind that if there are multiple ptasks that holds the senders of each other, some senders has to be wrapped within `Weak` references to avoid circular references.

```rust
use futures::{channel::mpsc, StreamExt};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

// This demo shows how it works on a single thread.
#[test]
fn demo() {
    let (tx, mut rx) = mpsc::unbounded();
    let result = Arc::new(AtomicUsize::new(0));

    // Spawn a ptask that runs immediately until the first `await`.
    // Then the `await`ed channel holds the reference to the task.
    ptask::spawn({
        let result = result.clone();
        async move {
            while let Some(i) = rx.next().await {
                result.fetch_add(i, Ordering::Relaxed);
            }
            result.fetch_add(10, Ordering::Relaxed);
        }
    });

    for i in 0..10 {
        // Sending to the channel makes the ptask resumes running until the next `await`.
        tx.unbounded_send(i).unwrap();
        assert_ne!(result.load(Ordering::Relaxed), 55);
    }

    // Drop the sender so that the ptask breaks out of the loop.
    drop(tx);

    assert_eq!(result.load(Ordering::Relaxed), 55);
}
```

## Capabilities

This crate supports `no_std` but requires the `alloc` crate to be available.

## Optional features

This crate by default marks all functions as `#[inline(always)]`, which ensures extreme performance in desired cases. However, if code size is still a concern, you can disable this behaviour by disabling the default features, which inlines like normal crates.
