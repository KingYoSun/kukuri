# 自分のアカウント操作メニュー実装プラン

## 状態・目的・対象外

- 状態: In progress。推奨案・Issue作業・コミット・PR・CI成功後のマージ・Windows/Ubuntu24実機確認は承認済み。
- Scope revision: 2026-09-13-v3。追加要件を受け、v1の「ログアウト後は未ログイン画面」を撤回し、本書全体を更新した。
- 基準commit: `3aca305fc8a167c5714c218d06c0b70021d471dd`（初回調査時）。実装開始時に現在の差分と再照合する。
- 実装のリスク区分: C（identity、鍵生成・import、registry永続化、runtime切替、初回案内と同意境界）。
- 目的: 通常画面から自分のプロフィール表示・アカウント切替・追加・管理・ログアウトを行い、新規アカウントでは初回プロフィール設定へ案内する。
- 対象: Windows / Linux desktopの現行shell、アカウントlifecycle、初回起動、ログアウト後の再起動と再ログイン。
- 対象外: 完全削除、全端末ログアウト、サーバー側鍵失効、暗号形式変更、backup形式の再設計、workspace全体の再設計。「アカウント追加」は鍵importと「新しいアカウントを作成」の両方を提供する。鍵生成は初回起動・他アカウントがないlogout・明示的新規作成の経路で行う。

参照: [PLANS.md](../../PLANS.md)、[Issue運用](../runbooks/issue-lifecycle.md)、[DESIGN.md](../../DESIGN.md)、[UI開発フロー](../adr/0014-uiux-dev-flow.md)、[UI実装配置](../architecture/desktop-ui-implementation.md)、[検証マトリクス](../../REFACTORING.md)、[開発手順](../runbooks/dev.md)。

## 現行実装から確認した事実

CodeGraphを先に利用して調査した。以下は既存の事実であり、後述の変更要件とは区別する。

| 対象path / symbol | 現行挙動と影響 |
| --- | --- |
| `apps/desktop/src/shell/page/DesktopShellControlCenter.tsx` | `.shell-control-cluster` にtriggerがあり、その直前がアカウントボタンの挿入位置。呼出元は `shell/DesktopShellPage.tsx`。 |
| `shell/page/DesktopShellSettingsDrawer.tsx` | section ID `account` が `AccountKeyPanel` を表示する。既存route同期を利用する。 |
| `components/settings/AccountKeyPanel.tsx` | 一覧・export・preview付きimport・切替を持つ。import成功は登録のみで「今すぐ切り替える」は別操作。切替成功後に列下書きを消し、ページをreloadする。 |
| `crates/desktop-runtime/src/accounts.rs` | AccountRecordはid / pubkey / label / timestampsのみ。labelはプロフィール表示名ではない。registryのactive IDは必須String。registry欠落で移行または初回鍵生成へ進むため、logoutでregistry自体を削除してはいけない。 |
| 同ファイルの `add_account` | 登録済みpubkey/idは拒否する。安定したIDのdirectoryへ鍵を保存後に登録する。残存データを安全に再利用する契約を追加する必要がある。 |
| `crates/desktop-runtime/src/host/mod.rs::ClientHost::switch_account` | guardで直列化し、次runtime構築・active更新・runtime置換・旧runtime停止を行う。CLIとTauriの共有境界。 |
| `apps/desktop/src-tauri/src/commands/identity.rs` | identity IPC入口。起動状態・復元guard・通知再同期との整合が必要。 |
| `apps/desktop/src/shell/page/useCommunityNodeOnboarding.ts` | accountのpubkeyごとにsession内の「表示済み」を保持する。別Dialogが開いている間は表示を待つ。「表示済み」は同意完了を意味しない。 |
| `shell/page/CommunityNodeOnboarding.tsx` | 導入説明から規約Dialogへ引き継ぎ、「あとで」では説明を閉じる。新しいプロフィール設定へ渡す同意／スキップの結果通知が必要。 |
| `shell/useDesktopShellRouting.ts::openProfileOverview` | 現在はprofileModeとrouteを更新する。既存プロフィールカラムの検索・追加・画面scroll・DOM focusまで保証するかを実装時に確認する。 |
| `components/extended/ProfileEditorPanel.tsx` | 表示名・ユーザー名・自己紹介・画像等の既存編集component。初回設定のモーダルは新設し、保存・検証処理を共用する。 |

