# #959 接続診断の状態説明と回復案内

- 判定: Audit pending（コードdelta確認済み、最終CI待ち）
- Scope revision: `959-plan-v1 / 2026-09-11`（ユーザー承認済み）
- 基準commit: `70aa3e9e463f24889e1db1d15717ff26c9cd682d`
- リスク区分: B。表示と既存状態取得・設定内遷移。network / IPC / 永続化 / 認証・同意guardは変更しない。
- 実装、コミット、PR、CI成功後のマージまで承認済み。独立確認は固定headで別工程に置く。
- 目的: 候補と実接続、接続と配送、取得失敗と0件を分け、状態に合った確認操作へ進める。
- 対象外: 初回join timeoutそのものの解消、強制再接続、新しい通信経路、自動同意・認証・購読、診断の外部送信。
- 正本: [Issue運用手順](../runbooks/issue-lifecycle.md)、[ADR 0014](../adr/0014-uiux-dev-flow.md)、[DESIGN](../../DESIGN.md)、[UI実装配置](../architecture/desktop-ui-implementation.md)。

## 修正前の再現

- 2026-09-11、基準commitにtestだけを追加しVitest実行。過去エラーでLiveが隠れる表示、peer0でrelay-supported表示、古いsync readが新event snapshotを巻き戻す3件が失敗、既存CN取得test2件は成功。最初の表示testは期待語の大小文字も誤っていたため、現行localeの`connected`へ修正した（修正前の実値は`error`であり不具合の再現は有効）。
- Ubuntu 24.04.5 / WebKitGTK 2.52.6、リモートデスクトップのTauriで製品Appに検証用mockを供給。peer0、configured candidate、docs assist、初回join timeoutを固定して接続画面を開いた。日本語要約と英語原文が主面で反復し、候補と実績の説明、次アクションがないことを確認。
- [Ubuntu変更前](assets/issue-959/ubuntu-before.jpg)。window内1280×800相当、dark / ja。実ネットワークでtimeoutを再現した証拠ではない。
- #915の翻訳を継承。CodeGraph取得の一部結果には古い行が含まれていたため現在の実ファイルも照合した。接続panelは現在常に`ready`で共有errorを表示していた。今回sync専用の取得状態で分離する。

## 固定する受入条件・不変条件

| ID | 条件 | 作業 / 証跡 |
| --- | --- | --- |
| AC-1 | peer0・設定候補ありで、未接続と候補の存在を矛盾なく説明できる。Control Center・接続・topic・discoveryが同じ意味で状態を示す。 | T1、T3、T5 / TR-2・4・5・8 |
| AC-2 | 初回join timeout・未知エラーが日本語で要約され、実行可能な次の確認操作がある。raw error / 内部コードは主説明と分かれ、詳細から参照できる。 | T3、T5、T6 / TR-2・4 |
| AC-3 | topicの診断欠落を取得待ち・未購読・停止中に根拠付きで分け、取得失敗を0件や未使用にしない。 | T2、T3、T5 / TR-1・3 |
| AC-4 | 「診断を更新」と既存設定へのCTAが実動し、更新中・失敗・再試行・回復が観測できる。表示更新や遷移だけで接続設定・認証・同意・購読を変更しない。 | T2、T4、T5 / TR-3・6・7 |
| AC-5 | `Live` / `DurableReady` / `DurableRecovering` / `Offline`、3経路、過去エラーの組合せで、現在の接続・配送能力を誇張しない。 | T1、T3、T5 / TR-2・4・5・8 |
| AC-6 | ja / en / zh-CN、dark / light、通常／Developer mode、広幅・狭幅で意味と操作を保持する。Ubuntu24 TauriとWindows WebView2で対象導線を確認する。 | T5、T6 / TR-6・7・8 |
| INVAR-1 | 通信優先度と3経路の定義、wire値、transport / docs / blobの状態判定を変えない。 | T1、T3、T7 |
| INVAR-2 | 既存認証・同意・retry deadline・seedのenv lock・gossip停止を維持する。診断CTAに新しい通信mutationを作らない。 | T2、T4、T5、T7 |
| INVAR-3 | ticket / seed / CNの既存操作、draft・scroll・workspace・戻り先・focusを壊さない。通常モードに技術詳細を出さない。 | T4、T5、T6 |
| INVAR-4 | 翻訳は表示専用。raw error / ID / protocol値を保存・送信前に翻訳せず、#915の既存callerを回帰させない。 | T3、T5、T7 |
| INVAR-5 | 取得失敗時に以前のsnapshotを失わず、古いresponseが新しいruntime eventや別操作で更新された状態を巻き戻さない。再起動時は現状を取り直す。 | T2、T5 / TR-1・3・7 |

