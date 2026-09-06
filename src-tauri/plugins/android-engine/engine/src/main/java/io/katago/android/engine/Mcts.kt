package io.katago.android.engine

import io.katago.android.nativeboard.NativeBoard
import kotlin.math.exp
import kotlin.math.ln
import kotlin.math.max
import kotlin.math.sqrt
import kotlin.math.tanh

/** A move on the board, or [PASS]. */
data class MctsMove(val x: Int, val y: Int) {
    val isPass: Boolean get() = x < 0 || y < 0

    companion object {
        val PASS = MctsMove(-1, -1)
    }
}

private fun softmax(logits: FloatArray): FloatArray {
    val max = logits.max()
    val exps = FloatArray(logits.size) { exp((logits[it] - max).toDouble()).toFloat() }
    val sum = exps.sum()
    return FloatArray(logits.size) { exps[it] / sum }
}

/**
 * Single-threaded PUCT search over policy prior + value backup. Ports four of KataGo's real
 * search formulas from the vendored katago/cpp/search sources (see go_webui conversation that
 * decided this scope) on top of the original minimal spike:
 *  - log-scaling cPUCT ([cpuctForVisits], from searchexplorehelpers.cpp::cpuctExploration)
 *  - FPU, first play urgency, for unvisited children ([fpuValue])
 *  - a score-aware utility term derived from the ownership head ([winLossValue]/[scoreUtilityValue])
 *  - LCB (lower confidence bound) final move selection instead of plain most-visited
 *    ([selectFinalMove])
 *
 * Still deliberately *not* a full port of KataGo's actual search.cpp: no multithreading/
 * virtual loss, no transposition table, no time control, no root noise, no subtree reuse
 * across genmove calls. KataGo's real search is an ~12k line subsystem built around batched
 * GPU inference across many threads; on this single-threaded mobile CPU backend that
 * machinery wouldn't parallelize anything (every thread would just queue behind the same
 * serial NN call), so it wasn't worth porting.
 *
 * Each node stores the move sequence from the root rather than a live board handle, and
 * replays it onto a fresh [NativeBoard] whenever a leaf needs evaluating. That's O(depth)
 * per simulation instead of O(1), which is fine for the visit counts a mobile device can
 * afford in this minimal search but would need a proper copy/undo API on NativeBoard to
 * scale further.
 *
 * Not thread-safe.
 *
 * go_webui local patch: added [rootHistory] so search can start from an arbitrary
 * mid-game position instead of always the empty board - upstream android_kata only ever
 * benchmarked/tested from an empty board (docs/benchmarks.md), so this constructor
 * parameter didn't exist there. [evaluatePath] simply replays [rootHistory] before this
 * search's own in-tree [path], keeping the existing "replay from scratch per leaf"
 * architecture unchanged.
 */