仕様参照: [ADR 0047](../adr/0047-account-key-export-import-multi-account.md)、[ADR 0048](../adr/0048-device-backup-restore.md)、[ADR 0049](../adr/0049-linux-gui-cli-control-plane.md)。

## 受入条件

| ID | 完了条件 |
| --- | --- |
| AC-1 | Control Center左隣に、自分の丸いアバターボタンを表示する。画像欠落時もfallbackで操作可能。 |
| AC-2 | メニューは管理対象アカウントだけを列挙し、各行にアバター・表示名・ユーザー名を表示する。現在アカウントを文字／チェックとaccessible stateで区別する。 |
| AC-3 | 別account行を1回選ぶと追加確認なしで切替が始まり、新しい本人性でUIを再構築する。pendingは二重操作を防ぎ、失敗時は旧accountを維持する。 |
| AC-4 | メニューは最上部に「プロフィール表示」、その下にアカウント一覧、最下部に「アカウント追加」「アカウント管理」「ログアウト」をこの順で配置する。 |
| AC-5 | 「アカウント追加」は設定の「アカウント鍵のインポート」と同じ入力・preview・検証・エラー・成功・切替操作を備える共通formのモーダルを開く。 |
| AC-6 | 「アカウント管理」は設定の `account` sectionへ直接移動し、既存routeと戻る文脈を保つ。 |
| AC-7 | 「ログアウト」は現在accountの確認モーダルを表示する。ローカルデータ保持、管理一覧からの除外、同じ鍵を追加して再ログイン可能と説明する。ボタンは「はい」「キャンセル」。取消／Escapeでは変更しない。 |
| AC-8 | logout成功で対象の登録とruntime・購読・通知を外し、切替前に使用していた登録済みaccountをactiveにする。他accountの内容とlogout対象のローカルデータを削除・上書きしない。 |
| AC-9 | 他に登録accountがないlogoutでは新しい鍵ペアとaccountを作成・有効化し、新設の初回プロフィール設定モーダルへ進む。再起動や再試行で追加の鍵ペアを重複生成せず、logoutしたaccountも自動復活させない。同じ鍵の再importで元データを再利用できる。 |
| AC-10 | pointer / keyboard、focus復元、狭幅、長名、対応locale、dark/light、画像欠落、loading/partial/errorを扱う。 |
| AC-11 | 「プロフィール表示」は現在ログイン中の本人のプロフィールカラムを探して画面とkeyboard focusを合わせる。なければ1件追加してfocusする。連打で重複作成せず、他人のプロフィールカラムは置換しない。 |
| AC-12 | 初回起動で生成したaccountでも新設の初回プロフィール設定モーダルを出す。初回のコミュニティノード同意が正常完了した後、または利用者がスキップした後に表示する。説明から規約への移動や同意失敗を完了扱いしない。 |
| AC-13 | 初回設定は既存プロフィール保存・検証契約を共用し、保存成功で本人のプロフィール・アバター・メニューに反映する。保存失敗は入力を保持して再試行可能。完了済みaccountでは通常切替・再起動で初回設定を繰り返さない。 |
| AC-14 | 「アカウント追加」モーダルに「新しいアカウントを作成」を配置する。押下すると既存登録・データを残して新しい鍵ペアを作成し、そのaccountへ切り替える。必要な初回CN同意／skipの後に初回プロフィール設定へ進む。二重押下・retryで登録を重複させず、失敗時は既存accountを利用可能に保つ。 |

### 維持する条件

- INVAR-1: 平文秘密鍵をUI / IPC / ログ / クリップボードへ出さない。暗号化envelopeとパスフレーズの既存保護を維持する。
- INVAR-2: logoutでDB・blob・プロフィール・設定・非公開チャネル状態等を削除しない。別accountのデータ・編集中入力・下書きを表示／送信しない。
- INVAR-3: switch/import/logout/鍵生成/restoreをbackendで直列化し、active・registry・runtimeの成功を一致させる。途中失敗・crash・再試行で対象復活、二重鍵生成、別account除外を起こさない。
- INVAR-4: menu一覧・非activeプロフィール表示のために非active runtime、P2P、CN sessionを起動しない。一覧DTOとregistryに秘密値を追加しない。
- INVAR-5: アプリ同意・年齢確認・テーマ・言語・workspace layoutの端末単位契約を維持する。CNの同意／スキップをプロフィール表示条件と混同せず、未同意nodeへの外部送信を許可しない。
- INVAR-6: 初回案内はaccount単位で管理する。旧accountの遅延callbackが切替先のプロフィール保存・初回完了・同意状態を変更しない。

