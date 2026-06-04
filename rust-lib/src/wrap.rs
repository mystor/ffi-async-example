use crate::executor;

struct ExternResolver<R> {
    resolve: unsafe extern "C" fn(*mut libc::c_void, R),
    closure: *mut libc::c_void,
}

impl<R> ExternResolver<R> {
    fn resolve(self, value: R) {
        unsafe { (self.resolve)(self.closure, value) }
    }
}

// SAFETY: This is safe because the C++ promise pointer is owned by this
// resolver, and moved into the resolve function. The FFI contract requires this
// part to be threadsafe.
unsafe impl<R: Send + 'static> Send for ExternResolver<R> {}

// Wrap a future such that the given extern "C" function is called with the
// future's output when the future completes.
pub unsafe fn wrap_future<F>(
    future: F,
    resolve: unsafe extern "C" fn(*mut libc::c_void, F::Output),
    closure: *mut libc::c_void,
) where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    let resolver = ExternResolver { resolve, closure };
    executor::spawn(async move {
        resolver.resolve(future.await);
    })
    .detach();
}
