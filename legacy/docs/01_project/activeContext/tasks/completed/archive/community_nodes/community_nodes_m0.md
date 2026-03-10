# Community Nodes 実装タスク（M0: KIP-0001 仕様固定 + kip_types）

最終更新日: 2026年02月15日

ステータス: 完了（2026年02月15日）
運用: 参照用に保持。新規着手は `docs/01_project/activeContext/tasks/priority/critical.md` から管理する。

目的: KIP-0001 v0.1 を確定し、KIPイベントの共通型/検証基盤を用意する。

参照（設計）:
- `docs/01_project/activeContext/community_node_plan.md`
- `docs/kips/KIP-0001.md`

## M0-1 KIP-0001 ドラフト

- [x] `docs/kips/KIP-0001.md` を追加し、kind/tag/schema/versioning を定義する
- [x] Goals/Non-Goals・用語・鍵配布フローを v0.1 として固定する

## M0-2 kip_types 基盤

- [x] kind 定数/列挙を実装し、KIPイベント判定を共通化する
- [x] `k`/`ver`/`d`/`exp` などの必須タグ検証を実装する
- [x] 39000/39001/39005/39006/39010/39011/39020/39021 の validate を用意する
- [x] 署名検証の切替（verify_signature オプション）を用意する
- [x] 代表的な成功/失敗ケースのユニットテストを追加する

## M0 完了条件

- [x] KIP-0001 v0.1 と `cn-kip-types` の検証ロジックが一致する
