# Community Nodes 実装タスク（M5: Trust v1）

最終更新日: 2026年02月15日

ステータス: 完了（2026年02月15日）
運用: 参照用に保持。新規着手は `docs/01_project/activeContext/tasks/priority/critical.md` から管理する。

目的: Apache AGE を用いた trust 計算（2方式）と attestation（39010）発行を実装し、User API から参照できる状態にする。

参照（設計）:
- `docs/03_implementation/community_nodes/services_trust.md`
- `docs/03_implementation/community_nodes/postgres_age_design.md`
- `docs/03_implementation/community_nodes/outbox_notify_semantics.md`
- `docs/03_implementation/community_nodes/event_treatment_policy.md`
- `docs/03_implementation/community_nodes/user_api.md`

## M5-1 AGE 有効化（Compose/スキーマ）

- [x] Postgres を AGE 対応イメージで起動し、`CREATE EXTENSION age` を migrations で再実行可能にする
- [x] trust 用の graph 初期化（vertex/edge の最小モデル）を行う

## M5-2 outbox consumer（trust worker）

- [x] outbox を `seq` で追従し、at-least-once を冪等で吸収する
- [x] replaceable/addressable の effective view、delete/expiration の扱いを `event_treatment_policy.md` に合わせる

## M5-3 方式A: 通報ベース trust（v1）

- [x] report（39005）と label（39006）を入力に risk_score を算出する（まずは単純集計）
- [x] `attestation(kind=39010)` を署名し、`exp` を付与して配布できるようにする

## M5-4 方式B: コミュニケーション濃度 trust（v1）

- [x] public な相互作用から interaction graph を更新し、単純な密度指標を算出する
- [x] 暗号化領域（friend/invite）は原則入力にしない（プライバシー保護）

## M5-5 User API: trust 参照

- [x] `GET /v1/trust/report-based?subject=pubkey:...`
- [x] `GET /v1/trust/communication-density?subject=pubkey:...`

## M5-6 ジョブ運用（再計算/再発行）

- [x] 再計算ジョブのキュー/進捗/失敗を Postgres に記録する
- [x] Admin API/Console から手動実行/スケジュール変更できる導線を用意する

## M5 完了条件

- [x] 2方式の trust が User API から参照でき、attestation が更新/失効（`exp`）で回る
- [x] 再計算が運用手順として実行できる（ジョブ化/監査/ログ）