## 固定surface inventory

表示変更が主であり新規sensitive sinkは設けない。既存取得・設定遷移のcallerを両方向に確認する。次の群の全memberは、対象symbolのCodeGraph callerと`getSyncStatus` / `setSyncStatus` / `sync_status_changed` / `syncStatusBadgeLabel` / `topicConnectionLabel` / `diagnosticErrorLabel`の参照検索からT1で再生成し、追加入口があれば対応する行へ実名を記録する。

| ID | 入口・trigger | shared helper | 読み書き・副作用 | guard / invariant | 遷移・検証 |
| --- | --- | --- | --- | --- | --- |
| INV-1 | Control Centerのtrigger・状態サマリー・設定入口 | `syncStatusBadgeLabel`、表示用判定、既存`openSettings` | store参照とdrawer / focus更新 | INVAR-1・3。開くだけで接続mutationなし | TR-2・4・6 / shell test・Playwright |
| INV-2 | 接続全体と全tracked topicの詳細 | `useSettingsViewModels`、`topicConnectionLabel`、`ConnectivityPanel` | snapshotの表示 | INVAR-1・4。候補と実績、未取得と0件を区別 | TR-1〜5・8 / view model・panel test |
| INV-3 | discovery診断・原文詳細 | `DiscoveryPanel`、`diagnosticValueLabel` / `diagnosticErrorLabel` | 表示・詳細開閉。seed編集は既存操作のみ | INVAR-1・2・4。env lockと未保存入力を維持 | TR-2・3・6・8 / panel test |
| INV-4 | 起動・既存取得trigger・診断更新・再試行 | `useConnectivityStatusRefresh`、`useDesktopShellData`、connectivity slice | 既存状態取得API、メモリsnapshotと取得状態更新 | INVAR-2・5。CN側の競合保護を保持、同期失敗を隠さない | TR-1・3・7 / refresh hook test |
| INV-5 | runtime event・設定操作後のrefresh | `useRuntimeEventBridge`、`useDesktopShellDataEffects`、全`setSyncStatus` caller | 既存event購読とメモリ更新 | INVAR-5。古い非同期結果と混在node状態を保持 | TR-4・7・8 / event・hook test |
| INV-6 | 診断から既存接続／CN設定への移動 | `DesktopShellSettingsDrawer`、既存section / focus helper | drawer内移動。既存の画面取得は許容 | INVAR-2・3。遷移による保存・同意・認証・購読変更なし | TR-6 / shell test・両OS実機 |
| INV-7 | locale切替・Developer mode・共通翻訳helperの他caller | `diagnosticLabels`、CN dependency表示、settings fixtures | 表示変換のみ | INVAR-3・4。他callerの原文参照・翻訳を維持 | TR-8 / 既存i18n・CN表示test |

期待するinventory差分は、INV-4への明示的な診断更新入口、INV-6の案内CTA、INV-2・3の要約／詳細表示の追加。network setter・認証・同意sinkは追加しない。取得statusを増やす場合もIPCへ追加せずUIメモリ内に限定する。

## 状態遷移と再現ケース

