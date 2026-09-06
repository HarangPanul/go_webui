package io.katago.android.engine

/**
 * Raw (pre-postprocessing) neural net outputs for one position, matching the three
 * tensors [InferenceWrapper] in tools/model_export/pytorch_to_executorch/export_spike.py
 * exposes from the underlying model_pytorch.Model:
 *
 *  - [policyLogits]: size numPolicyOutputs * (nnXLen*nnYLen + 1); last column per output is
 *    the pass logit (see PolicyHead.forward in katago/python/.../model_pytorch.py, which
 *    concatenates the board policy and the pass logit before returning).
 *  - [valueLogits]: size 3, {win, loss, noResult} pre-softmax.
 *  - [ownershipPretanh]: size nnXLen*nnYLen, pre-tanh.
 *
 * None of softmax/tanh has been applied -- callers (e.g. [io.katago.android.engine.Mcts])
 * are responsible for that, same division of labor as KataGo's own cpp/neuralnet/nneval.cpp.
 */
data class KataGoOutput(
    val policyLogits: FloatArray,
    val valueLogits: FloatArray,
    val ownershipPretanh: FloatArray,
)

/**
 * A loaded KataGo neural net backend. Implementations own whatever runtime
 * (ExecuTorch, and eventually LiteRT once docs/decisions/0002's blocker clears) is needed
 * to run the converted tf3-b11c768 ("t11b") model.
 *
 * Not guaranteed thread-safe; confine each instance to one thread (e.g. one per MCTS worker)
 * unless a specific implementation documents otherwise.
 */
interface KataGoNet : AutoCloseable {
    /** Board size this instance was built for (fixed at construction; matches nnXLen/nnYLen). */
    val nnXLen: Int
    val nnYLen: Int

    /**
     * Runs one forward pass.
     *
     * @param spatial [io.katago.android.nativeboard.NativeBoard.encodeInputsV7]'s first output:
     *   NCHW, length 22*nnYLen*nnXLen.
     * @param global [io.katago.android.nativeboard.NativeBoard.encodeInputsV7]'s second output:
     *   length 19.
     */
    fun evaluate(spatial: FloatArray, global: FloatArray): KataGoOutput
}