## UI・動作契約

変更分類は「新規画面・導線」。Control Center内容の再編は行わない。

### メニューとプロフィール表示

- triggerに追従する既存menu primitiveを優先する。名前のfallbackは表示名→ユーザー名→不明なユーザー。ユーザー名未設定と取得不能を区別し、公開鍵全体を通常行の代用品にしない。
- 非activeプロフィールとavatarは各accountのlocal projection/blobを読む。profile表示用DTOをregistry metadataから分離し、必要なら追加する。画像の所在はaccountとhashで扱い、欠落時はfallback。新たなnetwork取得はしない。
- プロフィール表示の対象pubkeyは操作時のactive本人から取得する。既存workspaceの追加／activate／scroll／focus経路を共用する。本人カラムが複数ある既存状態では現在の本人カラム、なければworkspace順の先頭を選ぶ。
- 本人の概要表示へ移動し、既存カラムの位置・幅・pin状態を保つ。編集途中のprofile draftは捨てず保持し、他カラムのdraftやscrollに触れない。menuを閉じた後のfocusをtriggerへ戻す処理が、カラムfocusを奪わないようにする。
- 一覧更新・profile保存・切替成功で表示を更新する。選択中accountの行は無操作。import成功は登録のみで「今すぐ切り替える」を提供する既存契約を維持する。

### ログアウト先の選択

1. A→Bと使用した後でBからlogoutしたらAへ戻す。メニュー以外の設定・import後切替・CLI共有host経路でも、切替成功時に使用履歴を更新する。失敗・同じaccount選択・一覧表示では履歴を更新しない。
2. 前のaccountが既に登録から外れている場合は、履歴を遡り、残る登録accountのうち直近使用のものを選ぶ。旧registry等で履歴がない場合は `last_used_at` 降順、同値はid順とする。これは未指定境界への作業仮定。
3. 他の登録accountが0件の場合だけ新しい鍵ペアを作成する。残存データdirectoryは「他の登録account」に数えない。
4. 選ばれたaccountの起動失敗を「他にaccountがない」と読み替えて新規生成しない。失敗は整合した旧状態または明示した回復状態として扱い、成功と表示しない。
5. 新規生成後もそのaccountで必要な初回CN同意／スキップを先に扱い、その後プロフィール設定を表示する。通常起動とlogout後の新規accountで同じ初回flowを使う。

### 初回プロフィール設定モーダル（新設）

- 順序: アプリ利用条件の既存gate → runtime ready → 初回CN案内／規約 → 同意完了またはスキップ → 初回プロフィール設定 → 通常利用。
- CNの「表示済み」やDialogが一瞬閉じたことでは進めない。同意成功と明示スキップを型付きの結果として初回flowへ渡す。規約確認の中断は説明へ戻すかスキップを選べるようにし、同意として扱わない。
- CN候補なし／既に同意済みと確認できた場合はCN工程を対象外／完了としてプロフィールへ進める。取得失敗・offlineを候補なしと見なさず、再試行と明示スキップを用意して案内が無期限に止まることを防ぐ。スキップはCN同意の保存・認証・送信を行わない。
- 表示名・ユーザー名・自己紹介・アバターは既存ProfileEditorPanelの項目と保存／画像検証を共用し、初回向けDialogを新設する。新しい必須項目や独自validationは追加しない。公開されるプロフィール情報であることを既存契約に沿って説明する。
- 作業仮定: ボタンは「保存」「あとで」。保存成功は完了を永続化。「あとで」／閉じる／Escapeはそのsessionの自動再表示だけを抑止し、未完了は次回起動時に再案内する。設定・プロフィール編集から後で設定できる。保存中は二重送信を防ぎ、失敗時は入力を保持する。
- 初回対象フラグと完了状態をaccount単位で永続化する。新規生成時に対象化し、既存account／import済みaccountをプロフィール空欄だけで新規と推定しない。migrationでは既存利用者へ一律に再表示しない。
- 初回設定未完了の新規accountはrestart後も同じ鍵ペアでflowを再開する。profile保存成功後、完了フラグ保存前のcrashも、同じ保存結果を利用して回復し、不必要なプロフィール再送信をしない。
- 他Dialogとの表示を直列化し、CN→profile間でfocusが背景へ飛ばないようにする。account切替時は旧Dialog・画像preview・非同期結果を破棄し、別accountへ保存しない。

### 保持範囲と確認文言

