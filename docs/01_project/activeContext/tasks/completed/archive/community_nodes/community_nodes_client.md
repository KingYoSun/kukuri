# Community Nodes 実装タスク（Client: ノード採用/鍵/暗号投稿）

最終更新日: 2026年02月15日

ステータス: 完了（2026年02月15日）
運用: 参照用に保持。新規着手は `docs/01_project/activeContext/tasks/priority/critical.md` から管理する。

目的: Community Node 連携に必要な UI/ストア/暗号投稿/鍵管理をクライアントに実装する。

参照（設計）:
- `docs/01_project/activeContext/community_node_plan.md`
- `docs/kips/KIP-0001.md`

## Client-1 ノード採用/設定UI

- [x] Community Node の Base URL/トークン管理 UI を追加する
- [x] 連携機能の ON/OFF（access control / label / trust / search）をストアで管理する

## Client-2 Node API 連携

- [x] bootstrap/labels/trust/search/consent の API 呼び出しを実装（invite/keys は P2P-only の Access Control を使用）。
- [x] 取得結果を設定画面に反映する

## Client-3 鍵管理と暗号投稿

- [x] key.envelope を受理してセキュアストレージへ保存する
- [x] scope 別の投稿暗号化/復号（friend/friend_plus/invite）を実装する
- [x] 鍵未取得時は暗号文プレースホルダーを表示する

## Client-4 テスト

- [x] 暗号投稿の暗号化/復号フローをテストで確認する
- [x] 設定画面の Community Node 反映をテストに組み込む

## Client 完了条件

- [x] 鍵同期→暗号投稿→復号表示のフローが成立する
