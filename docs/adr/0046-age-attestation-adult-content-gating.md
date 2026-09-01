# ADR 0046: 18歳以上の自己申告と成人向け表現の既定非表示

## Status
Accepted

## Context
- Issue #858(Parent #853 Phase A)。無償 Preview 公開の利用者保護ブロッカーとして、Community Node の利用有無に依存せず、初回起動時に 18 歳以上である旨の自己申告を必須にし、成人向け表現を明示的に許可するまで安全に非表示とする。
- 現行実装には成人向けを示すクライアント可視のラベルが存在しない。CN 側の NSFW 判定(ADR 0028)は index からの除外(`SafetyPolicy::on_high_confidence_nsfw` 既定 `Exclude`)であり、ラベルとして client へ配信されない。ADR 0025 §2.3 / ADR 0028 §2.6 は成人向け・暴力表現のサムネイル代替表示を client UI 設計に委譲している。
- アプリ同意ゲート(`docs/legal/app-consent-data-classification.md`)は、同意まで `DesktopRuntime` を構築せず iroh endpoint を bind しない fail-closed 構造を既に持つ。

## Decision

### 1. 年齢自己申告(age attestation)
- 初回起動時、アプリ利用開始の前提として「18 歳以上である」旨の明示的な自己申告を必須にする。申告が完了するまで `DesktopRuntime` を構築せず、ネットワーク接続を開始しない(既存アプリ同意ゲートと同一の fail-closed)。
- 自己申告は利用規約・プライバシーポリシーへの文書同意とは**別の記録**として `<db_path>.app-consent.json` に保存する(slug 方式の文書レコードとは独立した `age_attestation` レコード)。
- 生年月日・公的身分証等は収集しない。自己申告は公的な年齢確認ではないことを UI 文言と法務文書の双方に明記する。
- 記録はローカルのみ・非複製。新規端末では再申告を求める。

### 2. 成人向け表現の表示設定(adult content display)
- 「成人向け表現を表示する」設定は自己申告とは**別の状態**として扱い、既定 OFF とする。自己申告を済ませただけでは ON にならない。設定画面から明示的に変更できる。
- 設定の canonical source は Rust 側のローカル JSON(`<db_path>.content-display.json`)とし、frontend は mirror に徹する。バイト列を取得しない保証を UI 層のフラグに依存させないため、取得ゲートは Rust 側で enforce する。

### 3. 成人向けラベル(self-label)の信頼元
- v1 のラベル源は**投稿者自己申告**とする。投稿 envelope の署名対象 content(`KukuriPostEnvelopeContentV1`)に `content_labels`(文字列配列、現行の既知値は `adult` のみ)を追加し、投稿者が投稿時に付与する。
- ラベルは投稿者の署名で保護されるが、**申告の真正性は検証できない**。タグの付いていないコンテンツが安全であることを保証しない(fail-open の限界)。複数 Node / peer から同一投稿を観測した場合も、ラベルは署名済み envelope 由来であるため競合しない(envelope が異なれば別投稿として扱う)。
- CN advisory / VLM 判定結果をラベル源として合成することは本 ADR のスコープ外とする(ADR 0026 の relative trust の領分)。

### 4. 取得・キャッシュ制御
- 表示設定 OFF の間、成人向けラベル付き投稿の添付メディアについて、バイト列の取得・プリフェッチ・キャッシュ・デコードを行わない。
  - frontend: プリフェッチ対象から成人向けラベル付き添付を除外する。
  - Rust: `blob_media_payload` は、対象 hash が成人向けラベル付き投稿の添付として観測済み(projection 由来の `adult_media_hashes`)かつ設定 OFF の場合、blob 取得を行わず `None` を返す(fail-closed バックストップ)。
- 表示設定 ON で成人向けメディアを取得する場合は ephemeral fetch(`fetch_blob_ephemeral`)を使い、ローカル blob store(`blobs.db`)へ永続化しない。
- 設定を OFF へ戻した場合、以後の取得を停止し、frontend の in-memory object URL(デコード済み表示)を破棄する。ephemeral fetch のためディスク上に成人向けメディアのキャッシュ残余は発生しない(ラベル付与前に通常経路で取得済みの blob は本 ADR の対象外)。

### 5. 表示
- 成人向けラベル付き投稿は、タイムライン一覧・スレッド詳細・引用/埋め込み・検索結果解決・ブックマーク・プロフィールタイムラインの各表示経路で一貫して代替表示にする。メディアはプレースホルダー(取得もデコードもしない)、テキストは折りたたみの注意表示とする。
- 通知(actor avatar)・DM は v1 ではラベル源を持たないため通常表示となるが、Rust 側取得ゲートは hash 単位で適用される。

## Consequences
- 成人向けラベルの必須 contract / scenario は `docs/legal/age-attestation-data-classification.md` と `docs/legal/adult-content-display-data-classification.md` に定める。
- ラベルなしコンテンツは通常表示(fail-open)であり、本機能はラベル付きコンテンツに対する保護に限られる。この限界は利用規約・UI 文言で明示する。
- CN advisory との合成、通報起点のラベル付与、DM の self-label は将来の別 issue の領分。
