#pragma once

#include <stdint.h>
#include <stdlib.h>

// These are the symbols exported by rust-lib

extern "C" {
// Defined in executor.rs
void uniffi_runnable_run(void *runnable_ptr);

// Defined in executor.rs
void uniffi_dispatcher_set(void (*dispatch)(void *, bool));

// Defined in lib.rs
void uniffi_async_add(uint64_t left, uint64_t right,
                      void (*resolve)(void *, uint64_t), void *closure);
}
