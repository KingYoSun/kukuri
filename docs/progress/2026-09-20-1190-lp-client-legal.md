# #1190 クライアント法務文書のLP公開

## 依頼と境界

Store提出の文書URL整備として、ユーザーがLPのclient privacy／termsへの変更・全文表示と本番公開を承認した。本文・同意契約・legal bundle versionは変更せず、既存`docs/legal/privacy-policy.md`／`terms-of-service.md`を正本として公開面だけを追加する。Node APIの文書・アプリのnetwork gateは対象外。

## 実装

- `/privacy/`／`/terms/`へ日本語正文を全文表示するstatic HTMLを生成する。管理主体、適用範囲、日付、版、履歴を保持。
- 日本語・英語LPから上記へリンクする。英語LPは日本語本文であることを表示。残すNode専用文書リンクはCommunity Node対象と明示。
- `sync-legal.mjs`は正本文書から生成し、`--check`は更新漏れを拒否する。HTMLをescapeし、外部script／tracking／JavaScript依存を追加しない。
- LP Contracts CIで同期、全文一致、両言語リンク、release同期を検査する。

## 検証

- 修正前: Node向けURLが残り、clientページがないことを回帰testでFAIL確認。
- 修正後: Node test5件、sync-release、actionlint、diff check PASS。
- Chromiumで1280×900／390×900、JavaScript無効でprivacy／termsの描画本文を正文と全文照合。overflowなし、両言語LPからのprivacy遷移、Tabでhomeリンクへfocusを確認。スクリーンショットのPC利用規約・mobile privacyを目視確認。
- 法務内容自体の改訂・翻訳は行わない。Store用MSIX candidateのsource／hashは8cfd0e8d時点の記録を維持し、LPだけの追加commitで再buildしたと主張しない。
- 公開URLの到達確認と独立delta監査結果はPR #1191のcommentへ記録する。
- 初回本番検証でCloudflareのemail obfuscationが連絡先をJS依存のplaceholderへ置換することを検出。公式の`email_off`コメントを生成ページ内だけに付け、公開済みの連絡先を含む本文を保持する。zone全体の設定は変更しない。
