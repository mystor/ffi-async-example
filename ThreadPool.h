#pragma once

// NOTE: This thread pool implementation is just here because I needed a thread
// pool. There is no relevance to this implementation to the demo.

#include <chrono>
#include <condition_variable>
#include <functional>
#include <memory>
#include <mutex>
#include <queue>
#include <stddef.h>
#include <stdio.h>
#include <thread>
#include <utility>
#include <vector>

class ThreadPool {
public:
  explicit ThreadPool(const char *name, size_t thread_count) : name_(name) {
    for (size_t i = 0; i < thread_count; ++i) {
      workers_.emplace_back([this, i] { worker_loop(i); });
    }
  }

  ThreadPool(const ThreadPool &) = delete;
  ThreadPool &operator=(const ThreadPool &) = delete;

  ~ThreadPool() { shutdown(); }

  void post(std::function<void()> task) {
    printf("queueing a task on %s\n", name_);
    {
      std::lock_guard<std::mutex> lock(mutex_);
      tasks_.push(std::move(task));
    }
    condition_.notify_one();
  }

  void shutdown() {
    {
      std::lock_guard lock(mutex_);
      stopping_ = true;
    }
    condition_.notify_all();

    for (auto &worker : workers_) {
      if (worker.joinable()) {
        worker.join();
      }
    }
  }

private:
  void worker_loop(size_t i) {
    while (true) {
      std::function<void()> task;

      {
        std::unique_lock<std::mutex> lock(mutex_);
        condition_.wait(lock, [this] { return stopping_ || !tasks_.empty(); });

        if (stopping_ && tasks_.empty()) {
          break;
        }

        task = std::move(tasks_.front());
        tasks_.pop();
      }

      auto start = std::chrono::steady_clock::now();
      printf("[%s %zu] running a task\n", name_, i);
      task();

      auto elapsed_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
                            std::chrono::steady_clock::now() - start)
                            .count();
      printf("[%s %zu] finished a task, took %lld ms\n", name_, i,
             static_cast<long long>(elapsed_ms));
    }
  }

  const char *name_;
  std::vector<std::thread> workers_;
  std::queue<std::function<void()>> tasks_;
  std::mutex mutex_;
  std::condition_variable condition_;
  bool stopping_ = false;
};
