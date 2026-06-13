use std::ffi::c_void;
use std::time::Duration;

mod executor;
mod jni;

// An extremely legitimate function which needs to do some expensive sleeping
// off of the working thread.
async fn async_add(left: u64, right: u64) -> u64 {
    let in_parallel = executor::spawn(async move {
        executor::spawn_blocking(|| std::thread::sleep(Duration::from_secs(1))).await;
        left
    });
    executor::spawn_blocking(|| std::thread::sleep(Duration::from_secs(1))).await;
    in_parallel.await + right
}

// The wrapper to turn it into a UniFFI function - Actual UniFFI would need to
// do more here for error handling, marshalling & unmarshalling, etc.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uniffi_async_add(left: u64, right: u64, on_complete: *mut c_void) {
    let future = async_add(left, right);
    unsafe { executor::wrap_task(future, on_complete) }
}
