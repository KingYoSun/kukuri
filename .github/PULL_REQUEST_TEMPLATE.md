## 概要

<!-- 変更の目的と利用者への結果を短く書く。 -->

## Issue lifecycle

<!-- docs/runbooks/issue-lifecycle.md に従う。区分Aで不要な欄は「対象外: 理由」と書く。区分Cは監査前に自動Close文言を使わず Refs を使う。 -->

- 対象Issue（`Refs #...` / `Closes #...`）:
- リスク区分: A / B / C
- Scope revision / 基準commit:
- Fixed surface inventory（探索方法、変更前後、未分類件数）:
- Sensitive sink と shared helper の全caller確認:
- 対象状態遷移（通常、失敗、retry、restart、複数対象など）:

## AC / invariant traceability

| AC / INVAR ID | 実装箇所 | test / contract / scenario | 結果 |
| --- | --- | --- | --- |
| AC- |  |  |  |

## テスト先行証跡

<!-- fixでは、修正前に失敗したsequenceとtest名、修正後の結果を書く。fix以外は対象外理由を書く。 -->

- failing-before:
- passing-after:

## 種別

- [ ] fix
- [ ] feature
- [ ] refactor
- [ ] contract／scenario
- [ ] docs
- [ ] deps

## 挙動変更

<!-- 変更あり／なし。変更する場合は維持する既存挙動も書く。 -->

## UI変更

<!-- UI変更がなければ「対象外: 理由」と書き、残りは削除してよい。 -->

- 変更分類: 不具合修正／既存画面の改善／新規画面・導線／UI構造・design language再設計
- 対象利用者と利用文脈:
- 対象surfaceと単一目的:
- 主要操作と期待結果:
- 維持する挙動、scope、focus、draft、scroll、戻る文脈:
- 非目標:
- 対象platform／viewport／theme／locale／state:
- Preview（before／after画像または短い動画）:
- Accessibility（keyboard／pointer／touch／screen reader／自動検査）:
- 性能影響と計測、または対象外理由:
- 未確認事項と理由:
- DESIGN／ADR 0014からの例外:

## 検証

<!-- 実行したcommandと結果を書く。実行していない必須validationは理由を書く。 -->

- targeted:
- path別必須validation:
- 未実行と理由:
- CI:

## 独立監査

<!-- 区分Cは必須。PR head SHAとPASS/FAIL/INCONCLUSIVEを記録し、監査後の変更はdelta再監査する。 -->

- 要否と理由:
- 監査対象commit:
- inventory（合計 / 適合 / 不適合 / 未分類）:
- AC / INVAR evidence:
- blocker:
- non-blocker:
- 判定:
- 監査後delta:

## リスク

<!-- データ、互換性、platform、rollbackの観点を書く。 -->

- Existing-gap / Regression:
- New-requirement / Optional-hardening（Close blockerにしていない事項）:
