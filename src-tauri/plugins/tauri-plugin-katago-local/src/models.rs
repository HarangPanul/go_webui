// ping은 실제 기능이 아니라, Rust <-> Kotlin 플러그인 배선(Cargo 경로 의존성 ->
// gen/android 자동 gradle 등록 -> JNI 왕복)이 실제로 동작하는지 확인하기 위한 스텁이다.
// GTP 셸(boardsize/play/genmove/kata-analyze 등)은 이 배선이 검증된 뒤 별도로 추가한다.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingRequest {
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResponse {
    pub value: String,
}

/// GTP 명령 한 줄(개행 없이). gtp::android_transport::AndroidLocalTransport가 GTP
/// 프로토콜의 한 줄 단위 요청/응답 왕복을 그대로 이 커맨드 하나에 매핑한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GtpLineRequest {
    pub line: String,
}

/// `response`는 GTP 응답 블록 전체(예: 성공이면 "= ..." 또는 여러 줄, 실패면
/// "? ...") - 끝을 알리는 빈 줄은 포함하지 않는다(Rust 쪽에서 붙임). 항상 Ok로
/// 돌아온다 - GTP 자체의 "성공/실패"는 이 response 문자열이 "="/"?" 중 뭘로
/// 시작하는지로 표현하고, Err는 오직 JNI/플러그인 호출 자체가 실패했을 때만 쓴다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GtpLineResponse {
    pub response: String,
}
