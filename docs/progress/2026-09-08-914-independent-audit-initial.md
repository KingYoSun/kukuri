# Issue #914 / PR #946 独立監査

- 対象 commit: `bf9df4a8e7feca1a758dbd8fdd848fe0468a3ac1`
- 比較 base: `916f5e0a1c2a81cfe0f88aa463fd77cc1e793c19`
- Scope revision: `2026-09-08-914-plan-v1`
- リスク区分: C
- 担当: 実装担当とは別の監査コンテキスト
- 判定: **FAIL**。blocker 3件。修正後は該当deltaと影響callerを再監査する。
- inventory: 合計7群 / 適合4 / 不適合3 / 未分類0。適合は下記静的確認と局所検証の範囲。実機・全体gateの完了判定を含まない。

## 方法・対象

AGENTS.md、AGENTS.local.md、docs/README.md、dev/issue-lifecycle runbook、DESIGN.mdと承認済み計画のAC-1〜6 / INVAR-1〜5から再構築した。実装担当のprogressの結論を根拠にはしなかった。CodeGraph exploreを先行し、JSX callerとIPC登録をrg・現物読取で補完した。CodeGraphはCommunityNodePanelとDomeHostingPanelのJSX callerを最初のcaller一覧に含めなかったため、一覧をそのまま完全性の証拠にしていない。

固定headの製品差分にはRust変更なし。親担当の一時WebView fixture、追加中の画像・文書は製品コード監査の対象外とした。コード、tests、commit、PR操作は変更していない。

## Blocker

### F1: 設定等の規約取得は古い応答で新しい表示を上書きし、提示言語と保存言語が食い違う

- 分類: Existing-gap、AC-5 / INVAR-4 / TR-10 / INV-2,7。
- 根拠: `apps/desktop/src/shell/actions/profileTopicChannel.ts:704` の `handleFetchCommunityNodeConsents` は要求時の言語でfetchするが、応答をbaseUrlだけの共有entryに無条件保存する。`CommunityNodePanel.tsx:157` の `openConsentDialog` も要求世代を持たず、`:193-203` は表示文書だけを渡す。actionの`:733` は受諾時の現在言語を既定引数にする。DomeHostingPanelの`:113` / `:178-190` も同じ共有catalog/actionを使う。
- 到達sequence: 設定でAの日本語規約取得を開始→閉じる→言語を英語へ変更→Aを再開→新しい英語応答が先に完了→古い日本語応答が完了→受諾。modalのloadedはAに一致したままなので日本語文書を表示しながら、APIへ英語を送る。新しいhookの世代管理はこの既存callerには適用されない。
- 影響: `runtime/community_node_api.rs:384-397` は提示言語を暗号化local consent recordへ記録するため、実際に提示した言語と異なる同意証跡を保存する。snapshotのpreflightが存在しても表示言語の不一致を訂正しない。
- 局所再現: 実ファイルからfetch/accept handler本文を切り出してTypeScript transpileし、二つのdeferred policy応答を新→旧の順に解決した。出力は `accepted language: en displayed language: ja`。UI全体の自動再現testは本監査では追加していない。
- 必要な修正: 共有catalogのNode・言語・世代を対応付け、設定/Dome側も提示文書と言語を受諾まで固定する。閉じた要求と旧世代応答を無効化する。

### F2: 設定からの受諾失敗が規約modal内に表示されない

- 分類: Existing-gap、AC-5 / TR-5 / INV-2,7。
- 根拠: `CommunityNodePanel.tsx:202-209` は受諾rejectをcatchしてreturnし、`CommunityNodeConsentDialog`への`:535-548` のpropsにerrorがない。actionは`communityNodeError`を設定するが、それは背面Panelの`:249`付近に表示され、開いているmodal内には到達しない。DomeHostingPanelも受諾失敗を背面のerrorへ保存する。
- 到達sequence: 設定→規約取得成功→受諾→保存失敗またはsnapshot競合のreject。busyが解除されるだけでmodalには失敗・再確認・再試行理由が現れない。
- 影響: 利用者は受諾が完了しない理由や文書再取得が必要なことをその場で確認できず、同じ受諾を繰り返す。AC-5のmodal内回復の固定期待を満たさない。
- 必要な修正: 受諾errorを開いている規約modalへ表示し、再取得・再受諾の導線を結ぶ。保存済み同意は削除しない。

