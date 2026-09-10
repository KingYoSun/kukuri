# #957 フィードバック送信不可時の理由と設定導線

## 現在判定

- In progress（実装・local検証完了、PR/CI待ち） / リスク区分 B / Scope revision `957-plan-v1 / 2026-09-10`
- 基準commit: `035eadf6656ef6e6931971922444d2fc3e6e922f`
- 2026-09-10にユーザーが計画、実装、commit、PR、CI成功後のmergeを承認。
- 承認済みAC / INVARとinventoryの追跡先は本書。`.codex/plans`の元計画はローカル作業用。
- 仕様は[ADR 0039](../adr/0039-tester-feedback-intake.md)、UI契約は[DESIGN](../../DESIGN.md)、工程は[Issue lifecycle](../runbooks/issue-lifecycle.md)。

## 目的・変更境界

フィードバックを送れない理由と、次に設定で確認する操作を利用者へ示す。既存の設定CTAを活かし、frontendの説明とfocus遷移を変更する。ノード側の受付有効化、中央受付、認証・同意・通信・保存契約、共有適格判定の変更は対象外。

説明専用の`testerFeedbackAvailability`は取得済みstateから理由を返す。送信先は従来の`eligibleTesterFeedbackNodes`で決まり、説明のためのAPI・polling・同意・認証・再送信は追加しない。各ノードの公開名を示し、名前がない場合は既存label helperでURLに退避する。

## 固定AC / INVAR


| ID | 判定可能な条件 |
| --- | --- |
| AC-1 | 未設定、取得中/状態不明、取得失敗/接続不調、認証・同意待ち、受付非対応を判別できる説明がある。検索・同意だけで受付機能まで有効になると誤解させない。 |
| AC-2 | 空状態からkeyboard/pointerで既存のコミュニティノード設定へ移動できる。Dialogの重なり、focus消失、無関係なsectionへの移動がない。 |
| AC-3 | 利用者の操作（設定追加・既存の認証/規約確認・接続状態確認）と、ノード運営者が提供する受付機能を区別する。非対応時は対応ノードの設定または運営者への確認が必要と説明し、存在しない利用者向け有効化スイッチを案内しない。 |
| AC-4 | 設定を変更して再度開くと最新stateに基づく案内/適格一覧になる。複数ノードの一部だけが対応している場合は送信可能なノードを利用でき、不適格ノードを送信先へ加えない。 |
| AC-5 | 日本語・英語・中国語、dark/light、1280×800と390×844で理由とCTAが読めて操作できる。200% zoom相当のreflowと設定往復のfocusを確認する。 |
| INVAR-1 | `eligibleTesterFeedbackNodes` と共有 `eligibleCommunityNodes` の適格条件を維持する。新しい説明用の判定を送信許可の根拠にしない。 |
| INVAR-2 | Dialog表示・設定CTA・状態更新ではフィードバックを送信しない。送信は既存の明示操作と既存guardを通す。本文3項目、2000コードポイント上限、version/OSのruntime付与、既存error/success契約を維持する。 |
| INVAR-3 | 新しい認証・同意・retry・node設定の自動mutationを加えない。workspaceのscope/投稿draft/scrollを保つ。フィードバックの開閉と一覧更新時の本文扱いを上記の現行挙動に合わせ、閉じる際の扱いを利用者へ明示する。 |


## 固定surface inventory


| ID | 入口・trigger | helper / 対象 | 読み書き・副作用とguard | 遷移 / 検証 |
| --- | --- | --- | --- | --- |
| INV-1 | `onOpenTesterFeedback` | `DesktopShellPage` → `DesktopShellTesterFeedbackDialog` → 適格helper + 新説明関数 → `TesterFeedbackDialog` | store読取とDialogのlocal stateのみ。INVAR-1/2 | TR-1〜4、component/判定test |
| INV-2 | config/status/manifestの更新、再接続後・起動後の取得 | 既存connectivity slice / store購読 → 同じ判定 | 取得済み情報から再計算。新規I/Oなし。INVAR-1/3 | TR-2〜4/6、shell test |
| INV-3 | 空状態の設定CTA | feedback専用callback → `handleOpenCommunityNodeSettings` → `DesktopShellSettingsDrawer` | Dialog closeと既存routing。共有設定callbackの他callerの挙動は維持。自動同意/設定保存なし | TR-5、shell/Playwright |
| INV-4 | 送信、失敗後の再送信、送信中の一覧更新 | `TesterFeedbackDialog.submit` → `api.submitCommunityNodeTesterFeedback` | 既存適格判定・入力guard・request version制御を維持。変更は案内に限定 | TR-6/7、既存送信testと回帰test |

