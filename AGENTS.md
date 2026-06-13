# AGENTS.md

このファイルは詳細仕様ではなく、現行の kukuri 実装で作業するための短いポインタです。

## まず読む
- `docs/README.md`
- `docs/runbooks/dev.md`
- `docs/progress/2026-03-10-foundation.md`
- `REFACTORING.md`（リファクタリング・構造整理・大きめの移動/抽出を行う場合）
- `DESIGN.md`（UI/UX 作業時のビジュアル仕様。フロー/ガードレールは `docs/adr/0014-uiux-dev-flow.md`）

## 作業対象
- 新規実装・修正は原則 root workspace の現行実装のみ。
- 現行スコープの参照優先順位は `docs/README.md` に従う。
- foundation baseline は `docs/progress/2026-03-10-foundation.md`。
- builder preview / 配布 / 初回体験は `docs/progress/2026-04-16-mvp-builder-preview-plan.md`。
- Windows desktop support、seeded DHT discovery、community-node connectivity/auth、social graph v1、private channel audience v1 は current scope に含まれる。

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
- ビジュアル仕様: `DESIGN.md`
- UI/UX フロー・ガードレール: `docs/adr/0014-uiux-dev-flow.md`
- UI review record: `docs/ui-reviews/`
- 振る舞い: `crates/*` のテストと `harness/scenarios/`

## ガードレール
- 既存コードの丸ごとコピーは禁止。contract または scenario を先に置いてから必要最小限だけ移植する。
- 不具合修正は、必ず先に failing test / contract / scenario で再現してから行う。実機確認は test で表現できない最後の確認に限定する。
- リファクタリング、ファイル分割、責務境界変更、dead code 削除を行う場合は、先に `REFACTORING.md` を読む。
- リファクタリングPRでは、機能追加・仕様変更・依存更新を混ぜない。
- 変更pathごとの必須validationは `REFACTORING.md` の path別検証マトリクスに従う。
- テストでは workspace 全体の長時間の重い再リンクが走っても途中で止めず、原則として完走させて結果を確認する。
- root に新しい長文ドキュメントを増やさない。必要なら `docs/` に置く（例外: ビジュアル仕様 `DESIGN.md` は gestaloka 慣習に合わせ root に置く）。
- `console.error` は使わない。
- コミットはユーザーが求めたときだけ行う。

## 通信経路
- 本プロジェクトの基本優先度は `Direct P2P -> Relay Supported P2P -> Relay Fallback`。
- `Direct P2P` は manual ticket / `addr_hint` / DHT などの直接到達情報で接続し、relay URL を候補に含めない経路。
- `Relay Supported P2P` は topic rendezvous / discovery / hole punching / endpoint assist に community-node や relay を使い、同じ topic を subscribe している client 同士の P2P 接続を成立させる経路。これは fallback ではない。
- `Relay Fallback` は Direct P2P と Relay Supported P2P が成立しない場合だけ、gossip/docs/blob など実データを含む通信が relay 経由になる経路。
- `cn-user-api` は topic rendezvous state の owner。topic presence は Valkey/Redis-compatible KV に TTL 付き ephemeral state として置き、`cn-iroh-relay` は純粋な iroh relay のままにする。
- relay-only の実装やテストは通常成功経路として扱わず、`Relay Fallback` として明示する。
