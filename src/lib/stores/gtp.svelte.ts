// GTP 명령 전송 store - GameControls.svelte(자동 분석 스트림 시작/인터럽트)와
// GtpConsole.svelte(수동 명령 콘솔)가 공유한다. Tauri invoke("send_gtp_command"/
// "start_kata_analyze", ...) 호출과 "kata-analyze" 이벤트 구독을 이 한 곳에 모아둬서
// 어디서 뭘 부르는지 추적하기 쉽게 한다.
//
// send()는 GtpConsole의 로그 버퍼(log)에 sent/received/error를 남기는 "기록되는"
// 버전이고, sendSilent()는 GameControls가 kata-analyze 스트림을 멈추려고 아무 명령이나
// 하나 보내는 내부용으로, 콘솔 로그에는 남기지 않는다.
// startKataAnalyze()는 얇은 invoke() 래퍼일 뿐, 그 결과를 기다릴지/실패하면 어떻게
// 할지는 호출자(GameControls)가 결정한다(이 Promise는 분석이 멈출 때까지 계속 pending
// 상태로 남아있으므로 - process.rs::AnalysisContext 주석 참고).
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
    sendSilent(command: string): Promise<string> {
      return invoke<string>("send_gtp_command", { command });
    },
    async send(command: string) {
      log.push({ kind: "sent", text: command });
      try {
        const response = await this.sendSilent(command);
        log.push({ kind: "received", text: response || "(empty response)" });
      } catch (e) {
        log.push({ kind: "error", text: appErrorMessage(e) });
      }
    },
    startKataAnalyze(intervalCentiseconds: number): Promise<string> {
      return invoke<string>("start_kata_analyze", { intervalCentiseconds });
    },
  };
}

export const gtpStore = createGtpStore();
