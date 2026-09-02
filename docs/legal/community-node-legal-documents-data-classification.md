# Feature Data Classification: Community Node 法務文書

ADR 0002 (`docs/adr/0002-feature-data-classification-template.md`) に基づく分類。

### Feature Data Classification
- Feature 名: Community Node 法務文書の生成・公開・per-node 同意
- Durable / Transient: Durable
- Canonical Source: 現行の法務上の事実は各 node の `operator-config.yaml` にある `server` / `features` / `retention` / `safety` / `legal` と型付き capability descriptor。`cn_admin.policies` は公開済み正文 revision の append-only 履歴、`cn_admin.policy_translations` は正文 version に従属する参考訳履歴、client の既存暗号化ローカル同意記録はユーザーの同意行為の canonical record。別系統の catalog / consent store は作らない。
- Replicated?: No。文書は public HTTP で配信するが、gossip や P2P state として複製しない。ローカル同意記録も node や peer へ複製しない。
- Rebuildable From: 現行の公開文書と manifest は `operator-config.yaml` から再生成可能。公開済み正文・参考訳履歴とユーザーの同意記録は再構築不可で、retention cleanup の対象にしない。同意記録の喪失時は再同意する。
- Public Replica / Private Replica / Local Only: 現行・過去の正文、対応する参考訳、manifest metadata は Public Replica。DB の正文・参考訳・server consent 履歴は node-private。desktop の受諾 snapshot・日時・表示言語・app version は Local Only。
- Gossip Hint 必要有無: 不要
- Blob 必要有無: 不要
- SQLite projection 必要有無: 不要。desktop は既存の暗号化 local consent store を利用する。
- 必須 contract: `cn-operator` の生成・typed descriptor・明示保持値契約、`GET /v1/policies` と revision route、public manifest、`cn_admin.policies` の単調増加 append-only 同期、正文 version に固定された参考訳、desktop の slug/version/snapshot 再同意判定。
- 必須 scenario: 認証前に現行・過去正文と厳密な参考訳を取得できること、訳が無ければ正文 fallback が metadata で分かること、未同意では Node 通信を開始しないこと、法務 snapshot 変更で自動再同意になること、accept 競合が保存されないこと、同一 version の本文差し替えと version rollback が起動時に失敗すること。

## 境界

- 対象は当該 community node の運用だけであり、kukuri クライアント本体の規約・プライバシーポリシーとは別である。
- node が扱わない Direct P2P、他 node、peer が保持する copy は当該 node の削除・送信防止の権限外である。
- 生成物は法的助言・完全性保証ではない。第三者 operator も同じ schema と検証を使えるが、自らの実態、契約する provider、補足記述を確認する責任を負う。