### F3: 共通受諾actionの遅延応答が先に届いた最新eventを巻き戻す

- 分類: Existing-gap、AC-1,6 / TR-4,10 / INV-2,3。
- 根拠: `profileTopicChannel.ts:736-743` は受諾API戻り値を無条件upsertする。`presentation.ts:553-566` は`next`のauth/session等を採用するだけで要求世代を比較しない。`useConnectivityStatusRefresh.ts:21-44`にはbaseline保護が追加されたが、共通受諾actionにはない。
- 到達sequence: 受諾の戻り値snapshotがretrying/auth=falseとして確定→配送が遅延する間にruntime ready eventがUIへ到着→遅延した受諾応答が到着。storeの選択整合は最新eventではなく古い戻り値を元に走り、適格Node/選択が失われる。設定・Explore/topic・初回案内の共通入口から到達する。
- 影響: 検索可能になった画面が再び接続待ちへ戻り、Tauriの次のeventまたはfallback pollを待つ。AC-6は到着順に依存しない収束を明示しており、pollの修正だけでは不足する。
- 局所確認: 実handler本文にreadyの現在状態と遅延retrying responseを与えると出力は `status after delayed action response: retrying false`。この局所確認のupsert stubは置換を観測するものだが、実merge関数の上記代入と一致する。実event bridgeを含むdeferred統合testは修正側で必要。
- 必要な修正: action中のNode別状態更新を区別し、古い応答で新しい状態を戻さない。状態が変化した場合は最新のread-only状態再取得等で確定させる。既存refresh/撤回など同じstatus反映callerも確認する。

## 再構築した入口とsink

| 群 | 登録・入口 → helper → sink、guard | 判定 |
| --- | --- | --- |
| INV-1 | Appのstartup gate→DesktopShellPage→CommunityNodeOnboarding/useCommunityNodeOnboarding→firstUnconsentedCommunityNode。全config Nodeのlocal records確認、index 0、author/session表示state、他modal待機。説明表示はstore/UI stateだけで追加外部I/Oなし | 適合 |
| INV-2 | Explore/topicのCommunityIndexWorkspace、新規案内、CommunityNodePanel設定→新hookまたはprofileTopicChannel fetch/accept/withdraw→DesktopApi。新hookはNode/言語/世代・二重操作・設定再読取を実施。設定の旧経路はF1/F2、共通結果反映はF3 | 不適合 |
| INV-3 | runtime event bridge→applySyncStatusChange→merge、初回/60秒fallback poll→useConnectivityStatusRefresh、全store更新→withCommunityIndexSelection。poll競合は保護、受諾との逆順はF3 | 不適合 |
| INV-4 | 初回/section load、Node設定保存、明示retry→section loader/recovery→config/manifest読取、既存metadata IPC。request id/config identity/最新URL一覧で遅延manifestを棄却。recoveryはmembership・active consent・retry期限を再確認 | 適合 |
| INV-5 | Explore/topic header・設定auto/manual、Node削除→resolveCommunityIndexNodePreference、利用者search/discovery/recommendations→query IPC。manual不適格は停止、削除時auto、config順維持。context/sequence照合で旧Node/topic結果を除外、成功空だけempty表示 | 適合 |
| INV-6 | runtimeApi command→Tauri commands/community_node.rs:385/401、lib.rs:454/455→runtime accept/withdraw、scheduler→ensure session→preflight。local record→暗号化保存。Node membership/active records/現行snapshotがtoken発行・consent sync・bootstrapを支配。index queryもsession outcomeを確認。global applyはNodeごとのcurrent_policy_verified_forとlocal record照合 | 適合（静的確認、Rust実行は親gate） |
| INV-7 | Dialogの製品callerはIndex、設定、Dome Hosting、初回案内の4つ。eligibleCommunityNodesの直接wrapperはIndex/Trust/DistanceOptout/TesterFeedbackで変更なし。設定/Domeの共有catalogと受諾error経路はF1/F2 | 不適合 |

