import Dispatch
import Foundation
import RustLib
import Synchronization

private final class RustWaker: @unchecked Sendable {
  private let state: Mutex<CheckedContinuation<Void, Never>?> = Mutex(nil)

  func wake() {
    let continuation = state.withLock { state in state.take() }
    if let continuation {
      continuation.resume()
    }
  }

  func wait_poll(_ poll: (UnsafeMutableRawPointer) -> Bool) async -> Bool {
    var ready = false
    await withCheckedContinuation { continuation in
      state.withLock { state in state = continuation }
      ready = poll(Unmanaged.passUnretained(self).toOpaque())
      if ready {
        wake()
      }
    }
    return ready
  }
}

private let executorVTable = UniFFIExecutorVTable(
  dispatch: { runnable, blocking in
    let runnableAddr = UInt(bitPattern: runnable!)

    if blocking {
      DispatchQueue.global(qos: .utility).async {
        uniffi_runnable_run(UnsafeMutableRawPointer(bitPattern: runnableAddr)!)
      }
    } else {
      Task.detached {
        uniffi_runnable_run(UnsafeMutableRawPointer(bitPattern: runnableAddr)!)
      }
    }
  },
  waker_clone: { waker in
    _ = Unmanaged<RustWaker>.fromOpaque(waker!).retain()
    return waker
  },
  waker_wake: { waker in
    Unmanaged<RustWaker>.fromOpaque(waker!).takeRetainedValue().wake()
  },
  waker_wake_by_ref: { waker in
    Unmanaged<RustWaker>.fromOpaque(waker!).takeUnretainedValue().wake()
  },
  waker_drop: { waker in
    Unmanaged<RustWaker>.fromOpaque(waker!).release()
  }
)

private func pollTask<Result>(task: UnsafeMutableRawPointer, result: inout Result) async throws {
  let waker = RustWaker()

  try await withTaskCancellationHandler {
    while !Task.isCancelled {
      let done = await waker.wait_poll { wakerPtr in
        withUnsafeMutablePointer(to: &result) { resultPointer in
          uniffi_task_poll(task, wakerPtr, resultPointer)
        }
      }
      if done {
        return
      }
    }

    uniffi_task_cancel(task)
    throw CancellationError()
  } onCancel: {
    waker.wake()
  }
}

private func asyncAdd(_ left: UInt64, _ right: UInt64) async throws -> UInt64 {
  let task = uniffi_async_add(left, right)!
  var result: UInt64 = 0

  try await pollTask(task: task, result: &result)
  return result
}

@main
struct swift {
  static func main() async throws {
    withUnsafePointer(to: executorVTable) { vtablePointer in
      uniffi_executor_set_vtable(vtablePointer)
    }

    let start = DispatchTime.now()
    let result = try await asyncAdd(5, 10)
    let elapsed = DispatchTime.now().uptimeNanoseconds - start.uptimeNanoseconds
    let elapsedMs = Double(elapsed) / 1_000_000.0

    print("result: \(result)")
    print(String(format: "async_add took: %.0f ms", elapsedMs))
  }
}
