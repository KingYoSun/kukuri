# Issue #1174 URLパースとOGP preview 実装記録

- Issue: [#1174](https://github.com/kukuri-app/kukuri/issues/1174)
- Scope revision: `2026-09-19-v1`
- 基準commit: `d372c91bdc963bda07308a359fe3baeca8e06150`
- リスク区分: C
- 実装head: 本記録を含むPR head

## 実装結果

- `parseSmartText`へ資格情報を含まない絶対HTTP(S) URL segmentを追加し、末尾句読点／不釣合い括弧をlink外へ残した。`safeExternalHref`はMarkdownと投稿本文で共有し、内部route、topic、access token、mentionの優先順を維持した。
- `SmartReferenceText`は`PostCard`から明示opt-inされた場合だけexternal anchorを描画する。DM callerは従来どおりtextのまま。inline linkとOGP cardは元URLを既存`open_external_url`へ渡し、親thread actionをstopする。
- Timeline／Profile／Bookmarks、Thread、Community Indexの`PostCard`へpreviewを明示opt-inした。公開、表示可能、settled、viewport内、document visibleのprimary content先頭URLだけを取得する。private、Composer、adult／trust gate、withdrawn、missing、pending／syncing／failedはinvoke 0。
- Tauriに`fetch_link_preview`を追加した。page／image URLごとにscheme、credential、既定port、hostname、DNS全address、実接続先、redirectを検査し、検証済みaddressをreqwestへ固定する。system proxy、cookie store、Authorization、Refererは使わない。
- HTML tokenizerでOGP／titleを抽出し、PNG／JPEG／GIF／WebPのdeclared MIMEとmagic bytesが一致するbounded imageだけをdata URLへ変換する。WebView CSPへremote imageを追加していない。
- requestはredirect 3、connect 3秒、total 6秒、HTML 512KiB、image 1MiB、同時4件、distinct URLのin-flight待機32件。cacheはprocess-memory 128件／payload 16MiB、success 10分／failure 60秒で、同URLのin-flightを共有する。
- legal bundleをversion 8（施行日2026-09-19）へ更新し、正文、三言語UI、外部送信表示、data-flow inventory、同意分類を同期した。version 7以下は再同意前にReadyへ進まない。年齢自己申告versionは1のまま。
- 正本はADR 0051、`docs/legal/link-preview-data-classification.md`、`DESIGN.md`へ反映した。UI採用条件は`docs/ui-reviews/2026-09-19-1174-link-preview.md`。

## 修正前の再現

基準commit相当の実装へ、`https://example.test/...`をexternal segment／anchorとして期待するtestを先に追加した。

- `internalLinks.test.ts`: URL全体が`text` segmentのままで失敗。
- `PostCard.test.tsx`: URL名の`link` roleが存在せず失敗。

Issue添付画像のplain URL表示と一致した。実装後は同testが成功した。

## Surface／transition evidence

| 条件 | 主な証跡 |
| --- | --- |
| AC-1／3、INV-1／5、TR-1／6 | `internalLinks.test.ts`、`PostCard.test.tsx`、`LinkPreviewCard.test.tsx`。全URLのlink化、前後text、元URL、parent action 0、SVG data拒否、keyboard focusの実描画 |
| AC-2／4／7、INV-2、TR-2〜4／7 | `LinkPreviewCard.test.tsx`、`PostCard.test.tsx`。先頭1件、public成功、private／pending／adult／trust gate invoke 0、offscreen待機、unavailable時cardなし、articleをobserver対象に固定 |
| AC-5〜7、INV-3／4、TR-2／4／5／7 | `commands/link_preview.rs` unit。URL、IP、OGP parse、safe image、private DNS hit 0、redirect先再検証、cache expiry／entry上限。Tauri test binaryは下記制約でlocal実行不能だがcompileはPASS |
| AC-4／9、INV-6、TR-8 | desktop-runtime consent 37件、`App.test.tsx`、`LegalDocumentView.test.tsx`、i18n parity、Tauri canonical legal contractのcompile。version 7→8、age attestation独立 |

## Validation

- `cargo xtask check`: PASS（fmt、clippy、Tauri check、frontend lint／typecheck）
- `cargo xtask test`: PASS（Rust nextest 1,037件、4 skip、doctest、frontend Vitest 1,901件）
- `cargo xtask desktop-ui-check`: PASS（lint、typecheck、Vitest 1,901件、Storybook build、Chromium 366件、visual 42件）
- final UI delta: `LinkPreviewCard.test.tsx`／`PostCard.test.tsx`／`DesktopShellPage.communityIndex.test.tsx` 37件、typecheck、lint、Storybook build PASS
- `cargo xtask tauri-check`: PASS
- `cargo xtask e2e-smoke`: PASS（`desktop_smoke_post_persist` 6 step）
- `cargo test -p kukuri-cli --test command_parity`: 5件PASS、GUI command inventory 161件
- `cargo test -p kukuri-desktop-runtime consent`: 37件PASS
- `cargo xtask oversized-files`: PASS。production `link_preview.rs`は750行、test moduleは234行で閾値未満。既存baselineのwarningのみ
- `git diff --check`: PASS
- Windows Storybook実描画: dark／light、狭幅、長文、image、inline link／card focusを確認。初回確認でempty slotが`display:none`のためIntersectionObserverが発火しない不具合を再現し、親articleをobserveする修正後にcard表示を確認した
- Linux visual baseline: 最終run `35411041556`（head `06a9dd74`）でmissing状態から強制再生成し、`app-consent-en-dark.png`／`app-consent-ja-light-narrow.png` がbadge `V8`、施行日`2026-09-19`、version 8変更要約を同時に表示することを確認した

## 独立監査と修正delta

固定head `d2b8139a9159228179377dae29c53c878a2c8df1` の初回独立監査は `FAIL`（inventory 6 / 適合3 / 不適合3 / 未分類0）だった。

1. `rootMargin: 160px`でviewport外160px以内のcardが取得される: `rootMargin: 0px`へ変更し、observer optionと親article targetをcomponent testで固定した。
2. distinct URLごとのsemaphore待機task／in-flight entryが無制限: distinct URLのin-flightを32件に制限し、超過をtyped `busy`へした。上限到達時にnetwork taskをspawnせずentry数不変のtestを追加した。
3. IPv6 special-purpose `2001:10::/28`等をpublic扱い: IANA special-purposeを含む`2001::/23`、documentation、deprecated 6to4 `2002::/16`、`3fff::/20`を拒否し、特殊用途DNSでHTTP hit 0のtestを追加した。

監査のnon-blockerだったContent-Type欠落画像の許可もADR 0051へ厳密に合わせ、declared raster MIMEとmagic bytesの双方が一致する場合だけ許可するtestへ変更した。修正commitは同じ監査担当へdelta再監査する。

修正head `f558c87d1d4a9984c52ea2ad4addc65f13e7fc5b` のdelta独立監査は `PASS`（inventory 6 / 適合6 / 不適合0 / 未分類0、blocker 0）。non-blockerとしてvisual専用app-consent fixtureのversion/dateが7／2026-09-18のまま残っていたため、productionと同じ8／2026-09-19、stale accepted version 7へ同期し、Linux baselineを最終run `35411041556`で再生成した。

## 未確認と補完

- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --lib -- link_preview`はcompile完了後、test executable起動時にWindowsの`STATUS_ENTRYPOINT_NOT_FOUND`で終了した。既存target削除はせず、freshな`target/issue-1174-tauri`でも同じだったため、test logicのFAILとは区別する。
- Tauri codeは`cargo xtask check`／`tauri-check`でcompile済み。PR CIを最終確認とし、失敗時はmergeしない。packaged WebViewから実在public URLへアクセスするmanual network testは、第三者応答を自動testの成功条件にしないため未実施。

## Scope freeze

private channel／DMの自動preview、利用者設定、投稿時のmetadata固定、複数card、favicon／embed、Community Node proxy、永続cacheはNew-requirement。今回のClose条件へ追加しない。