- 作業仮定: ローカルデータ保持には既存の鍵保管も含める。再登録は暗号化鍵とパスフレーズの検証を必須とし、残存鍵だけで自動ログインしない。
- logout時の列下書きは対象account IDで退避し、active UIから外す。同じaccountの再ログイン時だけ復帰する。通常切替の下書き破棄契約は全面変更しない。退避失敗はlogout確定前に検出する。
- 確認モーダルは対象avatar・表示名・ユーザー名を示し、初期focusを「キャンセル」に置く。説明案:

> このアカウントからログアウトしますか？
>
> この端末のアカウント管理一覧から削除されますが、ローカルデータは端末に残ります。「アカウント追加」から同じアカウント鍵をインポートすると、残っているデータを利用できます。再ログインには、エクスポート済みのアカウント鍵とパスフレーズが必要です。
>
> ［はい］［キャンセル］

- 確認時の補助文に、戻るaccountがある場合はその表示名、ない場合は「新しいアカウントを作成し、初回設定へ進みます」を表示する。確定時はbackendが対象と切替先を再検証し、staleな確認で別accountをlogoutしない。
- Desktop／759px以下ともControl Center左隣の順序を維持する。viewport内のmenu、長い一覧のscroll、低い高さ・200% zoomでも主要ボタン到達、portal token／safe area／z-indexを確認する。
- 全surfaceでdefault/hover/focus-visible/selected/disabled/pending/error、loading/refresh/partial/offline/emptyを定義する。emptyは取得成功0件のみ。キーボードとpointer、screen readerで同じ結果へ到達できるようにする。

## データ境界・永続化

- 通常のlogout成功状態は必ず別のactive accountを持つ。v1で提案した未ログイン画面やactiveなしを通常の製品状態としては追加しない。
- registryには切替履歴、初回設定状態等の必要なLocal Only metadataを追加する。形式変更は既存registryの移行と不明version／破損拒否をセットにし、既存鍵やIDを変更しない。
- logoutは共有lifecycle操作で、対象IDを検証し、履歴から戻り先選択／0件なら鍵生成、次runtime準備、登録除外と次activeの永続化、runtime置換・旧停止を制御する。
- commit point、journal等の要否、保存／停止／置換失敗時のrollback範囲をT1で確定する。生成した鍵の識別子と再開情報を残して再試行で再利用し、crashで新規accountを量産しない。確定済みlogoutを巻き戻して対象を自動復活させない。
- 再importは復号後の完全なpubkeyで残存identityと一致確認する。ID先頭16桁一致だけで採用しない。DB・blob・設定・鍵を別鍵で上書きしない。登録済み同一鍵の拒否を維持する。
- 共有host・registry型の変更はTauri/CLI/backup/restore/起動の全callerへ反映する。同意gate、復元排他、通知購読の再同期はfrontend表示だけで保証しない。

### Feature Data Classification（ADR 0002）

| 項目 | 分類 |
| --- | --- |
| Feature名 | アカウントメニュー・履歴に基づくlogout・新規account初回プロフィール設定 |
| Durable / Transient | 登録・履歴・鍵・初回完了・回復情報・退避下書きはDurable。menu/Dialog/入力/pendingはTransient |
| Canonical Source | registryとaccount単位の初回状態、既存profile projection、local blob、既存keyring/file |
| Replicated / 区分 | 管理／初回案内状態はLocal Only。保存するprofileとavatarは既存の公開・複製契約に従う |
| Rebuildable From | 表示はlocal profile/blob。logout意思・履歴・初回完了は明示的な永続状態を正とし、directoryや空欄から推定しない |
| Gossip Hint / Blob | 管理状態の新規gossip不要。profile保存・avatar uploadは既存経路を利用 |
| SQLite projection | 管理用の新規projectionは原則不要。profileは既存projectionを更新 |
| 必須contract | 履歴選択・鍵生成冪等性・データ保持・再import・同意後の表示順・初回状態のaccount分離・失敗回復 |
| 必須scenario | A→B→B logout→A、最後のlogout→新規C→CN同意/skip→profile→restart、旧鍵再import→元データ復帰 |

実装時にADR 0047、[既存データ分類](../legal/account-key-export-data-classification.md)、DESIGNの初回案内・profile導線を同期する。profile保存の外部副作用は既存仕様を参照し、単なるローカルmenu表示と区別する。

## 固定surface inventoryとsensitive sink

T1で共有helperの全callerとsink逆引きを補完し、未分類を残さない。

| ID | 入口・trigger | shared helper / 対象 | 読み書き・副作用 | guard / invariant | 検証 |
| --- | --- | --- | --- | --- | --- |
| INV-1 | menu・設定一覧・profile更新 | identity API、accounts、local profile/blob | 読取 | INVAR-1/4 | TR-1 |
| INV-2 | menu切替・設定切替・import後切替・CLI switch | ClientHost::switch_account、Tauri/CLI | active/履歴、runtime、通知 | INVAR-2/3/5/6 | TR-2/7 |
| INV-3 | 設定import・追加Dialog | preview/import、add_account | 復号・鍵・登録 | INVAR-1/2/3、同意・完全pubkey検証 | TR-3/6 |
| INV-4 | logout確認・新IPC | 共有logout、registry、鍵生成、host | 対象除外、次active、鍵、runtime停止 | 全INVAR、対象active検証・排他 | TR-4/5/7 |
| INV-5 | 初回・restart・移行・CLI host開始 | accounts初期化、host、startup、App | registry、鍵、runtime、初回状態 | INVAR-1/3/5/6 | TR-5/8/10 |
| INV-6 | backup/restore、observer/scheduler/通知再開 | backup、restore_lifecycle、host | registry・runtime・CN更新 | INVAR-2/3/5/6 | TR-7/8 |
| INV-7 | プロフィール表示 | shell routing、workspace activate/add、scroll/focus | route・カラム配置 | INVAR-2、本人pubkeyで選択 | TR-9 |
| INV-8 | CN同意・skip・設定移動・取得失敗、profile保存/あとで | CommunityNodeOnboarding、consent flow、新初回Dialog、profile actions | 同意、profile/avatar保存、初回状態 | INVAR-1/2/5/6 | TR-10/11 |

逆引き対象:

- `save_registry` / `set_active_account` / `register_restored_account`: import・switch・初回生成・migration・backup・logout・履歴更新。
- 鍵生成 / `persist_keys` / `load_existing_keys` / `delete_identity`: 条件付き生成・再利用・pubkey照合。logout対象の削除0回。
- runtime構築／置換／停止、scheduler・observer・通知dispatch: logout完了後に対象の送信・通知・background mutationが0回。
- IPC/CLI登録点と復元guard: stale要求、並行操作、同意不足が永続mutationへ到達しない。
- profile/avatar保存と初回状態の保存: account generation検証、保存先本人性、CN未同意への送信0、保存失敗を完了扱いしない。
- localStorage・store・cache・route: 下書き保全と別account混入防止、プロフィールカラム重複0。

## 状態遷移と検証

| ID | 事前状態・操作 | 期待状態／許可I/O | 禁止副作用 | 証跡 |
| --- | --- | --- | --- | --- |
| TR-1 | menu表示、画像／profile欠落、offline、取得失敗→retry | local値保持・fallback・回復 | 非active runtime起動0、false emptyなし | UI/Rust読取test、Story |
| TR-2 | A→B、同じA選択、B起動失敗 | 成功時だけactiveと履歴更新、失敗A維持 | 失敗履歴更新0、別本人性表示0 | host/Tauri/CLI回帰 |
| TR-3 | 新規鍵import、誤passphrase、改竄、登録済み | 登録またはエラー | 検証失敗時の鍵/DB/registry変更0 | import contract/form test |
| TR-4 | A→B→B logout、cancel/Escape、履歴対象消失 | 成功A復帰、取消B維持、消失なら残存履歴へ | 対象データ削除0、残存accountありの鍵生成0 | 履歴/取消/UI test |
| TR-5 | 最後のA logout→新規C→restart/retry | 同じCをactiveとして初回flow再開 | C二重生成0、A自動登録/起動0 | 永続scenario、fault injection |
| TR-6 | logout済みAの同鍵import→A切替 | 元ID・投稿・profile・blob・退避下書き復帰 | pubkey不一致の上書き0 | 永続scenario、負例test |
| TR-7 | logout中のswitch/import/restore、stale要求、停止/保存/鍵生成失敗、crash | 直列化・拒否・整合回復 | 別account除外0、成功偽装0、確定対象復活0 | 並行/故障注入test |
| TR-8 | 初回、旧registry、破損/不明version、同意不足、復元待ち | 適切な初期化/移行/拒否 | 破損から鍵生成0、未同意送信0、既存account一律初回扱い0 | migration/consent/backup/CLI |
| TR-9 | 本人カラムあり／なし／他人だけ、連打、狭幅 | 本人をfocusまたは1件追加、本人概要表示 | 他人置換0、重複0、draft破棄0 | pointer/keyboard browser、route test |
| TR-10 | 初回CN説明→規約→同意成功 or skip、同意失敗/中断、候補なし、offline | 完了/明示skip後だけprofile。対象外は確定後に進行 | Dialog競合0、失敗の完了扱い0、skipの同意mutation/送信0 | onboarding既存test＋順序test |
| TR-11 | profile保存成功/失敗/あとで、完了後restart、保存中account切替、完了記録前crash | 成功反映、失敗入力保持、未完了再案内、完了非再表示 | 別account保存/完了0、遅延callback適用0、不要再送信0 | profile actions/UI/永続test |