shared helperの全callerはCodeGraphと参照検索で確認する。予定するinventory差分はINV-1/2の表示判定とINV-3の案内改善のみ。IPC/HTTP登録点、共有guard、新規sinkは増減させない。

## 状態遷移と証跡

| ID | 事前状態 → 操作 | 期待結果 | 許可I/O / 禁止副作用 | test・証跡 |
| --- | --- | --- | --- | --- |
| TR-1 | 設定取得成功・0件 → 開く | 設定追加の案内とCTA、送信不可 | 既存取得のみ / 送信・設定自動保存なし | 判定・Dialog test |
| TR-2 | 初期取得中、欠落、取得失敗、再接続/期限付きretry → 開く・state更新 | 確認中/失敗/接続状態を区別。非対応と断定しない | 既存取得のみ / 追加polling・retry・送信なし | 判定test、Story |
| TR-3 | 未認証、未同意、撤回/規約更新待ち → 開く | 該当ノードの設定/認証/規約確認を案内 | 表示のみ / 自動認証・同意なし | 判定・Dialog test |
| TR-4 | 検索可能だが受付非対応、または複数ノードの混在 → 開く | 受付の独立性を説明。適格ノードがあれば通常送信先へ | 表示のみ / 別ノードへの無断送信なし | 修正前失敗test、混在fixture |
| TR-5 | 理由表示 → CTA → 設定を閉じる | 指定sectionへ到達、単一overlay、focus/閲覧文脈保持 | 既存設定表示の読取 / 送信・自動mutationなし | shell test、Playwright、Ubuntu GUI |
| TR-6 | 設定で状態を修正 → 再度開く、または開いたまま適格性喪失 | 最新stateを反映し、失格ノードに送信できない。本文の開閉/一覧更新契約を維持 | 既存の明示設定操作のみ / 自動送信なし | shell/再render test |
| TR-7 | 適格ノード → 明示送信 → 成功/401/403/404/通信失敗 | 既存成功・error表示と入力制約を維持、状態確認だけで再送信しない | 明示送信のみ / 不適格時のsubmit呼出し0 | 既存送信test、必要なerror回帰test |


## 変更前の証拠

- `DesktopShellPage.testerFeedback.test.tsx`の検索専用ノードfixtureで、現行コードに「Search Only」と受付非対応の具体的説明が表示されない失敗を確認（2026-09-10、1 failed）。
- Ubuntu 24.04.5 LTS / Wayland / WebKitGTK / 1280×800 / 日本語darkで、基準commitのTauri frontendをmock APIと検証専用profileで起動。空状態CTAは見えて設定にも移動できたが、受付非対応・次の設定手順の説明はなかった。Debian 13の報告版でCTAが見当たらなかった現象とは区別する。
- Playwrightで設定遷移後のfocusが設定内にない失敗を確認。AC-2 / TR-5のExisting-gapとして、feedbackのclose完了と設定のfocus移譲を修正する。

## 実装・検証の証跡

### AC / INVARとの対応

| 条件 | 実装 / 証拠 |
| --- | --- |
| AC-1 / AC-3 | `testerFeedbackAvailability.ts`とそのtestの未設定・取得中/失敗・同意・接続・受付非対応・混在fixture。`TesterFeedbackAvailabilityNotice.tsx`と3 localeの説明。更新失敗時は前回の送信先を維持し「現在未確認」と明示。 |
| AC-2 | `TesterFeedbackDialog`はclose完了後に設定callbackを呼ぶ。`DesktopShellTesterFeedbackDialog`はCSS visibilityのtransition完了後に設定の閉じるボタンへfocusを移す。固定時間のsleepや共有設定guardの変更はない。shell test・Playwright・両OSのEnter操作で確認。 |
| AC-4 | 判定testの混在ノード、Dialog testの適格性喪失/復帰、Playwrightの設定refresh→再open。追加した17 Storyはcached failureを含む。 |
| AC-5 | Playwrightの3 locale×2 theme×2 viewport（1280×800/390×844）12件、200% zoom/reflow 1件。2枚のLinux視覚baseline。Ubuntu/Windowsの日本語dark実機。 |
| INVAR-1 | `communityIndex.ts`の共有helper・送信条件は差分なし。説明projectionは送信許可に使わず、従来のhelperの結果をselectorへ渡す。 |
| INVAR-2 | Dialog testの送信payload/上限/成功/401/403/404/通信失敗、送信不可時と表示/設定遷移時のAPI呼出し0。shell/Playwrightで認証・同意・保存・送信の自動呼出しなしを確認。 |
| INVAR-3 | Dialog testの一覧更新時draft維持/再open時初期化と喪失予告。既存workspace/routingの全体testと設定往復を確認。新しいpolling、保存形式、復帰時の自動送信はない。 |

