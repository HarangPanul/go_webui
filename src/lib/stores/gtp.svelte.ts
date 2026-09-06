// GTP 명령 전송 store - GameControls.svelte(자동 분석 스트림 시작/인터럽트)와
// GtpConsole.svelte(수동 명령 콘솔)가 공유한다. Tauri invoke("send_gtp_command"/
// "start_kata_analyze"/"stop_kata_analyze", ...) 호출과 "kata-analyze" 이벤트 구독을
// 이 한 곳에 모아둬서 어디서 뭘 부르는지 추적하기 쉽게 한다.
//
// send()는 GtpConsole의 로그 버퍼(log)에 sent/received/error를 남기는 "기록되는"
// 버전이고, sendSilent()는 콘솔 로그에 남기지 않는 내부용(현재는 send()가 내부적으로만
// 사용). 여러 프로필이 동시에 연결될 수 있으므로 어느 세션으로 보낼지 profileId로
// 명시해야 한다 - GtpConsole이 드롭다운으로 고른 값을 그대로 넘긴다.
//
// startKataAnalyze()/stopKataAnalyze()는 얇은 invoke() 래퍼일 뿐, 어느 세션에 보낼지는
// 백엔드가 "지금 차례 색에 배정된 프로필"을 기준으로 알아서 고른다(commands::gtp 참고) -
// 그래야 두 호출이 항상 같은 세션을 가리켜서 stop이 start가 켠 스트림을 정확히 멈춘다.
// 이 Promise는 분석이 멈출 때까지 계속 pending 상태로 남아있으므로(process.rs::
// AnalysisContext 주석 참고) 호출자(GameControls)가 기다릴지/실패하면 어떻게 할지 결정한다.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { appErrorMessage } from "../appError";
import type { KataAnalyzeEvent } from "../generated/bindings";

export interface GtpLogEntry {
  kind: "sent" | "received" | "analysis" | "error";
  text: string;
}

function createGtpStore() {
  let log = $state<GtpLogEntry[]>([]);

  listen<KataAnalyzeEvent>("kata-analyze", (event) => {
    const top = event.payload.candidates[0];
    const text = top
      ? `info: ${event.payload.candidates.length} candidates, top ${top.move} winrate=${(top.winrate ?? 0).toFixed(3)} visits=${top.visits}`
      : "info: (empty)";
    log.push({ kind: "analysis", text });
  });

  return {
    get log() {
      return log;
    },
    sendSilent(profileId: string, command: string): Promise<string> {
      return invoke<string>("send_gtp_command", { profileId, command });
    },
    async send(profileId: string, command: string) {
      log.push({ kind: "sent", text: command });
      try {
        const response = await this.sendSilent(profileId, command);
        log.push({ kind: "received", text: response || "(empty response)" });
      } catch (e) {
        log.push({ kind: "error", text: appErrorMessage(e) });
      }
    },
    startKataAnalyze(intervalCentiseconds: number): Promise<string> {
      return invoke<string>("start_kata_analyze", { intervalCentiseconds });
    },
    stopKataAnalyze(): Promise<void> {
      return invoke<void>("stop_kata_analyze");
    },
  };
}

export const gtpStore = createGtpStore();