| ID | 事前状態 / event sequence | 期待状態 | 許可I/O | 禁止する副作用 | 検証 |
| --- | --- | --- | --- | --- | --- |
| TR-1 | 初期snapshot → 初回取得中 → 成功、または再起動から取得 | 未取得を未使用／接続確定とせず、成功後に各topicの状態へ移行 | 既存状態取得 | 未取得の0件確定、前sessionの状態による成功表示 | slice・refresh・view model test |
| TR-2 | CN ready、peer0、configured / docs-assistあり、initial join timeout | リアルタイム未接続、候補と配送回復の説明、timeout要約・原文詳細・確認先 | 診断更新時の取得 | 候補から接続成功判定、CN同意済みを接続保証とすること | 失敗test→成功、Story、実機前後 |
| TR-3 | 有効snapshot → 更新失敗 → 再試行 → 成功。snapshotなしの失敗も別fixture | 前回値と取得失敗を併記、初回失敗は未確認。更新成功で取得エラーを解除 | 既存状態取得 | 値の破棄、0件／未使用への化け、連打で無制限な要求 | hook・panel・browser test |
| TR-4 | timeout / recovering → peer接続・Live。逆に切断。raw last_errorが残るcaseを含む | 現在の能力と過去のエラーを別表示、Control Centerと詳細が一致 | runtime event / 既存refresh | 古いerrorだけで常時失敗、接続能力の過大表示 | 表示純関数・event test |
| TR-5 | topic診断なし、未購読／購読済み／gossip停止。別topicだけLive | 対象topicごとの説明と既存操作案内 | 状態参照 | 履歴の推定、暗黙購読・gossip再開、全topic一括成功 | table-driven test・Story |
| TR-6 | 案内CTA → 指定設定section → 閉じる。未保存seed / ticket入力あり | focusと編集・閲覧文脈を維持。単一overlay | 既存設定表示の取得 | 保存・認証・同意・seed反映・購読変更 | mutation API spy、Playwright、Ubuntu/Windows |
| TR-7 | 診断更新中 → 新しいruntime event / CN受諾・撤回event → 古いresponse到着 | 最新stateを巻き戻さない。旧失敗を新snapshotへ誤適用しない | 既存並行取得・event | CN競合保護の破壊、古い同期値による復帰 | deferred promiseによるhook / event test |
| TR-8 | 3経路 × 4配送状態の有効な組合せ、discoveryとtopicの差、node混在、locale・mode変更 | 未接続時の経路表示を誤認させず、fallbackを通常の補助経路と区別。全表示がlocaleに追従 | 表示参照 | raw値変更、node一件から全体判定、通常面への内部コード露出 | 有効組合せの表形式test・3locale Story / browser |

直積をすべて増やさず、意味が異なる有効な組合せをT1でfixtureとして固定する。再現可能なUI logicは先に失敗testを実行する。実ネットワークのtimeoutが起きない場合も再現用snapshotで表示の欠陥を検証できるが、実ネットワークで障害再現済みとは記録しない。


## 作業・検証の結果