## 作業

4段階・8 Task。T1/T2で契約と変更前の保護を置いてから実装する。

| ID | 段階 | AC / INVAR | 作業・対象path | 受入条件と検証・証跡 | 依存 |
| --- | --- | --- | --- | --- | --- |
| T1 | 1. 契約 | 全AC / 全INVAR | accounts/host/startup/CLI/backup、shell routing/onboarding/profile調査。ADR 0047・DESIGN・分類更新案 | caller/sink分類、履歴移行・鍵生成commit point・初回flow永続状態・profile保存の根拠付き判断を固定 | なし |
| T2 | 1. 契約 | 全AC / 全INVAR | accounts_migration、host/Tauri/CLI tests、AccountKeyPanel.test、DesktopShellPage.communityNodeOnboarding.test、scenario | 既存挙動のcharacterizationとTR-1〜11の必要testを先に置き、未実装の失敗を既知失敗と区別 | T1 |
| T3 | 2. lifecycle | AC-2/3/8/9/12/13 / 全INVAR | `crates/desktop-runtime/src/accounts.rs`・`host/`、Tauri identity/startup/state/restore gate、CLI/schema | 履歴復帰、最後だけ生成、冪等回復、初回対象永続化、データ保持・再importをRust/Tauri/永続scenarioで確認 | T2 |
| T4 | 2. 共通操作 | AC-3/5/6/13 / INVAR-1/2/3/6 | `lib/api/identity.ts`、生成型、AccountKeyPanel、共通import form、shell actions/data、profile保存 | 共通import/switch・下書き退避復帰・profile検証保存を統合。既存設定回帰と型生成差分を確認 | T3 |
| T5 | 3. menu | AC-1〜7/10/11 / INVAR-1/2/4/5 | ControlCenter、DesktopShellPage、新menu/Dialog、useDesktopShellRouting/workspace、styles/i18n | 最上部プロフィール、左隣trigger、一覧、3操作、本人カラムfocus/addをStory・Vitest・browser実操作で確認 | T4 |
| T6 | 3. 初回flow | AC-9/10/12/13 / INVAR-2/3/5/6 | CommunityNodeOnboarding/useCommunityNodeOnboarding、consent flow、新初回profile Dialog、ProfileEditorPanel共用部、startup/store同期 | 初回と最後logout後でCN同意/skip→profile順序、保存/回復/account分離をTR-10/11で確認 | T3/T4/T5 |
| T7 | 4. 統合検証 | 全AC / 全INVAR | 対象test、browser/visual/localization、Windows/Linux実機、docs/ui-reviews/progress | 下記必須validation、同条件before/after、全ACと証跡の対応。未確認を明示 | T6 |
| T8 | 4. 独立監査 | 全AC / 全INVAR | 実装担当と独立した担当/別コンテキストによる固定commit監査 | inventory未分類/不適合0、blocker0、PASS。PR工程が承認された場合はPR headで実施し変更delta再監査 | T7 |

### 必須validationと証跡

