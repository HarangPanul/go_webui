package io.katago.android.engine

import org.pytorch.executorch.EValue
import org.pytorch.executorch.Module
import org.pytorch.executorch.Tensor

/**
 * [KataGoNet] backed by ExecuTorch (org.pytorch:executorch-android), running the `.pte`
 * produced by tools/model_export/pytorch_to_executorch/export_spike.py.
 *
 * See docs/decisions/0002-litert-vs-executorch.md: ExecuTorch is currently the only backend
 * that successfully converts the full 11-block tf3-b11c768 model, so this is the sole
 * [KataGoNet] implementation for now.
 */
class ExecuTorchEngine(
    ptePath: String,
    override val nnXLen: Int,
    override val nnYLen: Int,
    private val forwardMethod: String = "forward",
) : KataGoNet {

    private companion object {
        const val NUM_FEATURES_SPATIAL_V7 = 22L
        const val NUM_FEATURES_GLOBAL_V7 = 19L
    }

    private val module: Module = Module.load(ptePath)

    override fun evaluate(spatial: FloatArray, global: FloatArray): KataGoOutput {
        require(spatial.size == (NUM_FEATURES_SPATIAL_V7 * nnYLen * nnXLen).toInt()) {
            "spatial size ${spatial.size} != 22*$nnYLen*$nnXLen"
        }
        require(global.size == NUM_FEATURES_GLOBAL_V7.toInt()) {
            "global size ${global.size} != 19"
        }

        val spatialTensor = Tensor.fromBlob(spatial, longArrayOf(1, NUM_FEATURES_SPATIAL_V7, nnYLen.toLong(), nnXLen.toLong()))
        val globalTensor = Tensor.fromBlob(global, longArrayOf(1, NUM_FEATURES_GLOBAL_V7))

        // Matches InferenceWrapper.forward(input_spatial, input_global) in
        // tools/model_export/pytorch_to_executorch/export_spike.py: returns
        // (policy_logits, value_logits, ownership_pretanh) in that order.
        val outputs = module.execute(forwardMethod, EValue.from(spatialTensor), EValue.from(globalTensor))
        check(outputs.size == 3) { "expected 3 outputs from ExecuTorch forward, got ${outputs.size}" }

        return KataGoOutput(
            policyLogits = outputs[0].toTensor().dataAsFloatArray,
            valueLogits = outputs[1].toTensor().dataAsFloatArray,
            ownershipPretanh = outputs[2].toTensor().dataAsFloatArray,
        )
    }

    override fun close() {
        module.destroy()
    }
}
