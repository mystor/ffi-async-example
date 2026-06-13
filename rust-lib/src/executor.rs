use std::ffi::c_void;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::OnceLock;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;

#[inline]
fn abort_on_panic<T>(f: impl FnOnce() -> T) -> T {
    struct Bomb;

    impl Drop for Bomb {
        fn drop(&mut self) {
            std::process::abort();
        }
    }

    let bomb = Bomb;
    let t = f();
    std::mem::forget(bomb);
    t
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct UniFFIExecutorVTable {
    pub dispatch: unsafe extern "C" fn(*mut c_void, bool),
    pub complete: unsafe extern "C" fn(*mut c_void, *mut c_void),
}

// NOTE: This is a static copy of the table, rather than holding the pointer to
// the provided table, as the caller may not be able to reliably statically
// allocate this table (e.g. swift appears to have no mechanism for obtaining a
// static immutable reference to a vtable?)
static EXECUTOR: OnceLock<UniFFIExecutorVTable> = OnceLock::new();

fn executor_vtable() -> &'static UniFFIExecutorVTable {
    EXECUTOR.get().expect("executor vtable was not set")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uniffi_executor_set_vtable(executor_vtable: *const UniFFIExecutorVTable) {
    abort_on_panic(|| {
        if EXECUTOR.set(unsafe { *executor_vtable }).is_err() {
            panic!("executor vtable was set multiple times");
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uniffi_runnable_run(runnable_ptr: *mut c_void) {
    // NOTE: I am pretty sure `async_task::Runnable` already handles panics
    // internally, so I don't think this is ever likely to panic. This is
    // defensive.
    abort_on_panic(|| {
        let runnable = unsafe {
            async_task::Runnable::<()>::from_raw(NonNull::new_unchecked(runnable_ptr as *mut ()))
        };
        runnable.run();
    });
}

fn dispatch(runnable: async_task::Runnable<()>, blocking: bool) {
    unsafe { (executor_vtable().dispatch)(runnable.into_raw().as_ptr() as *mut c_void, blocking) }
}

// See tokio's similar-in-vibes spawn method.
// Can be used to start parallel lines of async work.
pub fn spawn<F>(future: F) -> async_task::Task<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (runnable, handle) = async_task::spawn(future, |runnable| dispatch(runnable, false));
    dispatch(runnable, false);
    handle
}

// See tokio's similar-in-vibes spawn_blocking method.
// Can be used to execute a single blocking task on a background thread,
// avoiding blocking the main thread pool.
pub fn spawn_blocking<F, R>(f: F) -> async_task::Task<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let (runnable, handle) =
        async_task::spawn(async move { f() }, |runnable| dispatch(runnable, true));
    dispatch(runnable, true);
    handle
}

pin_project! {
    struct WrapTask<F> {
        #[pin]
        future: F,
        on_complete: *mut c_void,
    }
}

unsafe impl<F> Send for WrapTask<F> where F: Send {}

impl<F> Future for WrapTask<F>
where
    F: Future,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.future.poll(cx).map(|mut result| unsafe {
            (executor_vtable().complete)(*this.on_complete, &mut result as *mut _ as *mut c_void)
        })
    }
}

pub unsafe fn wrap_task<F>(future: F, on_complete: *mut c_void)
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    spawn(WrapTask {
        future,
        on_complete,
    })
    .detach()
}
