package io.katago.android.engine

/**
 * Placeholder for a LiteRT-backed [KataGoNet].
 *
 * Not implemented: docs/decisions/0002-litert-vs-executorch.md found that the current
 * `litert-torch` (successor to `ai-edge-torch`) converter fails on the full 11-block
 * tf3-b11c768 model with a `tfl.batch_matmul` rank-mismatch error (reproduces at >=4
 * stacked transformer blocks; individual blocks/heads convert fine in isolation, so this
 * looks like a converter bug rather than something wrong with the model). Revisit once
 * that's fixed upstream or root-caused further.
 */
class LiteRtEngine(
    @Suppress("UNUSED_PARAMETER") tflitePath: String,
    override val nnXLen: Int,
    override val nnYLen: Int,
) : KataGoNet {

    override fun evaluate(spatial: FloatArray, global: FloatArray): KataGoOutput {
        throw NotImplementedError(
            "LiteRT backend is blocked on a litert-torch converter issue -- see " +
                "docs/decisions/0002-litert-vs-executorch.md. Use ExecuTorchEngine instead."
        )
    }

    override fun close() = Unit
}
