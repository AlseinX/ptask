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
