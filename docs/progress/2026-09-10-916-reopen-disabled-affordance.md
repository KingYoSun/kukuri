# #916 再オープン対応: 年齢未確認の同意ボタン

- Issue: [#916](https://github.com/KingYoSun/kukuri/issues/916)
- 状態: 実装・検証中。配布候補の実機確認、独立監査、必須CIの完了後に判定する。
- Scope revision: `916-r1`を維持。2026-09-10に再開計画と実装・commit・PR・CI成功後のmergeを承認済み。
- 基準commit: `f74243c3c2001d5c257e67ca5c77d9b01de0091f`
- リスク区分: C。UI分類: 不具合修正。

固定AC-1〜5 / INVAR-1〜4、inventory、状態遷移は[初回作業記録](2026-09-09-916-consent-gate-affordance.md)を継承する。過去のPASSは当時の対象条件に対する記録であり、再オープン後の完了根拠にはしない。

## 問題と変更結果

未チェック時のボタンにも通常の「同意して続行」というaction名があり、明るい実線の輪郭は有効なoutline buttonと区別しづらかった。操作前から理由文は見えるが、ボタン自身が押せるように見えるというAC-2のExisting-gapを扱う。

未チェック時はボタン内に鍵アイコンと「年齢確認が必要」相当の状態名を表示し、中立色の背景・破線の境界・影なしで示す。選択後は従来のprimaryと「同意して続行」、pendingは既存の処理中文字に切り替わる。native disabled、年齢guard、同意IPC、保存・runtime・復元の条件は変えない。クリック時のtoastを追加するためにdisabledを解除しない。

対象は `ConsentGateView`、gate専用CSS、3localeのgateキーと対応tests。共有Button、global token、法務文書の本文・版、CN同意・CLI・backupの仕様変更は対象外。

## 修正前の再現と見逃しの原因

[再測定報告](https://github.com/KingYoSun/kukuri/issues/916#issuecomment-5606278898)は `0.2.1-preview.1` / Linux / ja / clean profile。[添付画像](https://github.com/KingYoSun/kukuri/issues/916#issuecomment-5606306158)を直接確認したところ、未選択のボタンは灰青色・明るい輪郭だった。「オレンジ寄り」という報告文をCSSの未適用と同一視せず、公開AppImageでも確認した。

- 公開artifact: `kukuri_0.2.1_amd64.AppImage`、Release `v0.2.1-preview.1`。
- 公開source: `f2cdb5cacf02218b34291a35754b55a15c2f564e`。
- SHA-256: `9f4990ae0913a9682a23d28779ea0791f7e53c2c8eab1f64ec7c33e52995df3b`。GitHub Release asset digestと取得fileが一致。
- 実行環境: Ubuntu 24.04.5 LTS / x86_64、GNOME Wayland session中のXWaylandアプリ。RDPで表示と実pointer入力を確認。X window client領域1280×800を`xwininfo`で確認した。
- 新しい専用profileで日本語初回起動→無効ボタンclick→年齢選択→解除を確認。未選択は灰青色・実線輪郭・通常action名、選択後はprimaryとなった。起動logはruntimeを同意まで延期し、profileには同意ファイルが作成されなかった。
- `/proc/<pid>/maps`でAppImage内のWebKit/GTKをロードしていることを確認。host WebKit2.52.6をAppImageのWebKit版とは扱わない。

![公開AppImage: 未チェック](../ui-reviews/assets/issue-916-reopen-before-unchecked.png)

![公開AppImage: チェック後](../ui-reviews/assets/issue-916-reopen-before-checked.png)

画像は同じRDP撮影からアプリclient領域だけを切り出したもの。色・文字・描画内容は加工していない。報告者の元の実行fileそのもののhashは得ていないため、上記を同じ公開版の再測定として区別する。

専用disabled CSSの未適用や後続差分によるRegressionは確認されなかった。問題は有効な操作に見える状態表現である。既存browser testはdisabled属性、ARIA、到達性、IPC回数を保護していたが、ボタン自身の状態名と実効背景・影・境界を検査していなかった。旧実機確認も隔離ホストとsynthetic応答で、報告された配布版は対象外だった。

新しいbrowser testをproduction変更前に実行し、ja/darkで `Expected: 年齢確認が必要 / Received: 同意して続行` の失敗を確認した。同じtestの修正後成功と、配布物の同条件before/afterを組み合わせる。disabled属性や単なるRGBの差だけで識別性を判定しない。

## inventoryと境界の対応

既存6group / TR-1〜8を維持し、production IPC・保存sink・別consumerの追加/削除は0。INV-2の表示と試験だけが変わる。

| 条件 / inventory / transition | 実装と確認 |
| --- | --- |
| AC-1/2、INV-2、TR-1〜3 | 理由文とdescriptionは維持。Viewの状態名・鍵アイコン、`base.css`のgate内disabled。新browser testは3locale×2themeで背景・background-image・影・破線・状態名、pointer/Spaceによる選択/解除、reload、同意IPC0を確認 |
| AC-3/5、INV-2、TR-4/5 | 既存layout matrixと390×541の拒否→選択→保存失敗→解除を維持。文書末尾、focus、狭幅/低高さ/zoomを確認 |
| AC-4/5、INVAR-1〜3、INV-1/3/4、TR-1〜7 | `App::handleAccept`の年齢/`acceptInFlight` guardとpayload/status/restore分岐は不変。AppのIPC回数・引数・error/retry/renewed/restore、locale既存test |
| INVAR-1〜3、INV-5、TR-1/6/7 | SINK-1の保存、SINK-2の初期化、SINK-3のrestore activationはRust production不変。保存不変・host未構築・Tauri invoke禁止・restore状態表を既存testとsourceで確認 |
| INVAR-4、INV-6、TR-8 | gate ancestor外の共有Button/StartupStatusScreen/AboutPanel/CNは不変。CSSはpending時のgate内拒否ボタンにも適用される。既存画面のselector・diff・test/visualを確認 |

CodeGraphと登録点/参照の確認による独立予備監査で、保存sinkの逆引きを補足した。

- `record_app_consents`のcallerはTauri commandとCLI `ClientSession::accept_consents`。
- `save_app_consent_store`にはCLI `main.rs::accept_consents`の直接callerもある。`--accept-documents` / `--age-confirmed` / 非空languageとProfileLeaseのguardがあり、INV-6の不変consumerに分類する。
- `reset_app_consent_at_path`経由の復元リセットをINV-5 / TR-7に分類する。保存pathは`db_path.with_extension("app-consent.json")`。
- SINK-2の`build_desktop_state`→`ClientHost::start_if_consented`は構築前に再検査する。startup/受諾/restore rollbackのcallerは不変。
- SINK-3のactivationはstartup/受諾と共有CLI orchestrator、frontend applyはAppのstartup ready/受諾readyの2箇所。成功前のapplyを増やしていない。
- 共有Buttonの48 import元に新しいconsumerはなく、今回の表示はgate ancestor内に限られる。locale保存は年齢/同意保存と別の既存副作用として分類する。

未分類0の分類案を作成済み。実行結果と最終headが確定するまでは適合6や監査PASSとは判定しない。

## 検証結果

| 検証 | 現在の結果 |
| --- | --- |
| 修正前browser | ja/darkの状態名assertが失敗。再現証拠を修正前に保存 |
| 同意browser | 新6件＋既存26件、32 passed。production Vite build＋mock IPC。配布アプリの確認とは区別 |
| targeted Vitest | App / design-contract / css-vars: 3 files、17 passed |
| `cargo xtask doctor` | 成功 |
| 日常integration / desktop-ui-check | 実行中または未実行。完了後に結果を更新 |
| Linux visual / Storybook a11y / 配布候補実機 / Windows実機 | 未完了。baseline更新と実機証跡を同じ対象差分へ対応付ける |
| 最終PR head独立監査 / 必須CI / merge後照合 | 未完了。以前のPASSを代用しない |

Windowsのintegration検証には実在を確認したSDK `10.0.22621.0` のucrt/um x64 directoryをprocessのLIBに追加する。製品のbuild設定は変更しない。LinuxローカルでもCI未設定のvisualは比較skipであるため、Linux/Chromiumの比較を別に実行する。

## 完了条件

固定AC/INVARの証跡、Ubuntu24の通常配布候補のsource/hashと実描画、必要なWindows/入力/zoom確認、独立監査PASSと必須CIを揃える。PRは`Refs #916`とし、merge後に対象tree/surfaceを照合してIssue現在判定を更新する。具体的blocker0・未分類0になるまでCloseしない。未確認条件を成功へ読み替えず、変更のない範囲の全監査を繰り返さない。