class Mcts(
    private val net: KataGoNet,
    private val boardSize: Int,
    private val rulesName: String = "chinese",
    private val rootHistory: List<MctsMove> = emptyList(),
    // 아래 상수들은 katago/cpp/search/searchparams.cpp의 기본("rules" 프리셋 근처) 값을
    // 그대로 옮긴 것 - 정확한 출처는 각 사용처 주석 참고.
    private val cpuctExploration: Double = 1.0,
    private val cpuctExplorationLog: Double = 0.45,
    private val cpuctExplorationBase: Double = 500.0,
    private val fpuReductionMax: Double = 0.2,
    private val rootFpuReductionMax: Double = 0.1,
    private val winLossUtilityFactor: Double = 1.0,
    // KataGo 원본의 staticScoreUtilityFactor(0.1) + dynamicScoreUtilityFactor(0.3) 합.
    // 실제 scoreMean/scoreStdev 헤드가 없어 ownership에서 근사하므로(valueFromOutput 참고)
    // 두 항을 나눠봐야 의미가 없어 하나로 합쳤다.
    private val scoreUtilityFactor: Double = 0.4,
    private val lcbStdevs: Double = 5.0,
    private val minVisitPropForLcb: Double = 0.15,
) {
    private class Node(val prior: Float) {
        var visitCount: Int = 0
        var valueSum: Double = 0.0
        var valueSqSum: Double = 0.0
        // Backed up in parallel with valueSum but tracks *only* the win/loss term, never the
        // blended score utility - see [rootCandidates]'s kdoc for why this needs to be kept
        // separate from valueSum/meanValue (which is deliberately unbounded past [-1, 1] and
        // is only ever used for search/selection, never reported as a "winrate").
        var winLossSum: Double = 0.0
        val children = LinkedHashMap<MctsMove, Node>()
        var expanded = false

        val meanValue: Double
            get() = if (visitCount == 0) 0.0 else valueSum / visitCount

        val meanWinLoss: Double
            get() = if (visitCount == 0) 0.0 else winLossSum / visitCount
    }

    private val root = Node(prior = 1f)

    // 루트 위치 자체를 신경망이 한 번 평가한 원본(pre-tanh) 출력 - 루트가 처음
    // expand될 때 한 번만 채워지고 이후 search()를 몇 번을 더 불러도 안 바뀐다(그
    // 시점의 신경망 평가 자체는 트리가 자라도 달라지지 않으므로). kata-analyze
    // 스트리밍(GtpShell.kt)이 [rootOwnership]으로 매 보고 주기마다 재사용 - 매번
    // 새로 신경망을 돌리면 보고 한 번에 시뮬레이션 하나만큼의 비용이 더 들어간다.
    private var rootOutput: KataGoOutput? = null

    /** Runs [numSimulations] simulations from the current root and returns the chosen move
     * (LCB winner among sufficiently-visited children - see [selectFinalMove]). */
    fun search(numSimulations: Int): MctsMove {
        require(numSimulations > 0)
        repeat(numSimulations) { runSimulation() }
        return selectFinalMove()
    }

    /** 지금까지 실제로 방문된(visitCount > 0) 루트의 자식 수들, 방문 수 내림차순.
     * kata-analyze 스트리밍이 "info move ..." 후보 목록으로 그대로 보고한다. */
    data class RootCandidate(val move: MctsMove, val visits: Int, val winrateForRootMover: Double)

    fun rootCandidates(): List<RootCandidate> {
        return root.children.entries
            .filter { it.value.visitCount > 0 }
            .sortedByDescending { it.value.visitCount }
            .map { (move, node) ->
                // node.meanWinLoss(순수 승/패 항)에서 계산한다 - node.meanValue는 FPU/
                // exploreSelectionValue/selectFinalMove의 LCB에 쓰는 "탐색용 블렌디드
                // 유틸리티"라 winLossUtilityFactor(1.0) + scoreUtilityFactor(0.4)가 겹치는
                // 극단적으로 확실한 국면에서는 (-1.4, 1.4) 범위까지 벗어날 수 있다 -
                // 그걸 그대로 아래 공식에 넣으면 winrate가 0~1을 벗어나(예: 1.2 = 120%)
                // WinrateGraph의 막대 비율이 깨진다(발견 경위: go_webui 세션 코드 리뷰).
                // node.meanValue와 마찬가지로 backup()이 매 ply마다 부호를 뒤집으므로
                // "그 수를 둔 다음 상대가 둘 차례" 관점 - 루트에서 지금 둘 차례인 쪽
                // 기준으로 보려면 한 번 더 뒤집어야 한다(genmoveReal()이 고르는 최종
                // 수와 같은 기준으로 맞추기 위함).
                RootCandidate(move, node.visitCount, (-node.meanWinLoss + 1.0) / 2.0)
            }
    }

    /** 루트 위치의 원본 ownership(신경망이 그 자리에서 낸 값 그대로, pre-tanh) - 아직
     * 한 번도 expand되지 않았으면(=아직 시뮬레이션이 한 번도 안 끝났으면) null.
     * 진짜 KataGo의 kata-analyze는 탐색 트리 전체의 방문 가중 평균을 내지만, 이
     * 최소 구현은 그 정교함 없이 "지금 위치를 신경망이 어떻게 보는가"만 근사로
     * 보여준다(트리가 자라도 이 값 자체는 안 바뀜 - 위 [rootOutput] 참고). */
    fun rootOwnership(): FloatArray? = rootOutput?.ownershipPretanh

    private fun runSimulation() {
        val path = mutableListOf<MctsMove>()
        val nodePath = mutableListOf(root)
        var node = root
        var depth = 0

        // Select down to a leaf using PUCT (+ FPU for never-visited children).
        while (node.expanded && node.children.isNotEmpty()) {
            val fpu = fpuValue(node, isRoot = depth == 0)
            val parentVisits = node.visitCount
            val (move, child) = node.children.maxByOrNull { (_, c) ->
                exploreSelectionValue(c, parentVisits, fpu)
            }!!
            path += move
            nodePath += child
            node = child
            depth++
        }

        val (output, board) = evaluatePath(path)
        board.use {
            val winLoss = winLossValue(output)
            val value = winLoss * winLossUtilityFactor + scoreUtilityValue(output) * scoreUtilityFactor
            if (!node.expanded) {
                expand(node, output, board)
            }
            backup(nodePath, value, winLoss)
        }
    }

    /** KataGo's log-scaling cPUCT (searchexplorehelpers.cpp::cpuctExploration): grows with
     * visit count instead of staying a flat constant, so exploration keeps mattering deep
     * into a long search instead of being swamped by accumulated exploitation values. */
    private fun cpuctForVisits(totalVisits: Int): Double {
        return cpuctExploration +
            cpuctExplorationLog * ln((totalVisits + cpuctExplorationBase) / cpuctExplorationBase)
    }

    private fun exploreSelectionValue(child: Node, parentVisits: Int, fpu: Double): Double {
        val utility = if (child.visitCount == 0) fpu else child.meanValue
        val explore = cpuctForVisits(parentVisits) * child.prior * sqrt(parentVisits.toDouble()) / (1 + child.visitCount)
        return utility + explore
    }

    /** First play urgency: KataGo evaluates never-visited children pessimistically (the
     * parent's own utility, nudged down) rather than as a neutral 0 - a child that merely
     * looks "as good as this position already is" isn't reason enough to try it over a
     * sibling that's already confirmed better. See searchexplorehelpers.cpp::
     * getFpuValueForChildrenAssumeVisited.
     *
     * We're always fully expanded (every legal move gets a child node up front in [expand]),
     * so "visited policy mass" is approximated as the prior mass of children that have
     * actually been *searched* at least once (visitCount > 0) - 0 for a brand new node,
     * which correctly makes fpu == parent utility for the very first pick.
     */
    private fun fpuValue(node: Node, isRoot: Boolean): Double {
        val reductionMax = if (isRoot) rootFpuReductionMax else fpuReductionMax
        val visitedPriorMass = node.children.values
            .filter { it.visitCount > 0 }
            .sumOf { it.prior.toDouble() }
        val reduction = reductionMax * sqrt(visitedPriorMass)
        return node.meanValue - reduction
    }

    /**
     * win/loss margin (KataGo's winLossUtilityFactor) plus a score-aware term derived from
     * the ownership head, both already in "player to move" perspective: katago/python/
     * katago/game/gamestate.py's get_model_outputs only negates the raw ownership map when
     * board.pla == BLACK, meaning the raw net output is relative to the mover - exactly like
     * the value head - so it can be summed/backed-up with the exact same sign convention
     * used for winLoss below, no extra color bookkeeping needed.
     *
     * This is *not* KataGo's real score utility (that uses a dedicated scoreMean/scoreStdev
     * head we don't export - see InferenceWrapper in tools/model_export/pytorch_to_executorch/
     * export_spike.py, which drops it along with the other aux heads). It's an approximation
     * from the ownership map: sum tanh(ownership) over the board as a rough point-margin
     * estimate, squashed through tanh (same qualitative shape as KataGo's own soft-clamped
     * score-to-utility curve - bounded, roughly linear near zero, saturating for blowouts)
     * and normalized by board area.
     *
     * [winLossValue] and [scoreUtilityValue] are kept as separate functions (rather than one
     * combined valueFromOutput like the original spike had) because callers need both the
     * blended sum *and* the bare win/loss term on its own - see [Node.winLossSum]'s kdoc for
     * why reporting can't just reuse the blended value.
     */
    private fun winLossValue(output: KataGoOutput): Double {
        val probs = softmax(output.valueLogits) // [win, loss, noResult]
        return (probs[0] - probs[1]).toDouble()
    }

    private fun scoreUtilityValue(output: KataGoOutput): Double {
        val numCells = boardSize * boardSize
        var scoreEstimate = 0.0
        for (i in 0 until numCells) scoreEstimate += tanh(output.ownershipPretanh[i].toDouble())
        return tanh(scoreEstimate / (0.5 * numCells))
    }

    private fun expand(node: Node, output: KataGoOutput, board: NativeBoard) {
        if (node === root) rootOutput = output

        // policyLogits layout: [numPolicyOutputs, nnXLen*nnYLen + 1]; row 0 is the move
        // KataGo actually plays on. See KataGoOutput's kdoc and PolicyHead.forward in
        // katago's model_pytorch.py.
        val numCells = boardSize * boardSize
        val movePolicyRow = FloatArray(numCells + 1) { output.policyLogits[it] }
        val probs = softmax(movePolicyRow)

        for (y in 0 until boardSize) {
            for (x in 0 until boardSize) {
                if (!board.isLegal(x, y)) continue
                val pos = y * boardSize + x
                node.children[MctsMove(x, y)] = Node(prior = probs[pos])
            }
        }
        if (board.isLegal(-1, -1)) {
            node.children[MctsMove.PASS] = Node(prior = probs[numCells])
        }
        node.expanded = true
    }

    private fun backup(nodePath: List<Node>, leafValue: Double, leafWinLoss: Double) {
        // Both alternate sign by ply since they're always from the mover-to-play's perspective.
        var value = leafValue
        var winLoss = leafWinLoss
        for (node in nodePath.asReversed()) {
            node.visitCount += 1
            node.valueSum += value
            node.valueSqSum += value * value
            node.winLossSum += winLoss
            value = -value
            winLoss = -winLoss
        }
    }

    private fun evaluatePath(path: List<MctsMove>): Pair<KataGoOutput, NativeBoard> {
        val board = NativeBoard(boardSize, rulesName)
        for (move in rootHistory + path) {
            val ok = if (move.isPass) board.pass() else board.play(move.x, move.y)
            check(ok) { "illegal move replayed from tree during search: $move" }
        }
        val (spatial, global) = board.encodeInputsV7(boardSize, boardSize)
        return net.evaluate(spatial, global) to board
    }

    /**
     * LCB (lower confidence bound) move selection (searchhelpers.cpp::
     * getSelfUtilityLCBAndRadius): instead of always taking the most-visited child, prefer a
     * less-visited one if its utility is provably better even after subtracting a confidence
     * margin - guards against a move "winning" purely on visit count from a lucky early
     * rollout. Only considered among children with at least [minVisitPropForLcb] of the
     * most-visited child's visit count, same eligibility floor KataGo uses.
     *
     * Simplified vs KataGo's real version, which re-weights the winning child's visit count
     * so it also wins the plain-visit-count comparisons used elsewhere in the engine (e.g.
     * kata-analyze's reported "visits" column) - we don't have any such secondary consumer,
     * so we just return the best-LCB child directly.
     */
    private fun selectFinalMove(): MctsMove {
        val children = root.children
        if (children.isEmpty()) return MctsMove.PASS

        val mostVisited = children.values.maxOf { it.visitCount }
        var bestMove: MctsMove? = null
        var bestLcb = Double.NEGATIVE_INFINITY

        for ((move, child) in children) {
            if (child.visitCount < minVisitPropForLcb * mostVisited) continue
            val lcb = lowerConfidenceBound(child) ?: continue
            if (lcb > bestLcb) {
                bestLcb = lcb
                bestMove = move
            }
        }
        return bestMove ?: children.maxByOrNull { it.value.visitCount }?.key ?: MctsMove.PASS
    }

    /** Null if too few visits to estimate a variance from (falls back to plain visit count). */
    private fun lowerConfidenceBound(child: Node): Double? {
        if (child.visitCount <= 1) return null
        val mean = child.meanValue
        val variance = max(0.0, child.valueSqSum / child.visitCount - mean * mean)
        val stderr = sqrt(variance / child.visitCount)
        return mean - lcbStdevs * stderr
    }
}
