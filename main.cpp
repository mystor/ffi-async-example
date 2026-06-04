#include "RustLib.h"
#include "ThreadPool.h"

#include <chrono>
#include <future>
#include <inttypes.h>
#include <stdio.h>

ThreadPool fg_pool("fg_pool", 4);
ThreadPool bg_pool("bg_pool", 4);

// Example implementation of the dispatch callback.
void dispatch_impl(void *runnable, bool may_block) {
  ThreadPool &pool = may_block ? bg_pool : fg_pool;
  pool.post([runnable] { uniffi_runnable_run(runnable); });
}

// This is mostly demonstrating the async executor parts in Rust - so just use a
// blocking `std::future` on the C++ side. We'll block the main thread and the
// async work will be happening on the pool so this is OK.
std::future<uint64_t> async_add(uint64_t left, uint64_t right) {
  auto promise = std::make_unique<std::promise<uint64_t>>();
  auto future = promise->get_future();
  uniffi_async_add(
      left, right,
      [](void *raw_promise, uint64_t value) {
        std::unique_ptr<std::promise<uint64_t>> promise(
            static_cast<std::promise<uint64_t> *>(raw_promise));
        promise->set_value(value);
      },
      promise.release());
  return future;
}

int main() {
  uniffi_dispatcher_set(&dispatch_impl);

  auto start = std::chrono::steady_clock::now();

  // Block until the future is resolved.
  uint64_t result = async_add(5, 10).get();

  auto elapsed_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
                        std::chrono::steady_clock::now() - start)
                        .count();

  printf("result: %" PRIu64 "\n", result);
  printf("async_add took: %lld ms\n", static_cast<long long>(elapsed_ms));
}
