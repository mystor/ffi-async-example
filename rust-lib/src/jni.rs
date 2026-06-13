use crate::executor::{UniFFIExecutorVTable, uniffi_executor_set_vtable, uniffi_runnable_run};
use crate::uniffi_async_add;
use jni::errors::Result as JNIResult;
use jni::objects::{JByteBuffer, JClass, JObject, JValue};
use jni::refs::Global;
use jni::sys::{jboolean, jlong, jobject};
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

unsafe extern "C" fn complete(on_complete: *mut c_void, result: *mut c_void) {
    JavaVM::singleton()
        .unwrap()
        .attach_current_thread(|env| -> JNIResult<_> {
            let on_complete = unsafe { env.global_from_raw::<JObject>(on_complete as jobject) };
            let result_buffer = env
                .call_method(
                    &on_complete,
                    jni_str!("resultBuffer"),
                    jni_sig!("()Ljava/nio/ByteBuffer;"),
                    &[],
                )?
                .into_object()?;
            let result_buffer = unsafe { JByteBuffer::from_raw(env, result_buffer.into_raw()) };
            let result_ptr = env.get_direct_buffer_address(&result_buffer)?;
            let result_len = env.get_direct_buffer_capacity(&result_buffer)?;
            unsafe { std::ptr::copy_nonoverlapping(result.cast::<u8>(), result_ptr, result_len) };
            env.call_method(&on_complete, jni_str!("complete"), jni_sig!("()V"), &[])?;
            Ok(())
        })
        .expect("Calling complete failed");
}

static NATIVE_VTABLE: UniFFIExecutorVTable = UniFFIExecutorVTable { dispatch, complete };

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
    mut env: EnvUnowned<'_>,
    _class: JClass<'_>,
    left: jlong,
    right: jlong,
    on_complete: JObject<'_>,
) {
    env.with_env(|env| -> JNIResult<_> {
        let on_complete = env.new_global_ref(on_complete)?.into_raw() as *mut c_void;
        unsafe { uniffi_async_add(left as u64, right as u64, on_complete) };
        Ok(())
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_KotlinBinding_uniffiRunnableRun(
    _env: EnvUnowned<'_>,
    _class: JClass<'_>,
    runnable: jlong,
) {
    unsafe { uniffi_runnable_run(runnable as *mut c_void) };
}
