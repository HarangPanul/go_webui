// invoke()가 reject한 값에서 사람이 읽을 메시지를 뽑아내는 헬퍼.
//
// src-tauri/src/error.rs::AppError가 6단계(tauri-specta 연결)부터 plain string이
// 아니라 `{ kind, message }` 구조화된 객체로 직렬화되도록 바뀌었다(kind는 지금 당장
// 프런트가 분기하는 곳은 없지만 나중을 위한 태그, message는 기존과 똑같은 한국어
// 에러 문구). 예전에는 `String(e)`로 그대로 표시해도 됐지만 이제 그러면
// "[object Object]"가 보이므로, 이 함수로 message 필드를 꺼내 쓴다.
//
// invoke()가 reject하는 값이 항상 AppError라는 보장은 없으므로(예: Tauri
// 프레임워크 자체 에러 등) 문자열/그 외 형태도 방어적으로 처리.
export function appErrorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (
    e &&
    typeof e === "object" &&
    "message" in e &&
    typeof (e as { message: unknown }).message === "string"
  ) {
    return (e as { message: string }).message;
  }
  return String(e);
}