- 基本は `cargo xtask check` / `cargo xtask test`。同じcommandの重複実行を避ける。
- frontend: `cargo xtask desktop-ui-check`。新menu、logout確認、import、初回profileのStory、実pointer/keyboard browser、localization-layout、visualを含める。
- runtime: `cargo xtask rust-test`、起動/永続往復の `cargo xtask e2e-smoke`。TR-4/5/6の追加scenarioは既存smokeで代替せず個別登録・実行する。
- Tauri: `cargo xtask tauri-check`、`cargo xtask e2e-smoke`、対象lifecycle/guard tests。compileのみでIPC検証済みとしない。
- CN session切替・停止と初回同意経路に影響するため `cargo xtask scenario community_node_public_connectivity`。未同意skip経路の禁止送信0を独立に検証する。
- identity restart、registry migration、backup/restore、CLI lifecycle/schema、既存onboarding、既存profile編集の回帰を確認。生成DTOは生成元から同期する。
- UI条件: 1440px/760px/390px、低い高さ、dark/light、全対応locale、長名/画像失敗/多数件、200% zoom、Windows High Contrast、reduced motion、screen reader。CN→profileのfocus引継ぎと本人カラムのscroll/DOM focusを実操作で確認する。
- Windows/Linux Tauri/WebViewで鍵保管、実runtime切替・停止、最後logout→新規鍵→初回flow→restart→旧鍵再importを確認。Windowsのsnapshot skipはLinux/CIで補う。
- `git diff --check`。未実行・失敗・実機未確認を成功扱いしない。

## 未決事項・次の一手

- ユーザー追加要件はAC-8/9/11/12/13に反映済み。未ログイン画面への遷移案は廃止した。
- 未指定境界への作業仮定は、戻り先履歴が無効な場合の選択順、初回profileの「あとで」と再表示、既存accountの移行対象除外、鍵保持・下書き退避。新しい要件として黙って確定した既存仕様とは扱わない。
- T1で非active profile読取、履歴／初回状態の永続形式、logout切替・鍵生成のcommit point、CN結果通知とprofile保存の全callerを有限に確認する。
- 承認済み範囲でT1から実装・検証・独立監査・PR・マージまで進める。

## v3追加要件（ユーザー承認済み）

- AC-14はT3（生成予約・登録とactiveの原子的反映）、T4（共通session遷移）、T5（追加モーダル）、T6（既存初回flow）で実装する。対象はidentity IPC/API、account lifecycle、AccountMenu、設定との共通import formの境界。
- INV-9: 追加モーダルの明示的新規作成→CreateAccountRequest→host生成・切替→registry/鍵/runtime。INVAR-1/2/3/5/6を適用する。
- TR-12: A使用中に新規作成→Bがactive、A登録・データを保持、B初回flow。cancel／stale対象／生成・保存・起動失敗は既存状態を保持。operation IDでretryを冪等にし、重複登録・別account削除を0にする。Rust/UI/browser/実機で検証する。
- 新規作成後のlogoutはAへ戻る。既存のimport成功は登録のみという契約を維持する。

## 実装・検証記録（進行中）

- Issue: #1005、関連報告 #993、PR: #1006。
- registryは履歴・生成予約・完了operation記録・初回対象をLocal Onlyで保持。登録済みidentityは生成前に完全pubkey一致を検証する。
- 初回profileは署名済みenvelopeとrequest hashを先にjournalへ保存する。同じ要求は再利用し、変更された明示入力は古い操作の回復後に新保存する。既に同じdocs内容なら再送信しない。
- logout/create/switchのcommit失敗はregistry読戻しで分類。未確定だけ旧runtimeへ戻す。確定済みは再flushして新runtimeを維持、不明なら両runtimeを停止してFailedとする。
- node停止は所有taskと完了通知でcancel-safeにした。router停止前にblobをflushし、endpoint closeを完了させる。routerが閉じたblob actorへの二回目shutdownは成功条件にしない。
- 下書きは切替前に現在storeを同期flushし、旧writerのdebounce/pagehide/cleanupを停止する。失敗が旧accountのままと確認できた場合だけwriterを再開する。
- CLI parityの既存契約によりcreate_account/logout_account/get_account_displayの生commandを共有hostへ配線。初回Dialogの保存/完了協調はGUIのfrontend_stateとして名前を限定して分類。独立したCLI専用機能設計は追加しない。

### 成功した対象検証

