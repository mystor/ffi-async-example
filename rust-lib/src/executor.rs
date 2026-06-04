use std::ptr::NonNull;
use std::sync::Mutex;

// Arguments: (runnable: *mut libc::c_void, may_block: bool)
type DispatcherFunc = unsafe extern "C" fn(*mut libc::c_void, bool);

static DISPATCHER: Mutex<Option<DispatcherFunc>> = Mutex::new(None);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uniffi_runnable_run(runnable_ptr: *mut libc::c_void) {
    let Some(runnable_ptr) = NonNull::new(runnable_ptr as *mut ()) else {
        return;
    };
    let runnable = unsafe { async_task::Runnable::<()>::from_raw(runnable_ptr) };
    runnable.run();
}

// Set the global "dispatch" function callback.
// This should generally only be set once by the embedder early during startup.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uniffi_dispatcher_set(dispatch: Option<DispatcherFunc>) {
    *DISPATCHER.lock().unwrap() = dispatch;
}

// Internal helper to schedule a task with the dispatcher
fn schedule(runnable: async_task::Runnable<()>, may_block: bool) {
    let dispatcher = DISPATCHER
        .lock()
        .unwrap()
        .expect("Cannot schedule an async task without a dispatcher!");

    let closure = runnable.into_raw().as_ptr() as *mut libc::c_void;
    unsafe {
        dispatcher(closure, may_block);
    }
}

// See tokio's similar-in-vibes spawn method.
// Can be used to start parallel lines of async work.
pub fn spawn<F>(future: F) -> async_task::Task<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let (runnable, task) = async_task::spawn(future, move |runnable| {
        schedule(runnable, false);
    });
    runnable.schedule();
    task
}

// See tokio's similar-in-vibes spawn_blocking method.
// Can be used to execute a single blocking task on a background thread,
// avoiding blocking the main thread pool.
pub fn spawn_blocking<F, R>(f: F) -> async_task::Task<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let (runnable, task) = async_task::spawn(async move { f() }, move |runnable| {
        schedule(runnable, true);
    });
    runnable.schedule();
    task
}