### 実行結果

- Windows: `cargo xtask check` 成功。最終差分の`pnpm typecheck` / `pnpm lint`成功。
- Windows: 対象Vitest 3ファイル **25件成功**。`testerFeedbackAvailability.test.ts`、`TesterFeedbackDialog.test.tsx`、`DesktopShellPage.testerFeedback.test.tsx`。
- Windows: `pnpm test:e2e:browser tester-feedback-recovery.spec.ts --workers=2` **14件成功**。設定から戻った後の再表示と未送信も確認。
- Ubuntu: `cargo xtask desktop-ui-check` 成功（Vitest **167ファイル/1341件**、Storybook build、browser **197件**、比較skipのvisual smoke **20件**）。その後のcached状態の案内追加は上記targeted test・lint/typecheckと最終PRのCIで補完。
- Ubuntu: 新規の視覚test 2件だけを`CI=1 ... test:e2e:visual --grep "feedback unavailable" --update-snapshots`で生成。画像を目視確認後、`CI=1 cargo xtask desktop-visual-test` **22件比較成功**。既存baselineは更新していない。
- Storybook: `@storybook/addon-a11y`と同じaxe-core 4.13.0を使いDialog内の**17状態**を検査し、検出違反0。全CTAへkeyboard focus可能。一部の画面外要素等の`color-contrast`はincompleteで、全面的なAccessibility適合の主張はしない。既存のNotice/Button/文字色のtokenは変更なし、今回の理由とCTAは実描画でも確認。
- `oversized-files` 成功。途中で`DesktopShellPage.tsx`が1000行を超えたため、今回の機能の配線を専用page componentへ配置し、既存画面ファイルは基準より14行減。baseline上限の増加なし。
- `git diff --check` / `asset-check` 成功。
- Windows全体`cargo xtask test`: **成功**。Rust **902件**＋harness **22件**、doctest、frontend **167ファイル/1342件**が成功。既定のslow test 4件skipは変更していない。

### 実機と画像

Ubuntuはユーザー指定の`ssh local2` / `~/kukuri`、Ubuntu 24.04.5 LTS、Wayland、WebKitGTK 2.52.6、Noto CJKを使用。WindowsはWebView2を使用。Tauri window設定1280×800、日本語dark、同じ検索専用ノードfixtureで確認した。nativeの装飾を含むwindow寸法と、browser testのviewport寸法は区別する。

検証専用Vite設定で**本番のApp / shell / Dialog**へ`createDesktopMockApi`を渡した。nativeの`KUKURI_APP_DATA_DIR`は検証専用で、実ノードへのfeedback送信や運営設定変更はしていない。これはTauri/WebView上の表示と入力の検証であり、実ノードの受付を確認した証拠ではない。Linuxへ反映した変更pathのSHA-256一致を確認した。

- [Ubuntu 変更前](assets/issue-957/ubuntu-before.png)
- [Ubuntu 変更後](assets/issue-957/ubuntu-after.png)
- [Windows 変更後](assets/issue-957/windows-after.png)
- 視覚baseline: `apps/desktop/tests/playwright/__screenshots__/visual.spec.ts/feedback-unavailable-en-dark.png` / `feedback-unavailable-ja-light.png`

両OSで「フィードバック→設定→focus先でEnter→設定を閉じる」を確認。WindowsではControl Centerへ戻るfocus表示も確認。Ubuntuの変更前は既存CTAが見え、移動もできたため、報告された「導線が見当たらない」を再現済みと扱わない。

### 変更分類・限界

- Existing-gap: 理由の説明不足、設定表示のtransition中のfocus拒否、古い適格一覧が残る取得失敗の説明不足。固定AC-1/2/4の範囲で修正。
- 共有guard・認証/同意・外部送信・IPCは無変更のためBを維持。親Issue/Reopen案件でもなく、独立監査の必須条件には該当しない。差分reviewと固定AC証跡を確認する。
- 新たなネットワーク処理はなく、設定済みノードに対する純粋な表示変換のみ。重い一覧や動画等の性能変更は対象外。
- Debian 13 / 報告版f2cdb5cの実機、実ノードの運営設定、screen reader全文読み上げ、touch端末は未確認。scopeにない新規受付経路・自動有効化は追加していない。
- localで別の`cargo xtask oversized-files`を全体test中に起動した際、Windowsが実行中`xtask.exe`の置換を拒否した。検査自体は既存buildの`target/debug/xtask.exe oversized-files`で成功。testや上限の弱体化はしていない。
