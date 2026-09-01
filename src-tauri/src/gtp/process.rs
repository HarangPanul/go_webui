// 원격 서버에서 실행 중인 `katago gtp` 프로세스의 SSH 채널 I/O 핸들
// TODO: russh Channel의 write half/read half를 보관, GtpSession으로 노출

pub struct GtpSession {
    // TODO: channel write half, reconnect 상태 등
}
