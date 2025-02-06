use alloc::sync::Arc;
use core::{
    cell::UnsafeCell,
    future::Future,
    mem::ManuallyDrop,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, RawWaker, RawWakerVTable, Waker},
};

/// Creates a new task as a waker, wrapping the provided future.
#[inline(always)]
pub fn into_waker<F>(task: F) -> Waker
where
    F: Future<Output = ()> + Send + 'static,
{
    let ptr = Arc::into_raw(Arc::new(Task::new(task))) as *const ();
    unsafe { Waker::from_raw(RawWaker::new(ptr, Task::<F>::TASK_V_TABLE)) }
}

/// Spawns a new task to run the provided future.
#[inline(always)]
pub fn spawn<F>(task: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let task = Arc::new(Task::new(task));
    unsafe { Task::arc_dispatch(task.as_ref()) };
}

struct Task<F> {
    /// Enum value:
    /// - 0: Available for dispatching
    /// - 1: Dispatching
    /// - 2: Waken while Dispatching
    /// - 3: Exhausted
    status: AtomicUsize,
    data: UnsafeCell<F>,
}

impl<F> Task<F> {
    #[inline(always)]
    pub fn new(task: F) -> Self {
        Self {
            status: AtomicUsize::new(0),
            data: UnsafeCell::new(task),
        }
    }
}

impl<F> Task<F>
where
    F: Future<Output = ()> + Send + 'static,
{
    #[inline(always)]
    unsafe fn arc_dispatch(this: *const Self) {
        unsafe {
            let mut weak = false;
            loop {
                match (*this)
                    .status
                    .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
                {
                    Ok(_) => {}
                    Err(1) if !weak => {
                        if (*this).status.compare_exchange(
                            1,
                            2,
                            Ordering::Relaxed,
                            Ordering::Acquire,
                        ) != Err(0)
                        {
                            return;
                        }
                    }
                    _ => return,
                }
                let mut task = Pin::new_unchecked(&mut *(*this).data.get());
                let waker = ManuallyDrop::new(Waker::from_raw(RawWaker::new(
                    this as _,
                    Self::TASK_V_TABLE,
                )));
                let mut cx = Context::from_waker(&waker);
                if task.as_mut().poll(&mut cx).is_ready() {
                    (*this).status.store(3, Ordering::Release);
                } else if (*this).status.compare_exchange(
                    1,
                    0,
                    Ordering::Release,
                    Ordering::Relaxed,
                ) == Err(2)
                {
                    (*this).status.store(0, Ordering::Relaxed);
                    weak = true;
                    continue;
                }
                return;
            }
        }
    }

    const TASK_V_TABLE: &RawWakerVTable = {
        #[inline(always)]
        unsafe fn v_clone<F>(this: *const ()) -> RawWaker
        where
            F: Future<Output = ()> + Send + 'static,
        {
            unsafe { Arc::increment_strong_count(this as *const Task<F>) };
            RawWaker::new(this, Task::<F>::TASK_V_TABLE)
        }

        #[inline(always)]
        unsafe fn v_wake<F>(this: *const ())
        where
            F: Future<Output = ()> + Send + 'static,
        {
            unsafe {
                v_wake_by_ref::<F>(this);
                v_drop::<F>(this);
            }
        }

        #[inline(always)]
        unsafe fn v_wake_by_ref<F>(this: *const ())
        where
            F: Future<Output = ()> + Send + 'static,
        {
            unsafe { Task::<F>::arc_dispatch(this as _) };
        }

        #[inline(always)]
        unsafe fn v_drop<F>(this: *const ()) {
            unsafe { Arc::decrement_strong_count(this as *const Task<F>) };
        }

        &RawWakerVTable::new(v_clone::<F>, v_wake::<F>, v_wake_by_ref::<F>, v_drop::<F>)
    };
}
