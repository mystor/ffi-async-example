@file:JvmName("KotlinBinding")

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.DelicateCoroutinesApi
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.suspendCancellableCoroutine
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.coroutines.Continuation
import kotlin.coroutines.resume

private external fun uniffiExecutorSetVTable(executorVTable: ExecutorVTable)
private external fun uniffiAsyncAdd(left: Long, right: Long): Long
private external fun uniffiRunnableRun(runnable: Long)
private external fun uniffiTaskCancel(task: Long)
private external fun uniffiTaskPoll(task: Long, waker: RustWaker, result: ByteBuffer): Boolean

class RustWaker {
    private var continuation: Continuation<Unit>? = null
    private var woken = false

    fun wake() {
        val continuation = synchronized(this) {
            woken = true
            val stored = this.continuation
            this.continuation = null
            stored
        }
        try {
            continuation?.resume(Unit)
        } catch (_: IllegalStateException) {
            // Ignore exceptions if the continuation has been cancelled
        }
    }

    suspend fun waitPoll(poll: () -> Boolean): Boolean {
        synchronized(this) {
            woken = false
        }

        if (poll()) {
            return true
        }

        suspendCancellableCoroutine { continuation ->
            val ready = synchronized(this) {
                this.continuation = continuation
                woken
            }
            if (ready) {
                wake()
            }
        }

        return false
    }
}

suspend fun pollTask(task: Long, result: ByteBuffer) {
    val waker = RustWaker()
    var complete = false

    try {
        while (true) {
            complete = waker.waitPoll { uniffiTaskPoll(task, waker, result) }
            if (complete) {
                return
            }
        }
    } finally {
        if (!complete) {
            uniffiTaskCancel(task)
        }
    }
}

object ExecutorVTable {
    @OptIn(DelicateCoroutinesApi::class)
    fun dispatch(runnable: Long, blocking: Boolean) {
        val dispatcher = if (blocking) Dispatchers.IO else Dispatchers.Default
        GlobalScope.launch(dispatcher) {
            uniffiRunnableRun(runnable)
        }
    }
}

suspend fun asyncAdd(left: Long, right: Long): Long {
    val task = uniffiAsyncAdd(left, right)
    val result = ByteBuffer.allocateDirect(Long.SIZE_BYTES).order(ByteOrder.nativeOrder())

    pollTask(task, result)
    return result.getLong(0)
}

fun main() = runBlocking {
    System.loadLibrary("rust_lib")
    uniffiExecutorSetVTable(ExecutorVTable)

    val start = System.nanoTime()
    val result = asyncAdd(5, 10)
    val elapsedMs = (System.nanoTime() - start) / 1_000_000.0

    println("result: $result")
    println("async_add took: %.0f ms".format(elapsedMs))
}
