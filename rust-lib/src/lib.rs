use std::time::Duration;

mod executor;
mod wrap;

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uniffi_async_add(
    left: u64,
    right: u64,
    resolve: unsafe extern "C" fn(*mut libc::c_void, u64),
    closure: *mut libc::c_void,
) {
    let future = async_add(left, right);
    unsafe {
        wrap::wrap_future(future, resolve, closure);
    }
}
