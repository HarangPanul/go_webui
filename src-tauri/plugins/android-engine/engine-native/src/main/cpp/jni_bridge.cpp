// JNI bridge for io.katago.android.nativeboard.NativeBoard.
//
// This wraps just enough of KataGo's own game/board.cpp, game/rules.cpp,
// game/boardhistory.cpp and neuralnet/nninputs.cpp to (a) play a legal game of Go
// and (b) encode the exact same [22,19,19] spatial / [19] global input tensors
// that KataGo's own C++ engine feeds to the neural net (fillRowV7, model version
// 17). We deliberately reuse this code instead of reimplementing Go rules in
// Kotlin -- see docs/decisions/0001-model-and-pipeline.md.
#include <jni.h>

#include <memory>
#include <mutex>
#include <string>

// CMakeLists.txt adds katago/cpp as an include directory, so these resolve there
// (matching how katago's own .cpp files refer to their siblings).
#include "game/board.h"
#include "game/boardhistory.h"
#include "game/rules.h"
#include "neuralnet/nninputs.h"

namespace {

struct GameState {
  Board board;
  BoardHistory hist;
  Player nextPlayer;
};

GameState* handleToState(jlong handle) {
  return reinterpret_cast<GameState*>(static_cast<intptr_t>(handle));
}

// Board::initHash() populates the Zobrist tables Board/BoardHistory assert on; it must run
// exactly once before the first Board is constructed (see game/board.cpp's
// IS_ZOBRIST_INITALIZED assert, hit while writing this bridge).
void ensureKataGoInitialized() {
  static std::once_flag once;
  std::call_once(once, [] { Board::initHash(); });
}

}  // namespace

extern "C" {

JNIEXPORT jlong JNICALL
Java_io_katago_android_nativeboard_NativeBoard_nativeCreate(
    JNIEnv* env, jclass, jint boardSize, jstring jRulesStr) {
  ensureKataGoInitialized();

  const char* rulesCstr = env->GetStringUTFChars(jRulesStr, nullptr);
  std::string rulesStr(rulesCstr);
  env->ReleaseStringUTFChars(jRulesStr, rulesCstr);

  auto* state = new GameState();
  state->board = Board(boardSize, boardSize);
  state->nextPlayer = P_BLACK;

  Rules rules = Rules::parseRules(rulesStr);
  state->hist.clear(state->board, state->nextPlayer, rules, 0);

  return static_cast<jlong>(reinterpret_cast<intptr_t>(state));
}

JNIEXPORT void JNICALL
Java_io_katago_android_nativeboard_NativeBoard_nativeDestroy(JNIEnv*, jclass, jlong handle) {
  delete handleToState(handle);
}

// x < 0 (any negative) is treated as a pass, matching the PASS_LOC convention.
JNIEXPORT jboolean JNICALL
Java_io_katago_android_nativeboard_NativeBoard_nativePlay(
    JNIEnv*, jclass, jlong handle, jint x, jint y) {
  GameState* state = handleToState(handle);

  Loc loc = (x < 0 || y < 0)
      ? Board::PASS_LOC
      : Location::getLoc(x, y, state->board.x_size);

  if (!state->hist.isLegal(state->board, loc, state->nextPlayer)) {
    return JNI_FALSE;
  }
  state->hist.makeBoardMoveAssumeLegal(state->board, loc, state->nextPlayer, nullptr);
  state->nextPlayer = getOpp(state->nextPlayer);
  return JNI_TRUE;
}

JNIEXPORT jint JNICALL
Java_io_katago_android_nativeboard_NativeBoard_nativeGetNextPlayer(JNIEnv*, jclass, jlong handle) {
  return handleToState(handle)->nextPlayer;
}

// Non-mutating legality check, so callers (e.g. Mcts's move enumeration) don't need to
// speculatively play-and-undo.
JNIEXPORT jboolean JNICALL
Java_io_katago_android_nativeboard_NativeBoard_nativeIsLegal(
    JNIEnv*, jclass, jlong handle, jint x, jint y) {
  GameState* state = handleToState(handle);
  Loc loc = (x < 0 || y < 0)
      ? Board::PASS_LOC
      : Location::getLoc(x, y, state->board.x_size);
  return state->hist.isLegal(state->board, loc, state->nextPlayer) ? JNI_TRUE : JNI_FALSE;
}

// Fills spatialOut (length 22*nnYLen*nnXLen, NCHW layout) and globalOut (length 19)
// with the exact same values KataGo's own engine would feed to a model-version-17
// (transformer, e.g. tf3-b11c768/"t11b") net for the current position.
JNIEXPORT void JNICALL
Java_io_katago_android_nativeboard_NativeBoard_nativeEncodeInputsV7(
    JNIEnv* env, jclass, jlong handle, jint nnXLen, jint nnYLen,
    jfloatArray spatialOut, jfloatArray globalOut) {
  GameState* state = handleToState(handle);
  MiscNNInputParams params;

  jfloat* spatial = env->GetFloatArrayElements(spatialOut, nullptr);
  jfloat* global = env->GetFloatArrayElements(globalOut, nullptr);

  NNInputs::fillRowV7(
      state->board, state->hist, state->nextPlayer, params,
      nnXLen, nnYLen, /*useNHWC=*/false, spatial, global);

  env->ReleaseFloatArrayElements(spatialOut, spatial, 0);
  env->ReleaseFloatArrayElements(globalOut, global, 0);
}

}  // extern "C"