- T1: callerと3件の失敗test、Ubuntu変更前を記録。
- T2〜T4: sync取得状態、最新snapshot保護、設定・Control Centerの共通説明、回復CTAを実装。別操作の共有errorは同期状態と分けて保持する。
- T5: 状態表・取得競合・操作・翻訳・狭幅を検証。初回失敗→再取得、更新失敗→回復、raw詳細、topic欠落／停止、focusとdraft保持、mutation call0を固定。
- T6: 両OSのmock Tauri、Storybook、browserを確認。visualはLinux公式workflow生成の新規2枚を目視後に取り込み、Linux比較26件成功。既存画像はartifactと同一で変更なし。
- T7: `b5c7d29`の独立監査でControl Center topic一覧の欠落idle/0表示をExisting-gap B1と判定。修正前browser testで再現し、`76ff9fc8`のdelta独立確認はコードPASS、INV 7/適合7/不適合0/未分類0。最終visual/CIの判定は別途記録する。
- T8: [PR #974](https://github.com/KingYoSun/kukuri/pull/974)。最終必須CI成功後にmergeと対象tree一致を確認し、Issueの現在判定を更新する。

### AC / INVARと具体的な証跡

| 条件 | 実装・証跡 |
| --- | --- |
| AC-1 / AC-5 / INVAR-1 | `connectivityGuidance.ts`をControl Centerの全体・topic一覧と設定panelで共有。`connectivityGuidance.test.ts`の3locale×3経路×4配送状態、欠落・停止・未知error、`connectivity-diagnostics.spec.ts`の一覧／設定一致test。 |
| AC-2 / INVAR-4 | `diagnosticLabels.ts::diagnosticErrorSummary`で既存の厳密なtimeout識別を共用。原文関数の他callerは維持。原文詳細の非表示→展開、locale切替、未知errorのtestを維持。 |
| AC-3 / INVAR-5 | `useConnectivityStatusRefresh.test.tsx`で初回失敗、前回snapshot保持、連打抑止、最新eventに対する古い成功／失敗、既存CN per-node競合保護。topic countは不明時null。初回取得失敗のbrowser testで0件確定しないことと再試行を確認。 |
| AC-4 / INVAR-2・3 | 既存read APIとdrawer section切替のみ。browserでticket保持、移動先nav focus、閉じた後のtrigger focus、禁止mutation API call0。既存gossipやseed / CN設定の変更処理は変更しない。 |
| AC-6 | Ubuntu24 / WindowsのTauri上でmock表示・更新・回復導線。browser3locale×2theme・広幅／390px・通常モード200% zoom。Story12状態×3locale×2themeの[axe結果](assets/issue-959/story-a11y.json)は72条件・違反0・要確認0。 |

### 実行結果

| 検証 | 結果・対象範囲 |
| --- | --- |
| Windows `cargo xtask check` | 成功。Rust check、Tauri backend check、frontend lint/typecheck。 |
| Windows `cargo xtask test` | 成功。non-CN 902件成功（既存skip4）、harness22件成功、doctest成功、frontend169 files / 1392件成功。その後のtopic一覧・contrast差分は下記targetedと最終CIで補完。 |
| Ubuntu `cargo xtask desktop-ui-check` | lint/typecheck、Vitest169 files /1392件、Storybook build成功。初回browserでは未転送のnowrap修正と既存詳細開閉testで失敗し、修正・同期後にbrowserを再実行。全体成功と一括記録しない。 |
| Ubuntu `cargo xtask desktop-browser-test` | コードdeltaを含む220件成功。以後追加した初回失敗1本はWindowsでも成功し、最終CIで全体を確認。 |
| 最終targeted Vitest | 状態・view model・topic UI等58件、composer等の差分60件、settings panel・read・翻訳等66件成功（集合には重複があるため合算しない）。 |
| 最終Windows browser | `connectivity-diagnostics.spec.ts`10件成功。#915関連を含む先行targeted15件も成功。 |
| 最終lint / typecheck | 成功。 |
| Storybook / a11y | 最終build成功。最初の72条件でライトテーマの既存metricラベルにcontrast違反24条件。色付き背景上のdtのみ既存foreground色へ変更し、同条件72件を再検査して違反0・要確認0。 |
| Linux visual | [生成workflow](https://github.com/KingYoSun/kukuri/actions/runs/34513333475)はsource `76ff9fc8`で成功。追加した`connection-recovery-{ja-dark,en-light}.png`を目視し取り込み。`CI=1 cargo xtask desktop-visual-test`26件成功。 |
| 独立確認 | 初回71件、delta64件の独立targeted test成功。固定条件を再構築しB1のみ検出・修正確認。自己検証の件数で監査を代替しない。 |
| 大型ファイルgate | 既存`tests/playwright/shell.smoke.spec.ts`が詳細展開の1行追加で999→1000行となりCIでfail。`cargo xtask oversized-files --update-baseline`で生成した当該1件だけを登録。元の長いIDの表示・幅検証を弱めず、局所UI修正に全smoke分割を混ぜないための登録で、他pathの上限は維持。最終gateを再確認する。 |

### 実機確認と未確認の境界

- [Ubuntu変更前](assets/issue-959/ubuntu-before.jpg) / [Ubuntu変更後](assets/issue-959/ubuntu-after.jpg)、[Windows変更後](assets/issue-959/windows-after.jpg)。製品App / shell / panelへ既存mock APIを供給し、テスト用runtime profileで確認。実際の同意・認証・送信は行わない。
- Ubuntuでは「診断を更新」→Tabで設定CTA→Enter→移動先nav focus→Escape→Control Centerのfocus復元を確認。Windowsでは更新・設定移動と表示を確認。
- 実APIを使う通常起動も試みたが、Ubuntu既存profileは利用前の規約・年齢確認待ちでruntime起動が延期された。そこで終了し、同意記録や年齢申告は変更していない。実runtimeの診断面到達・実ネットワーク障害再現は未確認。mock Tauriの表示成功とは区別し、network挙動の変更はない。
- Debian13、screen reader読み上げ、touch端末実測は未確認。200%はbrowserのreflow検証であり、nativeのzoom操作成功とは記録しない。

## 固定callerの確認

- 状態取得: `useDesktopShellData` → `useConnectivityStatusRefresh`。起動、既存interval、loadTopics後のrefresh、`useCommunityNodeRecovery`、新しい診断更新CTA。
- event: `useRuntimeEventBridge` → `useDesktopShellDataEffects::applySyncStatusChange` → `setSyncStatus`。これと取得hookが製品の書込入口。
- 表示: `DesktopShellControlCenter`、`useSettingsViewModels`、topic一覧の`useDesktopShellViewModels`。
- 共通diagnostic helper: 上記view model、`components/settings/communityNodeDependency.ts`、fixtures、対応tests。原文helperは維持し、接続・discoveryの詳細内で使う。
- CTA: `DesktopShellPage`で既存refreshを渡し、`DesktopShellSettingsDrawer`の既存section切替・focus先を再利用。
- mock初期値とstoryは取得状態を明示。runtime初期値は未取得、event / 成功したreadだけで取得済みとする。翻訳や診断表示からAPI mutationは追加しない。
