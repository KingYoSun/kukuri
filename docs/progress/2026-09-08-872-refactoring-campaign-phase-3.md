# Issue #872 Phase 3: 個別実装・監査・mergeの記録

- 親Scope revision `2026-09-04-issue-872-refactoring-campaign-v1`を維持する。
- ユーザー承認範囲は#919〜#928の各実装、必須CI、独立監査後のmerge。各子の固定scopeを広げず1 PR=1意図で進めた。
- 現在の完了数: **10/10**。以下は個別実行の追跡記録であり、親Phase 4の完了監査・Close・#873用完了baselineではない。
- [Phase 0〜2](2026-09-08-872-refactoring-campaign-phase-2.md)はPR#938、`271b494c`で保存した。監査開始JSONの`completed_campaign_baseline=false`を維持し、当時のfail/未実施記録を遡及変更しない。

| Issue | PR | 構造上の成果・役割 | 現在判定 | merge SHA / 未merge head |
| --- | --- | --- | --- | --- |
| [#919](https://github.com/KingYoSun/kukuri/issues/919) | [#929](https://github.com/KingYoSun/kukuri/pull/929) | 通知取得contract | Complete | `0410261bb8091829db6b5cd8f1feb66791aaf327` |
| [#920](https://github.com/KingYoSun/kukuri/issues/920) | [#933](https://github.com/KingYoSun/kukuri/pull/933) | 通知取得owner2→1 | Complete | `4e833af8cbdb7263a75ad17f54ee64be6e263471` |
| [#921](https://github.com/KingYoSun/kukuri/issues/921) | [#932](https://github.com/KingYoSun/kukuri/pull/932) | CN Dome transfer contract | Complete | `d5c7769a9c73c76ccfeabb8b0b0453d495b1e1b6` |
| [#922](https://github.com/KingYoSun/kukuri/issues/922) | [#936](https://github.com/KingYoSun/kukuri/pull/936) | CN transfer副作用site2→1 | Complete | `f90d28fb5c5b0f87fd30233250ba786c3938eba0` |
| [#923](https://github.com/KingYoSun/kukuri/issues/923) | [#934](https://github.com/KingYoSun/kukuri/pull/934) | Dome非同期transition contract | Complete | `d9ea79be4fe03e3d97032b53bef2747760652908` |
| [#924](https://github.com/KingYoSun/kukuri/issues/924) | [#937](https://github.com/KingYoSun/kukuri/pull/937) | attempt直接更新9→0、専用owner9 | Complete | `3971fba25a8d6034e16623aefc5a5b7b51d02bd6` |
| [#925](https://github.com/KingYoSun/kukuri/issues/925) | [#935](https://github.com/KingYoSun/kukuri/pull/935) | source解決の禁止I/O contract | Complete | `7c9cba850d572c8d358d5f529798a89c3c84eb06` |
| [#926](https://github.com/KingYoSun/kukuri/issues/926) | [#939](https://github.com/KingYoSun/kukuri/pull/939) | source到達service依存6→2 | Complete | `5c61680c82607ae96ebcce78496e62e2f457ecb5` |
| [#927](https://github.com/KingYoSun/kukuri/issues/927) | [#931](https://github.com/KingYoSun/kukuri/pull/931) | 未同期Dome manifestのtyped欠落処理 | Complete | `2229fefaa479064bf8080bc55e2984acf025fc66` |
| [#928](https://github.com/KingYoSun/kukuri/issues/928) | [#930](https://github.com/KingYoSun/kukuri/pull/930) | capability規範を実装へ同期 | Complete | `36e3732edfb68e0637a60e7badf4c778d0530e4d` |

## 検証と監査

各子Issueの実行状況とPR commentsに、固定headの独立監査、AC/INVAR/INV/TR、必須CI、merge対象treeの比較を保持する。MERGEDだけでCompleteとはしない。最初の監査FAIL/CI failureも消さず、補強・修正後のdeltaと最終headを分けて記録した。

- 通知: 先行13 testsを追加し対象29 tests PASS。抽出前後に同29を維持。独立監査、正しいrootのdesktop-ui-check、最終Fast全9、Linux visual比較を確認。
- CN transfer: 既存6+新5の11 tests、AppService2 tests。成功view、4失敗段階、guard7条件、same operation/no-op、assignment後のpolicy更新を維持。CIで検出したlock分類5siteを登録し、完全一致検査は保持。
- Dome attempt: 既存14→先行24 tests。抽出時にはentry/model/panel含む57 testsを前後同条件で実行。全UI gateは152 files1209 tests、Storybook、browser64、Windows visual smoke14 PASS。Linux pixel比較は最終CIへ対応。
- CN source: 新6 contractと既存3suiteで28 tests、worker/queryを含む抽出前後6suiteで39 tests。独立再実行39 PASS、全targets Clippy -D warnings PASS。未検証sourceのfetch/provider/upsert0と許可deindexを区別。実DB/stackはcn-check/cn-test CIへ対応。
- Metaverse実iroh: 修正前に同じtestのmanifest欠落failを再現。欠落だけをtyped outcomeに分け、bad reference/signature等を抑止しない。既存testは変更せず、slow feature全199 tests PASSと独立delta監査を確認。
- capability文書: Availableへ移行済みの提供状態と、各配備のreadiness/公開条件を分けて同期。製品availabilityや過去ADRの判断は変更しない。

ローカルの対象test成功を全suite成功に読み替えない。全source PRは必要なFast9jobと適用されるpackage/image checksを確認してmergeした。docs-only PRではpath filterで製品CIが起動しないことと、構文/参照/diff check/GitGuardianの結果を区別する。

## Windows/WebViewと制約

[#924の実機比較記録](../ui-reviews/2026-09-08-issue-924-transition-owner.md)に条件、画像、入力/退出/画面外縮退と未確認範囲を保存する。実Tauri/WebViewを使い、mock APIの成功を実network/authorityの成功にしない。方向キー長押しの移動量と全画面中の移動は未確認として保持する。

全画面開始時のvisible/suspendedフラグは変更前の元sessionでも同条件で再現した既存状態で、今回の抽出で新たに発生したRegressionではない。通常表示のinput ownership/姿勢入力/退出・描画停止復帰は前後一致する。fullscreen lifecycleの修正はこの抽出へ追加しない。

## 引き継ぎ境界

各refactorは対応contractを保持したまま単独revert可能。schema/data migration・依存追加なし。候補の延期/却下を実装へ戻さず、fix/docsの別種別も分離した。

親#872はOpenを維持する。親固有の完了監査、各AC/INVARへの最終対応、完了時baselineの再測定・#873への昇格はPhase 4の別工程であり、この記録だけで完了扱いにしない。
