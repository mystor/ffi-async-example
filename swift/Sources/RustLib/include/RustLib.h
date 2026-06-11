#pragma once

#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

// These are the symbols exported by rust-lib.

typedef struct UniFFIExecutorVTable {
    // Dispatch a spawned runnable. If `blocking` is true, it should run on a
    // thread that can safely block.
    void (*dispatch)(void * /* runnable */, bool /* blocking */);

    // Clone the waker. Usually this is done by adding a new reference, and
    // returning the same waker pointer.
    void *(*waker_clone)(void * /* waker */);

    // Wake the waker, consuming ownership (waker_drop will not be called).
    void (*waker_wake)(void * /* waker */);

    // Wake the waker, without ownership (waker_drop will still be called).
    void (*waker_wake_by_ref)(void * /* waker */);

    // Drop a reference to the waker object.
    void (*waker_drop)(void * /* waker */);
} UniFFIExecutorVTable;

// Set the dispatcher functions which will be used for all uniffi async operations.
//
// Defined in executor.rs.
void uniffi_executor_set_vtable(const UniFFIExecutorVTable *executor_vtable);

// Run a runnable which was dispatched by the executor.
//
// Defined in executor.rs.
void uniffi_runnable_run(void *runnable);

// Poll a task.
//
// Returns `true` if the task is ready, and populates `result` with the result
// value (dependent on the type of task).
//
// Otherwise, returns `false`. The passed-in `waker` will be cloned using
// `waker_clone`, and later woken using `waker_wake` / dropped with
// `waker_drop`.
//
// Defined in executor.rs.
bool uniffi_task_poll(void *task, void *waker, void *result);

// Cancel a task. This must be done on the task polling thread instead of
// calling `uniffi_task_poll` when the task should be cancelled.
//
// Defined in executor.rs.
void uniffi_task_cancel(void *task);

// Defined in lib.rs.
// Returns a task which should be polled with uniffi_task_poll.
void *uniffi_async_add(uint64_t left, uint64_t right);

#ifdef __cplusplus
}
#endif
