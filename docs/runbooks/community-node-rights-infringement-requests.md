# Community Node 権利侵害申出の運用

## 目的と責任境界

この runbook は、`rights_request_endpoint` を明示的に有効化した Community Node が、権利侵害申出を受付・審査し、自 node 内の送信防止へ接続する手順を定める。仕様上の境界は [ADR 0033](../adr/0033-rights-infringement-request-intake.md) を正とする。

Community Node が実行できるのは、その node 自身の索引、検索、発見、推薦、moderation、対応済み blob cache に対する措置だけである。他 node、第三者端末、投稿正本、Direct P2P、暗号化 relay packet、既取得データには強制力がない。申出画面はこの範囲をフォームより先に示し、現行 `scope_revision` への明示同意がなければ受付しない。

## 有効化

`operator-config.yaml` に次を追加する。

```yaml
features:
  rights_request_endpoint: true

deploy:
  legal_data_key_secret_id: kukuri-cn-legal-data-key

manifest:
  rights_request_initial_response_target_days: 7
```

日数は初回応答の運用目標であり、法定期限や措置の保証ではない。有効化後、manifest の `rights_request_url` と `rights_request_policy_url`、`/v1/rights-requests/scope`、申出画面 `/rights-requests/new` を確認する。scope の `available_actions` は現在の manifest capability から生成される。

## 日常のトリアージ

IAP 経由の運営画面で「権利侵害申出」を開くか、CLI を使う。

```powershell
cargo run -p kukuri-cn-cli -- rights-requests list --limit 50
cargo run -p kukuri-cn-cli -- rights-requests show --id <REFERENCE_ID>
```

`verified_scope` だけが `received` になる。`unverified_scope` は `needs_information`、node の capability または authority 外は `out_of_scope` で受け付けられる。client の申告だけを根拠に `received` や `actioned` へ変更しない。

審査を始めるときは、詳細画面の確認画面を経るか、現在の `version` を指定する。

```powershell
cargo run -p kukuri-cn-cli -- rights-requests transition `
  --id <REFERENCE_ID> --expected-version <VERSION> `
  --actor legal@example.net --status reviewing `
  --public-message "審査を開始しました"
```

追加情報依頼は `needs-information`、送信者への照会は `sender-contacting`、棄却は `declined`、審査で authority 外と判明した場合は `out-of-scope` を使う。`delivery-status` の既定は `status_surface` で、自動 SMTP 配信を意味しない。外部で連絡した場合は、その配送結果を値として記録する。

## 送信防止を伴う措置

申出を `reviewing` または `sender_contacting` にした後、対象、権利根拠、requested capability、node-local な対象記録を確認する。措置は申出状態、append-only event、送信防止 record、operator audit が同じ transaction で確定する。

```powershell
cargo run -p kukuri-cn-cli -- rights-requests action `
  --id <REFERENCE_ID> --expected-version <VERSION> `
  --actor legal@example.net `
  --capabilities community-index,search,discovery,recommendation `
  --public-message "このノードの索引・検索・発見・推薦から対象を除外しました"
```

公開メッセージには申出人 PII、内部メモ、operator 名、非公開の判断根拠を含めない。送信防止の解除は既存の `transmission-prevention release` 手順で行い、解除だけでは古い索引を復活させず fresh scan / ingest を要求する。

## 追跡 secret と個人情報

- 追跡 secret は受付直後に一度だけ表示され、DB には SHA-256 hash だけが保存される。
- 参照 ID と secret の不一致は、存在しない参照 ID と同じ公開応答にする。
- 証拠は URL、hash、外部識別子だけを扱う。ファイルや侵害対象コンテンツを upload・複製しない。
- 申出本体は `cn_legal.rights_requests`、連絡先・本人／代理権情報・証拠参照は専用鍵で暗号化した `cn_legal.sensitive_items` に local-only で保存し、一般通報、public replica、operator audit へ複製しない。
- 公開 status は scope、状態、更新時刻、公開メッセージだけを返す。
- 保持期限、legal hold、限定 export は [ADR 0034](../adr/0034-community-node-case-retention-legal-hold.md) と [発信者情報開示 runbook](community-node-sender-information-disclosure.md) に従う。

## 無効化

`features.rights_request_endpoint: false` にして再配備すると manifest と公開受付が無効になる。既存 record は無効化だけでは即時削除せず、文書化された保持期限に達すると通常読取から除外され、定期 sweep で削除される。DB を直接編集して append-only event を破壊しない。
