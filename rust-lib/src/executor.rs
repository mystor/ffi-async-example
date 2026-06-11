use std::ffi::c_void;
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::OnceLock;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

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
    pub waker_clone: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    pub waker_wake: unsafe extern "C" fn(*mut c_void),
    pub waker_wake_by_ref: unsafe extern "C" fn(*mut c_void),
    pub waker_drop: unsafe extern "C" fn(*mut c_void),
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

unsafe fn waker_clone(waker: *const ()) -> RawWaker {
    RawWaker::new(
        unsafe { (executor_vtable().waker_clone)(waker as *mut c_void) } as *const (),
        &WAKER_VTABLE, // Cloned wakers are always owned
    )
}

unsafe fn waker_wake(waker: *const ()) {
    unsafe { (executor_vtable().waker_wake)(waker as *mut c_void) }
}

unsafe fn waker_wake_by_ref(waker: *const ()) {
    unsafe { (executor_vtable().waker_wake_by_ref)(waker as *mut c_void) }
}

unsafe fn waker_drop(waker: *const ()) {
    unsafe { (executor_vtable().waker_drop)(waker as *mut c_void) }
}

unsafe fn borrowed_waker_wake(_: *const ()) {
    unreachable!("The borrowed waker is not owned, so cannot be moved into a 'wake' call");
}

unsafe fn borrowed_waker_drop(_: *const ()) {
    /* the waker is borrowed, nothing to drop */
}

const WAKER_VTABLE: RawWakerVTable =
    RawWakerVTable::new(waker_clone, waker_wake, waker_wake_by_ref, waker_drop);

const BORROWED_WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(
    waker_clone,
    borrowed_waker_wake,
    waker_wake_by_ref,
    borrowed_waker_drop,
);

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uniffi_task_poll(
    task: *mut c_void,
    waker: *mut c_void,
    result: *mut c_void,
) -> bool {
    // FIXME: A better situation here would probably have `result` be in a known
    // format where we can communicate the panic out to the caller so the caller
    // can recover in this specific case. I'm punting on that.
    abort_on_panic(|| {
        // This waker is still owned by our caller, so capture it with a
        // BORROWED_WAKER_VTABLE so we don't double-drop it.
        let raw_waker = RawWaker::new(waker as *const (), &BORROWED_WAKER_VTABLE);
        let waker = unsafe { Waker::from_raw(raw_waker) };
        let mut context = Context::from_waker(&waker);
        // SAFETY: The vtable attribute is at the same offset due to #[repr(C)] for
        // all types of F.
        unsafe { ((*(task as *mut Task<()>)).vtable.poll)(task, &mut context, result) }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uniffi_task_cancel(task: *mut c_void) {
    // NOTE: Normally panicking in a destructor is fatal already IIRC, so this
    // probably is OK behaviour.
    abort_on_panic(|| {
        // SAFETY: The vtable attribute is at the same offset due to #[repr(C)] for
        // all types of F.
        unsafe { ((*(task as *mut Task<()>)).vtable.drop)(task) }
    })
}

struct TaskVTable {
    poll: unsafe fn(*mut c_void, context: &mut Context, result: *mut c_void) -> bool,
    drop: unsafe fn(*mut c_void),
}

#[repr(C)]
struct Task<F> {
    vtable: &'static TaskVTable,
    future: F,
}

impl<F> Task<F>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    unsafe fn task_poll(this: *mut c_void, context: &mut Context, result: *mut c_void) -> bool {
        let this = this as *mut Self;
        match unsafe { Pin::new_unchecked(&mut (*this).future) }.poll(context) {
            Poll::Ready(value) => unsafe {
                let _ = Box::from_raw(this);
                if !result.is_null() {
                    *(result as *mut F::Output) = value;
                }
                true
            },
            Poll::Pending => false,
        }
    }

    unsafe fn task_drop(this: *mut c_void) {
        let _ = unsafe { Box::from_raw(this as *mut Self) };
    }
}

pub fn into_raw_task<F>(future: F) -> *mut c_void
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    Box::into_raw(Box::new(Task {
        vtable: &TaskVTable {
            poll: Task::<F>::task_poll,
            drop: Task::<F>::task_drop,
        },
        future,
    })) as *mut c_void
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
