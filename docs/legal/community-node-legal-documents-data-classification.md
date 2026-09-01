# Feature Data Classification: Community Node 法務文書

ADR 0002 (`docs/adr/0002-feature-data-classification-template.md`) に基づく分類。

### Feature Data Classification
- Feature 名: Community Node 法務文書の生成・公開・per-node 同意
- Durable / Transient: Durable
- Canonical Source: 各 node の `operator-config.yaml` にある `server` / `features` / `retention` / `legal`。稼働中 node の `cn_admin.policies` は required 文書の現行 projection、client のローカル同意記録はユーザーの同意行為の canonical record。
- Replicated?: No。文書は public HTTP で配信するが、gossip や P2P state として複製しない。ローカル同意記録も node や peer へ複製しない。
- Rebuildable From: 公開文書、manifest、DB の現行 policy projection は `operator-config.yaml` から再生成可能。ユーザーの同意記録は再構築不可で、喪失時は再同意する。
- Public Replica / Private Replica / Local Only: 文書本文と manifest metadata は Public Replica。DB policy projection は node-private。受諾日時・言語・app version は Local Only（認証後の node consent status は node-local operational record）。
- Gossip Hint 必要有無: 不要
- Blob 必要有無: 不要
- SQLite projection 必要有無: 不要。desktop は既存の暗号化 local consent store を利用する。
- 必須 contract: `cn-operator` の生成契約、`GET /v1/policies`、public manifest の `legal_documents`、`cn_admin.policies` の単調増加 version 同期、desktop の slug/version 再同意判定。
- 必須 scenario: 認証前に本文・slug・version・施行日・言語を取得できること、未同意では Node 通信を開始しないこと、version 上昇で再同意になること、同一 version の本文差し替えと version rollback が起動時に失敗すること。

## 境界

- 対象は当該 community node の運用だけであり、kukuri クライアント本体の規約・プライバシーポリシーとは別である。
- node が扱わない Direct P2P、他 node、peer が保持する copy は当該 node の削除・送信防止の権限外である。
- 現行 KingYoSun Node の operator、連絡先、capability、保持期間を記述する。将来の第三者 operator 向け汎用オンボーディングは別フェーズとする。
