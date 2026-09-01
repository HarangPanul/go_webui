// 바둑 규칙 모듈: 1차 버전은 최소 범위(덤, 기본 종국 처리)만 지원하고
// 이후 중국식/일본식/핸디캡 등으로 교체 가능하도록 인터페이스로 분리

export interface GoRuleSet {
  komi: number;
  isGameEnd(passStreak: number): boolean;
  // TODO: scoreGame, isLegalMove(ko rule 등) 추가
}

export const defaultRuleSet: GoRuleSet = {
  komi: 6.5,
  isGameEnd: (passStreak) => passStreak >= 2,
};
