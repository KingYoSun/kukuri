# AGENTS.md

このファイルは詳細仕様ではなく、現行の kukuri 実装で作業するための短いポインタです。

## まず読む
- `docs/README.md`
- `docs/runbooks/dev.md`
- `docs/progress/2026-03-10-foundation.md`

## 作業対象
- 新規実装・修正は原則 root workspace の現行実装のみ。
- `legacy/` は参照専用。移植かユーザー明示依頼がない限り編集しない。
- Phase6 着手前の現行スコープでは Windows 対応、DHT discovery、community-node 連携を追加しない。

## 実行入口
- `cargo xtask doctor`
- `cargo xtask check`
- `cargo xtask test`
- `cargo xtask e2e-smoke`
- frontend 単体操作: `cd apps/desktop && npx pnpm@10.16.1 <install|dev|test>`

## 真実の置き場所
- 仕様: `docs/adr/`
- 実行手順: `docs/runbooks/`
- 現状: `docs/progress/`
- 振る舞い: `crates/*` のテストと `harness/scenarios/`

## ガードレール
- 既存コードの丸ごとコピーは禁止。contract または scenario を先に置いてから必要最小限だけ移植する。
- 不具合修正は、必ず先に failing test / contract / scenario で再現してから行う。実機確認は test で表現できない最後の確認に限定する。
- テストでは workspace 全体の長時間の重い再リンクが走っても途中で止めず、原則として完走させて結果を確認する。
- root に新しい長文ドキュメントを増やさない。必要なら `docs/` に置く。
- `console.error` は使わない。
- コミットはユーザーが求めたときだけ行う。
