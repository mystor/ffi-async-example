use crate::executor::{
    UniFFIExecutorVTable, uniffi_executor_set_vtable, uniffi_runnable_run, uniffi_task_cancel,
    uniffi_task_poll,
};
use crate::uniffi_async_add;
use jni::errors::Result as JNIResult;
use jni::objects::{JByteBuffer, JClass, JObject, JValue};
use jni::refs::Global;
use jni::sys::{JNI_FALSE, JNI_TRUE, jboolean, jlong, jobject};
use jni::{EnvUnowned, JavaVM, jni_sig, jni_str};
use std::ffi::c_void;
use std::sync::OnceLock;

static JVM_VTABLE: OnceLock<Global<JObject<'static>>> = OnceLock::new();

unsafe extern "C" fn dispatch(runnable: *mut c_void, blocking: bool) {
    let runnable = runnable as jlong;
    JavaVM::singleton()
        .unwrap()
        .attach_current_thread(|env| -> JNIResult<_> {
            env.call_method(
                JVM_VTABLE.get().unwrap(),
                jni_str!("dispatch"),
                jni_sig!("(JZ)V"),
                &[JValue::Long(runnable), JValue::Bool(blocking as jboolean)],
            )?;
            Ok(())
        })
        .expect("Calling dispatch failed");
}

unsafe extern "C" fn waker_clone(waker: *mut c_void) -> *mut c_void {
    JavaVM::singleton()
        .unwrap()
        .attach_current_thread(|env| -> JNIResult<_> {
            let waker = unsafe { JObject::from_raw(env, waker as jobject) };
            Ok(env.new_global_ref(waker)?.into_raw() as *mut c_void)
        })
        .expect("waker_clone failed")
}

unsafe extern "C" fn waker_wake(waker: *mut c_void) {
    JavaVM::singleton()
        .unwrap()
        .attach_current_thread(|env| -> JNIResult<_> {
            let waker = unsafe { env.global_from_raw::<JObject>(waker as jobject) };
            env.call_method(waker, jni_str!("wake"), jni_sig!("()V"), &[])?;
            Ok(())
        })
        .expect("waker_wake failed");
}

unsafe extern "C" fn waker_wake_by_ref(waker: *mut c_void) {
    JavaVM::singleton()
        .unwrap()
        .attach_current_thread(|env| -> JNIResult<_> {
            let waker = unsafe { JObject::from_raw(env, waker as jobject) };
            env.call_method(waker, jni_str!("wake"), jni_sig!("()V"), &[])?;
            Ok(())
        })
        .expect("waker_wake_by_ref failed");
}

unsafe extern "C" fn waker_drop(waker: *mut c_void) {
    JavaVM::singleton()
        .unwrap()
        .attach_current_thread(|env| -> JNIResult<_> {
            let _ = unsafe { env.global_from_raw::<JObject>(waker as jobject) };
            Ok(())
        })
        .expect("waker_drop failed");
}

static NATIVE_VTABLE: UniFFIExecutorVTable = UniFFIExecutorVTable {
    dispatch,
    waker_clone,
    waker_wake,
    waker_wake_by_ref,
    waker_drop,
};

#[unsafe(no_mangle)]
pub extern "system" fn Java_KotlinBinding_uniffiExecutorSetVTable(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    executor_vtable: JObject<'_>,
) {
    env.with_env(|env| -> JNIResult<_> {
        let executor_vtable = env.new_global_ref(executor_vtable)?;
        JVM_VTABLE
            .set(executor_vtable)
            .expect("already initialized EXECUTOR_VTABLE");

        unsafe { uniffi_executor_set_vtable(&NATIVE_VTABLE) };
        Ok(())
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_KotlinBinding_uniffiAsyncAdd(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    left: jlong,
    right: jlong,
) -> jlong {
    unsafe { uniffi_async_add(left as u64, right as u64) as jlong }
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_KotlinBinding_uniffiRunnableRun(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    runnable: jlong,
) {
    unsafe { uniffi_runnable_run(runnable as *mut c_void) };
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_KotlinBinding_uniffiTaskCancel(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    task: jlong,
) {
    unsafe { uniffi_task_cancel(task as *mut c_void) };
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_KotlinBinding_uniffiTaskPoll(
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    task: jlong,
    waker: JObject<'_>,
    result: JByteBuffer<'_>,
) -> jboolean {
    env.with_env(|env| -> JNIResult<_> {
        let result_ptr = env.get_direct_buffer_address(&result)?;

        let done = unsafe {
            uniffi_task_poll(
                task as *mut c_void,
                waker.as_raw() as *mut c_void,
                result_ptr.cast::<c_void>(),
            )
        };

        Ok(if done { JNI_TRUE } else { JNI_FALSE })
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}
