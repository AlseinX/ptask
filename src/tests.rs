use futures::{channel::mpsc, StreamExt};
use std::{
    future,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    task::Poll,
    thread,
    time::Duration,
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

#[test]
fn exhaust_test() {
    let waker = ptask::into_waker(async {});
    waker.wake_by_ref();
    waker.wake_by_ref();
    waker.wake();
}

#[test]
fn race_test() {
    let lock = Arc::new(AtomicBool::new(false));
    let input = Arc::new(AtomicUsize::new(0));
    let output = Arc::new(AtomicUsize::new(0));
    let waker = ptask::into_waker({
        let lock = lock.clone();
        let input = input.clone();
        let output = output.clone();
        future::poll_fn(move |_| {
            lock.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .unwrap();
            let input = input.load(Ordering::Relaxed);
            if input == output.load(Ordering::Relaxed) + 1 {
                output.store(input, Ordering::Relaxed);
            }
            thread::sleep(Duration::from_millis(10));
            lock.compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
                .unwrap();
            Poll::Pending
        })
    });
    let tasks = (0..9)
        .map(|_| {
            let waker = waker.clone();
            let input = input.clone();
            thread::spawn(move || {
                for i in 0..10 {
                    _ = input.compare_exchange(i, i + 1, Ordering::SeqCst, Ordering::SeqCst);
                    waker.wake_by_ref();
                    if i < 9 {
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            })
        })
        .collect::<Vec<_>>();
    for task in tasks {
        task.join().unwrap();
    }
    assert!(output.load(Ordering::Relaxed) == 10)
}

#[test]
fn re_wake_test() {
    let i = Arc::new(AtomicUsize::new(0));
    let waker = ptask::into_waker(future::poll_fn({
        let i = i.clone();
        move |cx| {
            if i.fetch_add(1, Ordering::Relaxed) < 9 {
                cx.waker().wake_by_ref();
            }
            Poll::Pending
        }
    }));
    waker.wake();
    assert_eq!(i.load(Ordering::Relaxed), 10);
}

#[test]
fn drop_test() {
    struct DropGuard(Arc<AtomicBool>);
    impl Drop for DropGuard {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Relaxed);
        }
    }
    let drop = Arc::new(AtomicBool::new(false));
    let waker = ptask::into_waker({
        let drop = DropGuard(drop.clone());
        let mut i = 0;
        future::poll_fn(move |cx| {
            if i < 10 {
                cx.waker().clone().wake_by_ref();
                i += 1;
            }
            _ = drop
                .0
                .compare_exchange(false, false, Ordering::Relaxed, Ordering::Relaxed);
            Poll::Pending
        })
    });
    waker.wake();
    assert!(drop.load(Ordering::Relaxed));
}
