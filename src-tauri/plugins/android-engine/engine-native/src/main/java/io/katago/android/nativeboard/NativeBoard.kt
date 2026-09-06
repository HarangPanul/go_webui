package io.katago.android.nativeboard

/**
 * Thin JNI wrapper around KataGo's own game/board.cpp + game/rules.cpp +
 * game/boardhistory.cpp + neuralnet/nninputs.cpp (see engine-native/src/main/cpp).
 *
 * Reusing KataGo's C++ rules/feature-encoding code instead of reimplementing Go
 * rules in Kotlin avoids subtle bugs around pass-alive area, suicide, superko, etc.
 * (docs/decisions/0001-model-and-pipeline.md).
 *
 * Not thread-safe: each instance owns one native game state; confine use to a
 * single thread (or synchronize externally).
 */
class NativeBoard(boardSize: Int, rulesName: String = "chinese") : AutoCloseable {

    /** Number of input global features for model version 17 (tf3-b11c768/"t11b"). */
    companion object {
        const val NUM_FEATURES_SPATIAL_V7 = 22
        const val NUM_FEATURES_GLOBAL_V7 = 19

        init {
            System.loadLibrary("katago_native")
        }

        @JvmStatic private external fun nativeCreate(boardSize: Int, rulesName: String): Long
        @JvmStatic private external fun nativeDestroy(handle: Long)
        @JvmStatic private external fun nativePlay(handle: Long, x: Int, y: Int): Boolean
        @JvmStatic private external fun nativeIsLegal(handle: Long, x: Int, y: Int): Boolean
        @JvmStatic private external fun nativeGetNextPlayer(handle: Long): Int
        @JvmStatic private external fun nativeEncodeInputsV7(
            handle: Long,
            nnXLen: Int,
            nnYLen: Int,
            spatialOut: FloatArray,
            globalOut: FloatArray,
        )
    }

    private var handle: Long = nativeCreate(boardSize, rulesName)
    val size: Int = boardSize

    /** Plays a stone at (x, y), or passes if either coordinate is negative. Returns false if illegal. */
    fun play(x: Int, y: Int): Boolean {
        check(handle != 0L) { "NativeBoard already closed" }
        return nativePlay(handle, x, y)
    }

    fun pass(): Boolean = play(-1, -1)

    /** Non-mutating legality check; pass (x<0 or y<0) is legal except in a few edge-case rulesets. */
    fun isLegal(x: Int, y: Int): Boolean {
        check(handle != 0L) { "NativeBoard already closed" }
        return nativeIsLegal(handle, x, y)
    }

    /** 1 = black, 2 = white (Board.P_BLACK / Board.P_WHITE in KataGo's C++). */
    val nextPlayer: Int
        get() {
            check(handle != 0L) { "NativeBoard already closed" }
            return nativeGetNextPlayer(handle)
        }

    /**
     * Encodes the current position exactly as KataGo's engine would for a model-version-17
     * (transformer) net: spatial is [22, nnYLen, nnXLen] in NCHW layout, global is [19].
     */
    fun encodeInputsV7(nnXLen: Int, nnYLen: Int): Pair<FloatArray, FloatArray> {
        check(handle != 0L) { "NativeBoard already closed" }
        val spatial = FloatArray(NUM_FEATURES_SPATIAL_V7 * nnYLen * nnXLen)
        val global = FloatArray(NUM_FEATURES_GLOBAL_V7)
        nativeEncodeInputsV7(handle, nnXLen, nnYLen, spatial, global)
        return spatial to global
    }

    override fun close() {
        if (handle != 0L) {
            nativeDestroy(handle)
            handle = 0L
        }
    }
}