逆引きはaccept/withdraw IPC登録、persist_community_node_local_consents、request_accept_community_node_consents、request_community_node_authentication_token/token保存、index_query_support HTTPとapply_ready_community_node_connectivityを起点に確認。Rustはsession runtimeとconsent preflight、scheduler、requests、config/supportの既存の保護を維持している。既存境界testにはnode_without_local_consent_is_never_contacted、status_getter_is_read_only_and_does_not_bootstrap_session、policy_update_is_not_silently_reaccepted、saved_token_does_not_bypass_same_version_snapshot_preflight、connectivity_apply_ignores_local_consent_without_verified_ready_session、pending consentでindex/private indexing HTTPを止めるtestがある。本監査では全Rust suiteを重複実行していない。

## AC / INVAR evidence

| 条件 | evidence・判定 |
| --- | --- |
| AC-1 | store同期導出で通常event→即時選択は成立する構造。F3の逆順は未達 |
| AC-2 | availability純関数とNotice、401/CONSENT_REQUIRED/その他403/429の型付き分岐、success empty区別。availability/recoveryの局所test成功 |
| AC-3 | shell配置・全Node未同意判定・index 0主操作。新flowはdefault特例なし |
| AC-4 | loaded/error/空/active consentで抑止、author別session state、dismiss後の既存手動入口。startup gateはApp側で先行 |
| AC-5 | 新hookの取得失敗retry/旧言語・旧Node応答/二重受諾/設定削除test成功。設定/DomeにはF1/F2が残る |
| AC-6 | manifest request世代・poll baseline・store導出で改善。F3が残る |
| INVAR-1 | 新説明は外部I/Oなし。新flowは公開policy→明示accept→既存runtime preflight。送信guardの緩和なし。Rust禁止I/O testの実行は親gateで確認が必要 |
| INVAR-2 | config順、空一覧、manual維持/削除時autoを純関数とstoreで確認。statusからNodeを追加する処理なし |
| INVAR-3 | App gate後だけshellを描画。不同意からDirect P2Pや非Node機能への追加禁止なし。queryのprivate/context境界変更なし |
| INVAR-4 | 新hookは提示snapshot/言語を固定。runtimeはlocal record正本、withdrawは履歴。設定/DomeでF1が未達 |
| INVAR-5 | 共通Dialogのfocus指定、handoff、background modal検出、query context/sequenceの既存防御。実機keyboard/zoomは親担当。F1の遅延文書はAC-5側で計上 |

TR-1〜3,6〜9は上記対応と対象テスト・ソース根拠に紐付けた。TR-4,5,10はF1〜3により未達。実機Debian/Windowsや全体CIの未確認を成功扱いしていない。

## 実行したvalidation

- `npx pnpm@10.16.1 exec vitest run src/shell/actions/useCommunityNodeConsentFlow.test.tsx src/shell/data/useConnectivityStatusRefresh.test.tsx src/lib/api/communityNodeAvailability.test.ts src/shell/actions/useCommunityNodeRecovery.test.tsx src/components/core/CommunityIndexWorkspace.consentGate.test.tsx`: **5 files / 34 tests PASS**、7.95秒。
- 実handler本文抽出による遅延policy応答・受諾反映の局所制御フロー再現: 上記F1/F3の不整合を観測。
- 全suite、CI、Debian/Windows実機確認は親担当の並行作業であり、この記録では実行済みとしない。

## Non-blocker

- backendの新API、統一された全アプリmodal coordinator、新しい同意store等は要求していない。固定要件を満たす最小修正でよい。
- 未変更のTrust/DistanceOptout/TesterFeedback自体の機能拡張はNon-goal。
- 視覚の最終証跡追加だけならコード全監査のやり直しは不要。F1〜3修正後は共通caller/応答順のdeltaを監査する。
