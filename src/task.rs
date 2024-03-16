#[cfg(not(test))]
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::{
    future::Future,
    mem::ManuallyDrop,
    pin::Pin,
    task::{Context, RawWaker, RawWakerVTable, Waker},
};

use parking_lot::Mutex;

pub(crate) struct Task {
    data: Mutex<Option<Pin<Box<dyn Future<Output = ()> + Send>>>>,
}

impl Task {
    #[inline(always)]
    pub fn new(task: Pin<Box<dyn Future<Output = ()> + Send>>) -> Self {
        Self {
            data: Mutex::new(Some(task)),
        }
    }

    #[cfg_attr(feature = "inlining", inline(always))]
    pub fn dispatch(self: &Arc<Self>) {
        unsafe { Self::arc_dispatch(self.as_ref()) }
    }

    #[inline(always)]
    unsafe fn arc_dispatch(this: *const Self) {
        let mut data = (*this).data.lock();
        let Some(task) = data.as_mut() else {
            return;
        };
        let waker = ManuallyDrop::new(Waker::from_raw(RawWaker::new(this as _, TASK_V_TABLE)));
        let mut cx = Context::from_waker(&waker);
        if task.as_mut().poll(&mut cx).is_ready() {
            *data = None;
        }
    }
}

const TASK_V_TABLE: &RawWakerVTable = {
    #[inline(always)]
    unsafe fn v_clone(this: *const ()) -> RawWaker {
        Arc::increment_strong_count(this as *const Task);
        RawWaker::new(this, TASK_V_TABLE)
    }

    #[inline(always)]
    unsafe fn v_wake(this: *const ()) {
        v_wake_by_ref(this);
        v_drop(this);
    }

    #[inline(always)]
    unsafe fn v_wake_by_ref(this: *const ()) {
        Task::arc_dispatch(this as _);
    }

    #[inline(always)]
    unsafe fn v_drop(this: *const ()) {
        Arc::decrement_strong_count(this as *const Task);
    }

    &RawWakerVTable::new(v_clone, v_wake, v_wake_by_ref, v_drop)
};