| 条件 | 実行・証跡 |
| --- | --- |
| AC-8/9/14、TR-4/5/6/12 | `cargo xtask scenario desktop_account_lifecycle`: 3工程PASS（実ClientHost、投稿・プロフィール・暗号化鍵import、restart） |
| INVAR-2/3、TR-7/8 | `cargo test -p kukuri-desktop-runtime --lib account_logout`: 10 tests PASS（creation tombstone、鍵欠落、再import、profile journal） |
| commit故障・queued操作 | `cargo test -p kukuri-desktop-runtime --lib host::accounts_tests`: 4 tests PASS（pre-commit、post-rename、unknown commit、終了後mutation 0） |
| node停止 | `cargo test -p kukuri-iroh-node cancelled_shutdown_caller`: PASS、caller cancel後に完了待ち・store再open |
| 下書き境界 | `columnDraftPersistence.test.ts` / `accountSession.test.ts`: 最新入力flush、pagehide書戻し禁止、restart回復、storage失敗でbackend mutation 0 |
| AC-12/13、TR-10/11 | `DesktopShellPage.initialProfile.test.tsx`: 5 tests PASS（skip、同意失敗/中断/成功、保存失敗後の編集、あとで/new session、offline明示skip） |
| AC-1..7/11/14 | `DesktopShellPage.accountMenu.test.tsx`、`account-menu.spec.ts`: menu順序、切替、logout取消/確定、本人カラムfocus/重複防止、作成入口PASS |
| 既存CN flow | Playwright `community-node-onboarding.spec.ts` + account-menu: 16 tests PASS（既存Escape regressionを修正後） |
| CLI全登録分類 | `cargo test -p kukuri-cli --test command_parity`: 5 tests PASS、151 GUI入口を分類 |
| frontend統合 | 全unit/shell 1624 tests PASS（後続追加分は対象testと最終CIで確認）。browser初回は275 PASS/1 FAIL、既存Escape regression修正後は対象16件PASS |
| 視覚 | Linux/Chromium専用workflow run 34751037087成功。新しいmenu/addの4画像だけを採用し、無関係な既存画像の揺れをbaseline更新しない |

### 実機確認の途中証跡

- Windows: 専用backendデータ `.codex/test-data/account-1005-windows` と専用WebView user dataでCN skip→初回profile→名前/ユーザー名保存→本人プロフィール反映、アカウントmenuの並びを確認。
- Ubuntu24: `ssh local2` の `/tmp/kukuri-ui-1005` と専用app dataで起動。Remote DesktopをComputer Useで操作し、CN skip→初回profile→名前/ユーザー名保存→本人プロフィール反映を確認。
- 新規作成・切替・logout・restart・同じ暗号化鍵の再importはWindows/Ubuntu24双方で成功。最終CIと監査の結果はPR #1006の固定headへ対応付ける。

### 最終実機結果・境界証跡

- Windows: A（Account QA Windows）→明示作成B→B logoutでAへ復帰→Aのlast logoutでC生成→アプリ再起動後もC維持→事前にGUIでexportしたA鍵を追加Dialogでpreview/import→A切替で元の名前/ユーザー名復帰。A/B directory保持、登録は除外、同じIDへ再登録を確認。
- Ubuntu24: Remote Desktopの実pointer/keyboardで同じsequenceを実行。A（Account QA Ubuntu24）の名前/ユーザー名が再import後に復帰。新規B/最後のCは初回CN skip後に初回profile Dialogを表示し、再起動でもCのIDが維持された。
- 画像と条件: [UI review record](../ui-reviews/2026-09-13-account-menu.md)。テスト専用データを使用し、端末の通常アカウントデータは対象にしない。
- INVAR-5: `node_without_local_consent_is_never_contacted` PASS。実runtime+HTTP fixtureでscheduler/status読取後の policies/challenge/verify/consent-status/consent-accept/heartbeat/bootstrap hit全0。
- INVAR-6: `queued_initial_profile_save_is_rejected_after_account_switch` PASS。guard待ちA保存要求をB切替後に解放して拒否し、A/B profile更新とregistry完了mutationが0。`InitialProfileSetup.test.tsx`でも遅延A応答後にB profileを維持。
- 新scenarioは `desktop_account_lifecycle` としてharness test／fast+release+nightly inventoryへ登録済み。直接scenario実行とharness testはともにPASS。
- CLI登録は既存136から139に増えるため、daemonのページ列挙期待値を139へ同期。全名前とschemaの整合はcommand_parity testが別途検証する。
- コード監査: 8fe37337までの製品コードと境界testの独立監査でblocker0・未分類0。後続はscenarioのCI登録、テスト件数期待値、証跡文書のみ。最終head CI完了と合わせて監査担当が最終判定する。
- CIのRust再実行では952件中951件が成功し、`account_logout.rs`の既存2箇所の`IdentityStorage`取得がlock分類表に未登録だったため最終contractが失敗した。分類行と総数132→134を同期し、同じcontractを単独実行してPASS。テストのlockや検査条件は変更していない。
- 同CIの初回DM接続timeoutは、製品コードを変更せずUbuntu24単独実行とCI再実行の双方でPASSを確認。初回失敗と再実行結果はPRの最終監査記録に併記する。
