import Dispatch
import Foundation
import RustLib
import Synchronization

final class CompleteCallback {
  let callback: (UnsafeMutableRawPointer?) -> Void

  init(_ callback: @escaping (UnsafeMutableRawPointer?) -> Void) {
    self.callback = callback
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
  complete: { closure, result in
    Unmanaged<CompleteCallback>.fromOpaque(closure!).takeRetainedValue().callback(result)
  }
)

private func asyncAdd(_ left: UInt64, _ right: UInt64) async -> UInt64 {
  await withCheckedContinuation { continuation in
    let callback = CompleteCallback { result in
      continuation.resume(returning: result!.load(fromByteOffset: 0, as: UInt64.self))
    }
    uniffi_async_add(left, right, Unmanaged.passRetained(callback).toOpaque());
  }
}

@main
struct swift {
  static func main() async throws {
    withUnsafePointer(to: executorVTable) { vtablePointer in
      uniffi_executor_set_vtable(vtablePointer)
    }

    let start = DispatchTime.now()
    let result = await asyncAdd(5, 10)
    let elapsed = DispatchTime.now().uptimeNanoseconds - start.uptimeNanoseconds
    let elapsedMs = Double(elapsed) / 1_000_000.0

    print("result: \(result)")
    print(String(format: "async_add took: %.0f ms", elapsedMs))
  }
}
