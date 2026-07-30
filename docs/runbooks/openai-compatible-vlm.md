# OpenAI-Compatible VLM Moderation Provider (Community Node Operator)

最終更新日: 2026-07-30

## 目的

Community Node operator が **自分の** OpenAI-compatible endpoint（self-host vLLM / 外部 API）を
使って、kukuri の非決定論的 moderation provider（`openai-compatible-vlm`、Issue #420 /
ADR 0028）を設定・検証・運用するための手順。

kukuri 本体は endpoint / model / API key を同梱・共有・代理利用しない（operator-owned。
#391 Project Arachnid Shield と同じ方式）。

## 位置づけ（何をする provider か）

- `general` / `unknown_csam` slot 用の **classifier** provider。判定は常に suspected
  （`basis = classifier_score`）で、確定（confirmed）には決して昇格しない。
- known-CSAM の確定判定は本 provider の役割ではない（`known_csam` slot には設定できない。
  known-match は #391 系 provider が担う）。
- 同一 scan が moderation verdict と **descriptive 検索タグ**の両方を生成する（二重スキャン
  しない。タグが index されるのは `allow` verdict の media のみ）。

## 設定（env）

| env | 必須 | 説明 |
|---|---|---|
| `COMMUNITY_NODE_SAFETY_PROVIDER_GENERAL=openai-compatible-vlm` | どちらか | general slot に設定 |
| `COMMUNITY_NODE_SAFETY_PROVIDER_UNKNOWN_CSAM=openai-compatible-vlm` | どちらか | unknown_csam slot に設定 |
| `COMMUNITY_NODE_VLM_API_BASE_URL` | ✔ | endpoint の base URL（`/v1/chat/completions` を付けない） |
| `COMMUNITY_NODE_VLM_MODEL` | ✔ | model 名（例: `inclusionAI/SingGuard-2b`） |
| `COMMUNITY_NODE_VLM_API_KEY` | - | Bearer key。**未設定なら無認証で接続**（self-host 向け）。値をコミットしない |
| `COMMUNITY_NODE_VLM_RESPONSE_FORMAT` | - | `json`（既定）/ `guard` |
| `COMMUNITY_NODE_VLM_API_TIMEOUT_SECS` | - | HTTP timeout（既定 60） |
| `COMMUNITY_NODE_SAFETY_SUSPECTED_THRESHOLD` | - | suspected 閾値 1-100（既定 70 = 0.7） |
| `COMMUNITY_NODE_SAFETY_SUSPECTED_SIGNAL_VISIBILITY` | - | suspected advisory の配布範囲 `local`（既定）/ `subscribed_nodes` / `public` |
| `COMMUNITY_NODE_SAFETY_OPERATOR_REVIEW` | - | operator レビュー（`cn-cli moderation edit/reissue`）の有効化（既定 false） |
| `COMMUNITY_NODE_MEDIA_FETCH_MAX_BYTES` | - | media scan 用一時 fetch のサイズ上限（既定 33554432 = 32 MiB。超過は fail-closed） |
| `COMMUNITY_NODE_MEDIA_FETCH_TIMEOUT_SECS` | - | media scan 用一時 fetch の timeout 秒（既定 30。超過は fail-closed） |

operator-config.yaml では `safety.providers.general` / `safety.providers.unknown_csam` と
`safety.moderation`（`suspected_threshold` / `suspected_signal_visibility` / `operator_review`）
に対応する。readiness check `classifier_providers_resolvable` が実装名を静的検証する。

## 応答形式の選び方

- **`json`（既定）**: 指示追従できる汎用 VLM 向け。system prompt で
  `{"categories":[{"category":"csam|cse|grooming|nsfw|spam|malware|phishing","score":0-1}],"tags":[...]}`
  の厳密 JSON を要求する。critical カテゴリの判定とタグ生成が可能。未知カテゴリや解析不能
  応答は fail-closed（hold）。
- **`guard`**: SingGuard 等の guard 系モデル（chat template が「1 行目 safe/unsafe +
  `<answer>` カテゴリ」を強制するもの）向け。確信度は先頭トークンの logprob から導く。
  guard の粗いカテゴリからは critical を断定できないため、**general カテゴリにのみ写像**する
  （A→nsfw / D→phishing / E→spam / B・C・G→nsfw / F=政治的内容は moderation 対象にしない）。
  タグは生成されない。

## fail-closed の挙動

- endpoint 不達 / 401 / 429 / 5xx / timeout / 解析不能応答は scan 失敗として扱われ、対象は
  index されない（hold。`allow` に落ちる経路は無い）。
- media 参照 post の media scan は `MediaFetcher`（blob の一時 fetch。#609 で cn-indexer に
  実装済み）経由で行われる。blob が未複製 / peer 不達で取得できない・サイズ上限超過・
  timeout・content type 不明のいずれも fail-closed（hold）で、media 参照 post は index
  されない。取得した bytes はローカルストアへ書き込まれない（no permanent blob storage）。
- endpoint / model の env 欠落は runtime 構築の失敗（ingest 起動せず）。エラーには env の
  **名前のみ**が含まれ、値は出力されない。

## 検証（実機 e2e）

endpoint を起動した状態で:

```bash
COMMUNITY_NODE_VLM_API_BASE_URL=http://<host>:<port> \
COMMUNITY_NODE_VLM_MODEL=<model> \
COMMUNITY_NODE_VLM_RESPONSE_FORMAT=guard \
cargo test -p kukuri-cn-safety-vlm --test live_endpoint -- --ignored --nocapture
```

無害テキスト → allow、phishing/spam テキスト → 非 index、画像（data URL）scan の成功を確認する。

## appeal / operator レビュー運用

- user / client からの異議申し立ては `POST /v1/report` の `appeal.risk_signal_id` で届き、
  対象 advisory が `disputed` になる。
- レビューは `cn-cli moderation` で行う:

```bash
cn-cli moderation list-signals
cn-cli moderation clear --id <signal-id>      # 認容（trust 寄与が戻る）
cn-cli moderation reject --id <signal-id>     # 棄却
COMMUNITY_NODE_SAFETY_OPERATOR_REVIEW=true cn-cli moderation edit --id <signal-id> --category nsfw --confidence 20
COMMUNITY_NODE_SAFETY_OPERATOR_REVIEW=true cn-cli moderation reissue --id <signal-id> --severity low
```

- 是正は node-local advisory（risk signal）に対して行われ、user の canonical state や署名済み
  moderation event は変更されない。`cleared` の advisory は配布に残り、受け手の trust 供給層が
  寄与から除外する。

## プライバシー / データの扱い

- scan 対象の text / media bytes は moderation 判定のためにのみ endpoint へ送信される。
  **外部 API を使う場合、投稿内容が第三者へ送信されることを意味する**。プライバシー方針に
  応じて self-host endpoint の利用を検討すること。
- Community Node は blob 本体を恒久保存しない。scan のための fetch は一時的で、scan 後に
  破棄される。
- 応答 body / credentials はエラーメッセージ・ログに出力されない。
