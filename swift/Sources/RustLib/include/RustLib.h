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

    // Handle completion of a task. The passed in pointer is the closure which
    // was passed to the async task.
    void (*complete)(void * /* on_complete */, void * /* result */);
} UniFFIExecutorVTable;

// Set the dispatcher functions which will be used for all uniffi async operations.
//
// Defined in executor.rs.
void uniffi_executor_set_vtable(const UniFFIExecutorVTable *executor_vtable);

// Run a runnable which was dispatched by the executor.
//
// Defined in executor.rs.
void uniffi_runnable_run(void *runnable);

// Defined in lib.rs.
void uniffi_async_add(uint64_t left, uint64_t right, void *on_complete);

#ifdef __cplusplus
}
#endif
