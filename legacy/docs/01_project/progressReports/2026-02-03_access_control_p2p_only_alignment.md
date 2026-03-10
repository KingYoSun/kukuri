# Access Control P2P-only 整合（invite/keys撤去）

- User API から `/v1/invite/redeem` `/v1/keys/envelopes` を削除し、P2P-only を正とする方針に統一
- Tauri の Community Node UI/Handler を `access_control_request_join` ベースへ切替し、鍵同期 UI を撤去
- `community-node.invite` E2E を P2P join 前提に更新（環境変数で有効化）
- ドキュメント/タスク一覧（roadmap/client/user_api/summary）を更新
