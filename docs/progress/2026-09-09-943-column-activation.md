# #943 Column 再選択の route 巻き戻り

## 現在判定

- 判定: 実装・ローカル UI 検証・独立監査 PASS。最終 PR head の必須 CI を待つ。
- Issue: [#943](https://github.com/KingYoSun/kukuri/issues/943)、種別 `fix`、リスク区分 B（共有 route／選択状態）。
- Scope revision: `2026-09-08-column-activation-ci-v1`。AC-1～3／INVAR-1～2 は Issue 本文を維持。
- 基準 commit: `d2610af4467f6a91fefa208952abc5728496dc29`。
- [承認済みプラン](../../.codex.plans/2026-09-09-issue-943-column-activation.md)。2026-09-09 に実装・commit・PR・CI 成功後の merge を承認済み。独立監査後に merge／Close を判定する。

## 原因と変更結果

Metaverse → Live の navigate は hash を直ちに変更するが、React Router の location render は後で追いつく。その間に Metaverse を選び直すと、旧 `useSyncRoute` は最新の要求を **render 済みの Metaverse URL** と比較して同一と判定し、navigate を省略して pending も解除した。store は一旦 Metaverse active になる一方、実 hash は Live に残る。その Live location が投影され、最後の選択が Live へ戻る。

変更後は、navigation の重複判定に実行時の hash を使う。hash がない hook／初期環境では従来の location へ fallback する。同じ hash への再要求は history を増やさず、render がまだ追いつかない間は pending を維持する。観測済み target では pending を解除する。URL の組立て、push／replace の選択、scope、nullable override、初期 normalize の契約は変えない。

製品変更は [useSyncRoute.ts](../../apps/desktop/src/shell/routing/useSyncRoute.ts) の比較と pending 更新だけ。Canvas の focus／scroll、IntersectionObserver、runtime の suspension 式、session lifecycle、storage、network、UI の見た目は変更していない。

### 修正前の再現

元の offscreen test は、変更なしの Windows／Chromium で単独1回（14.0秒）、5回／2 workers（30.7秒）が成功した。過去 CI failure の同条件再現や解消の証拠にはしない。

次に順序を固定した追加 test を、製品変更前に実行した。

| event | active Column | 修正前 hash | 修正後 hash |
| --- | --- | --- | --- |
| 初期 | Metaverse | `/game?topic=…` | `/game?topic=…` |
| Live header pointerdown → store 更新の microtask | Live | `/live?topic=…` | `/live?topic=…` |
| Metaverse header pointerdown → store 更新の microtask | Metaverse | **`/live?topic=…` のまま** | `/game?topic=…` |
| router の反映後 | 修正前 Live／修正後 Metaverse | Live に巻き戻る | 最後の選択を維持 |

- unit: `useSyncRoute.test.tsx` の `keeps the latest Metaverse selection when Live navigation has not rendered yet` が期待 `#/game`／実際 `#/live` で失敗（1 FAIL）。実 hash の同期更新と、まだ観測されていない location を同時に与える。
- browser: `column-immersive.spec.ts` の `rapid header reselection keeps Metaverse active while the previous route is pending` が期待 `/game`／実際 `/live` で失敗（1 FAIL、7.2秒）。最終 error-context は Live Active／Metaverse Inactive。製品を mock した renderer ではなく、production bundle と実 Chromium を使用した。
- browser 条件: Windows、Node 22.14.0、pnpm 10.16.1、Playwright 1.62.1 の Chromium、900×900、en-US／UTC、dark、通常 motion、fresh の5 Column＋Live＋Metaverse、mock API。`CI=1` で既存 server 再利用なし、`--workers=1 --retries=0 --trace=on`。
- `page.evaluate` 内で2回の header pointerdown の間に microtask だけを許可し、store 更新を router の transition render に先行させる。rAF callback のキャンセルを無効化したり、キャンセル済み callback を強制実行したりしていない。
- 修正後に同じ unit と browser が成功。browser の最終確認は active／hash／runtime／scene／canvas を単一 DOM 観測で検査する。元の offscreen test とその assertion は維持。

保存した trace 抜粋: [before sequence](2026-09-09-943-column-activation-evidence/before-sequence.json)、[after sequence](2026-09-09-943-column-activation-evidence/after-sequence.json)、[before 最終画面](2026-09-09-943-column-activation-evidence/before-rapid.jpeg)、[after 最終画面](2026-09-09-943-column-activation-evidence/after-rapid.jpeg)。画像は同条件 trace の最終 screencast frame。

過去の [Fast CI failure](https://github.com/KingYoSun/kukuri/actions/runs/34167073580/job/101880130851) は trace がなく、当時と今回の全イベント列が同一とは断定しない。今回、同じ再選択と最終巻き戻りを現行経路で再現できた原因を修正した。遅延 focus は今回の再現に必要な原因ではなく、遅い route の選択反映後にも生じる結果である。`loadTopics` 後の別変更や予約正規化の一般的な対策は追加していない。

## Inventory と状態遷移

登録点は `useDesktopShellRouting → useSyncRoute` の1か所。CodeGraph の `callers useSyncRoute`／`callers syncRoute` に加え、nested callback を `rg -n 'syncRoute\(' apps/desktop/src/shell -g '*.ts' -g '*.tsx' -g '!*.test.*'` で補完した。[44 call sites の一覧](2026-09-09-943-column-activation-evidence/sync-route-callers.txt)は比較用の固定 inventory であり、件数だけを適合根拠にしない。

| group | 全 member の列挙元 | 副作用・維持する契約 | 主な回帰証拠 |
| --- | --- | --- | --- |
| INV-1 選択要求 | `DesktopShellPage.tsx`、workspace の `activate`／`close`／composer・domain footer、`useDesktopShellRouting.ts` の Settings／Thread／Author／DM／各 section 操作 | route push／replace と active 更新。明示した最後の選択を反映し、通常 focus／parent close／scope を維持 | 新規 rapid／session test、Canvas、hash-routing、column-scope、routing hook tests |
| INV-2 共有同期・遅着 | `useRouteSynchronization.ts` の2予約、`actions/composeInteractions.ts`、`actions/liveGame.ts`、`actions/messageReactionSocial.ts`、`actions/profileTopicChannel.ts`、`useDesktopShellActions.ts`、`page/DesktopShellSettingsDrawer.tsx` | 初期／通常／history 投影、取得・作成後の replace、Settings focus 復元。URL equality と pending の意味だけ変更。旧 render を current hash と同一視しない | 重複 push の抑止と pending 維持、history 追越し後の replace、既存 pending/history／nullable override、全 frontend tests |
| INV-3 可視性・描画 | Canvas observer → workspace visible 集合 → `projectColumnRuntime` → provider → Surface／Metaverse View／Scene／Stream | `immersive && !visible && !audioFocused` を維持。visible と active を混同せず、明示 audio focus の例外を保つ。session を破棄しない | runtime unit、observer 同数置換、元 offscreen、session 継続 browser、WebView2 観測 |

期待した inventory 差分は旧比較の置換だけ。公開入口・callback 登録・永続状態・network sink の追加／削除は 0。実装者確認は3 group 適合／不適合0／未分類0。独立監査は別途同じ source から再構築する。

| transition | 検査した sequence | 結果・証跡 |
| --- | --- | --- |
| TR-1 | fresh → 7 Column → room 作成・Metaverse 表示 | active／rendering、Live offscreen。元 offscreen browser |
| TR-2 | Metaverse → Live | Metaverse suspended、room／canvas の維持。元 offscreen と WebView2 |
| TR-3 | Live → Metaverse を router の render 前に要求 | 修正前の hash 不更新と最終巻き戻り → 修正後は hash／active／runtime／scene 一致。新規 unit／browser と before／after |
| TR-4 | 正当な次の pointer／Tab／Control Center Focus／history back・forward／mobile swipe | 新しい要求を妨げない。Canvas・routing・column-scope・hash-routing・mobile gesture、session browser、native Tab |

`ColumnCanvas` の unmount cleanup／削除済み要素／mobile scroll guard、route helper の正規化・history guard は変更していない。state／I/O failure の新規モデルや account／node の global apply は対象外。既存 caller の API 呼出し・guard・引数はこの差分で変更しない。

## AC／INVAR と検証

| 条件 | 実装・証跡 |
| --- | --- |
| AC-1 | 製品変更前の unit 1 FAIL／browser 1 FAIL、上記 event sequence と画像・JSON。過去 CI の未観測部分を区別 |
| AC-2 | `useSyncRoute` の実 hash 比較。A→B→A、同じ pending target の再要求、history に追い越された push 後の正当な replace の unit tests |
| AC-3 | 元 offscreen test、rapid の同時点確認、`pointer selection and history preserve joined Live and hosted Metaverse sessions`。Live を作成・Joinし、切替・back・forward 後も Leave 可能／viewer 1。Live card、Metaverse stage／canvas は同じ DOM 要素を維持 |
| INVAR-1 | URL builder／override／push・replace は不変。active と visible・audio focus の分離、scope／draft／history／gesture／focus を既存 suite と native で確認 |
| INVAR-2 | 新規 sleep／skip／retry／assertion 緩和なし。元 test を保持。新規テストの表示語誤記と型・lint 指摘は修正して検証済み |

| command／確認 | 結果 |
| --- | --- |
| targeted Vitest: Canvas、runtime、route synchronization、syncRoute、route projection、routing hook の6ファイル | 55 PASS |
| targeted Playwright: column-immersive、hash-routing、column-scope | 16 PASS／新規session test 1 FAIL（`Viewers` の誤記）。実表記 `viewers` に修正し同 test 1 PASS |
| `cargo xtask desktop-ui-check` | PASS。lint、typecheck、Vitest 162 files／1305 tests、Storybook build、browser 139、Windows visual smoke 20 |
| rapid test の最終同時点 assertion 追加後の targeted rerun | 1 PASS。上記 full suite 後の test 強化だけを再検証。最終 head は CI の全 suite でも検証する |
| Windows WebView2 | 下記の通常切替、runtime 停止・復帰、Tab focus を確認 |
| `git diff --check`／`cargo xtask oversized-files` | PASS。大型ファイル baseline の上限変更なし |
| Linux CI の visual 比較／必須 CI | PR head で確認待ち。Windows の visual smoke を画像比較成功として扱わない |

`cargo xtask check`／`cargo xtask test` 全体のローカル再実行は省略し、変更 path の必須 `desktop-ui-check` と最終 head の Fast CI の Rust／Tauri 等の該当 gate を使用する。Rust・CN・network・wire・storage は差分0であり、frontend の成功をそれらの成功とは報告しない。ローカルログは `.codex/plans/issue-943-desktop-ui-check.log`。

## Windows WebView2 確認

900×900／English／dark、WebView2（user agent Edge152）、通常 motion。production mock bundle を隔離した Tauri host に読み、Live＋Metaverse の2 Column fixture で room 作成・Hosting、横スクロールと実 pointer header 選択、復帰後 Tab を操作した。製品 profile／同意記録／実 network は使っていない。

- Live 表示・選択時: hash `/live`、Live active／visible、Metaverse visible=false／suspended=true／scene suspended=true、canvas 1。
- Metaverse 再表示・選択時: hash `/game`、Metaverse active／visible、suspended 解除、Live offscreen／suspended、canvas 1。
- 復帰後 Tab で Metaverse の drag grip に focus ring と tooltip が表示され、選択を維持した。
- [全観測 JSONL](2026-09-09-943-column-activation-evidence/native-observations.jsonl)、[Live](2026-09-09-943-column-activation-evidence/native-live.png)、[Metaverse 復帰](2026-09-09-943-column-activation-evidence/native-metaverse.png)、[keyboard focus](2026-09-09-943-column-activation-evidence/native-keyboard-focus.png)。

確認ホストの最初の起動は common-controls manifest 不足で失敗し、host のリンク設定を修正してから確認した。これは製品の修正・回帰とは別の検証準備上の失敗。記録用 IPC は Tauri の App URL／dev URL で実行した。確認用 example は製品差分から除外した。

screen reader の実発話、Linux WebKitGTK 実機、既知 native fullscreen 停止問題、実ネットワークでの Live／Dome session、GPU frame rate の計測は未確認。今回変更する route の契約を browser／native と既存 tests で確認した範囲から、これらの解消や適合は主張しない。

## 独立監査・merge

[独立監査](2026-09-09-943-independent-audit.md)は `fc532883232233986535dd4f21a4e456b03975fb` に対して **PASS**。3 group 適合、不適合0／未分類0／blocker0。独自unit54件が成功し、基準hookの隔離再実行で A→B→A の不更新と同一pending先への重複pushの2件が失敗することを再確認した。

監査後の追加は本監査記録・作業記録と unit test 冒頭の説明コメントだけで、製品／test の実行コードは変えない。最終 head の delta 確認と必須 CI の結果は [PR #952](https://github.com/KingYoSun/kukuri/pull/952) に記録する。merge commit の対象 tree と監査対象の一致を確認後、Issue の現在判定と Close を更新する。
