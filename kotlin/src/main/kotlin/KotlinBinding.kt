@file:JvmName("KotlinBinding")

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.DelicateCoroutinesApi
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.coroutines.resume
import kotlin.coroutines.suspendCoroutine

private external fun uniffiExecutorSetVTable(executorVTable: ExecutorVTable)
private external fun uniffiAsyncAdd(left: Long, right: Long, onComplete: CompleteCallback)
private external fun uniffiRunnableRun(runnable: Long)

class CompleteCallback(resultSize: Int, private val callback: (ByteBuffer) -> Unit) {
    private val result = ByteBuffer.allocateDirect(resultSize).order(ByteOrder.nativeOrder())

    fun resultBuffer(): ByteBuffer {
        return result
    }

    fun complete() {
        result.position(0)
        callback(result)
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
    return suspendCoroutine { continuation ->
        val callback = CompleteCallback(Long.SIZE_BYTES) { result ->
            continuation.resume(result.getLong(0))
        }
        uniffiAsyncAdd(left, right, callback)
    }
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
