# Issue lifecycle runbook

## 目的

Issue の起票、計画、実装、監査、PR、Close、Reopen を一つの有限な流れとして扱い、次を防ぐ。

- 入口の件数だけを数え、共通 helper の早期 return や副作用を見落とす。
- happy path の成功と全体 test の成功を、完了条件の網羅と取り違える。
- retry、restart、保存済み状態、background 処理、複数対象の混在で境界を迂回する。
- 監査のたびに将来要件を追加して、Close 条件を動かし続ける。
- Issue 本文の過去記録と現在判定が混在する。

この runbook は作業手順の正本である。製品仕様は ADR、現在状態は progress、実際の振る舞いは tests / contracts / scenarios を正本とし、GitHub Issue / PR の checkbox や説明だけを完了根拠にしない。

## 適用区分

起票時に次のいずれかを選ぶ。迷う場合は、ファイル数ではなく、失敗時の利用者影響と境界の数で上位を選ぶ。

| 区分 | 対象 | 必須ゲート |
| --- | --- | --- |
| A: 軽微 | 挙動を変えない文書、局所的な文言、機械的な保守 | 受入条件、対象 path、targeted validation |
| B: 挙動変更 | 一つの責務境界内に閉じる feature / fix / UI 挙動 | 固定 surface inventory、適用可能な状態遷移、AC / INVAR と test の対応 |
| C: 境界重要 | 認証、同意、privacy、外部送信、秘密値、identity、暗号、永続化、migration、backup、network、複数対象への global apply、shared guard | B の全項目、sensitive sink の逆引き、独立監査、監査後の Close |

法務文書や運用文書でも、実装上の同意・外部送信・削除手順を規定する変更は A ではなく C とする。

## Lifecycle state gate

| 状態 | 入る条件 | 出る条件 |
| --- | --- | --- |
| Planned | Goal、Non-goals、固定 AC / INVAR、リスク区分がある。B / C は inventory と transition もある | 計画の各 Task が AC / INVAR と evidence に対応し、Scope revision が固定された |
| In progress | 計画が承認され、failing-before の取得方法が決まった | 実装、targeted validation、inventory / transition 更新が完了した |
| Audit pending | PR head commit と AC / INVAR evidence が固定された | 必要な独立監査が `PASS`。対象変更後は delta も `PASS` |
| Merge ready | 必須 CI と必要な独立監査がともに成功した | 承認された運用で merge された |
| Complete | merge commit と監査対象の一致を確認し、Issue 本文の現在判定を更新した | concrete blocker が発見されない限り維持する |
| Reopened | Blocker の四条件を満たす evidence がある | 最小の残タスクを同じ AC / INVAR / transition に結び直し、再び `In progress` へ進む |
| Blocked | 承認が必要な scope 変更、外部依存、再現不能など、現在の工程を進められない具体的理由がある | blocker と解除条件を記録し、解除後に中断前の状態へ戻る |

`FAIL` または `INCONCLUSIVE` を `PASS` と同様に扱って先へ進めてはならない。一方、停止条件を満たした `PASS` に対して、対象 surface の変更や concrete blocker なしに全監査を繰り返してはならない。

既存の Open / Reopened Issue は、次の実装計画を作る前にこの形式へ移行する。過去の監査記録を消したり書き直したりせず、本文先頭に現在判定、固定 AC、対象 invariant を置き、詳細な旧記録は `Superseded` を付けた progress 文書または comment として参照する。Closed Issue は、後述する Blocker の四条件を満たす新しい evidence がない限り、移行だけを目的に Reopen しない。

## 1. Issue 起票と Definition of Ready

Issue 本文の先頭に `Current status` とリスク区分を置き、現在判定だけを更新する。

```md
## Current status
- 判定: Planned / In progress / Audit pending / Merge ready / Complete / Reopened / Blocked
- Scope revision: <日付または識別子>
- 基準 commit: <SHA。未着手なら None>
- Blocker: <0件、または AC / INVAR ID と要約>

## リスク区分
A / B / C
```

本文には次を必須とする。

1. 利用者に観測できる Goal を一文で書く。
2. 不具合なら、現在の挙動と再現 sequence を書く。
3. canonical な ADR、runbook、tests、scenarios をリンクする。
4. In scope / Non-goals と、親子 Issue の責務所有を明示する。
5. 受入条件へ安定 ID `AC-1`, `AC-2`, ... を付け、Yes / No で判定可能にする。今回の変更でも維持する既存契約は `INVAR-1`, `INVAR-2`, ... として同時に固定する。
6. 区分 B / C は surface inventory と状態遷移表を作る。
7. 区分 C は device 外送信、DB mutation、token、鍵、relay / seed、権限・audience 拡張などの sensitive sink を列挙する。

### 固定 surface inventory

件数だけの棚卸しは禁止する。入口から sink への順方向と、sink から全 caller への逆方向を両方確認する。

```md
| ID | 入口・trigger | shared helper | 読み書き・外部副作用 | 必要な guard / invariant | 対象 transition | test / scenario |
| --- | --- | --- | --- | --- | --- | --- |
| INV-1 | ... | ... | ... | ... | TR-1 | ... |
```

入口には、該当するものをすべて含める。

- public API / route / IPC / command
- UI action と別画面・別導線
- startup / restart / restore
- scheduler / observer / retry / self-heal
- config setter / cache refresh / reconnect
- shared helper の全 caller
- 複数対象をまとめて反映する global / aggregate 処理

同じ guard と同じ sink を共有する入口は一つの group にまとめてよい。ただし、group の全 member 名または機械的な列挙方法を記録する。`未分類 = 0` は必要条件だが、各行の副作用と guard が正しいことを別に確認する。

### 状態遷移表

全状態の直積は作らない。機能に適用可能な制御フローの同値クラスを有限に選ぶ。

```md
| ID | 事前状態 | event / sequence | 期待状態 | 許可する I/O | 禁止する副作用 | test / scenario |
| --- | --- | --- | --- | --- | --- | --- |
| TR-1 | ... | ... | ... | ... | ... | ... |
```

区分 B / C では、該当する場合に次を必ず含める。

- fresh / missing state と正常状態
- stale version / revoked / corrupt state
- 外部 I/O 失敗から retry deadline 中まで
- restart 後の保存済み state / cache
- user action と background action の両方
- 401、再認証、fallback、error swallowing、早期 return
- 複数 user / account / node の mixed state と global apply
- mutation の cancel、部分失敗、rollback

区分 C では、guard がすべての sensitive sink を制御フロー上で支配していることを確認する。エラー値だけでなく、禁止 HTTP hit、DB row、token 更新、transport / relay / seed 適用が `0` または不変であることを test する。

### Scope freeze

計画承認時に `Scope revision`、AC、INVAR、inventory、transition を固定する。その後の発見は次の四つに分類する。

| 分類 | 条件 | 扱い |
| --- | --- | --- |
| Existing-gap | 固定済み `AC-*` / `INVAR-*` に違反し、現行 surface から到達する | 同じ Issue の Required。Close 済みなら Reopen |
| Regression | 対象差分が、固定外であっても canonical な既存挙動を新たに壊したことを before / after で示せる | merge 前は同じ PR の blocker。merge 後は原因 PR に紐づく bug Issueとし、必要な親だけ Reopen |
| New-requirement | 固定 `AC-*` / `INVAR-*` に含まれず、対象差分が新たに起こした Regression でもない製品要件 | 別 Issue。元 Issue の Close blocker にしない |
| Optional-hardening | 実用途の完了に不要な防御・一般化・性能改善 | Non-goal。原則として追加起票しない |

## 2. 実装計画

計画は `PLANS.md` と本書を同時に満たす。

- `AC / INVAR -> Task -> INV / TR -> test / evidence` の対応を記録し、孤立した条件と、条件に紐づかない実装 Task を 0 にする。
- 不具合は、静的な状態だけでなく問題が発生する event sequence を修正前に失敗する test / contract / scenario へ落とす。
- shared helper を変更する場合は全 caller を、sensitive sink を変更する場合は全 caller の逆引きを Task に含める。
- inventory の追加・削除・分類変更を予定する場合は、期待する差分を明記する。
- 区分 C では、独立監査を実装 Task に混ぜず、PR head に対する別工程として置く。

承認後に Existing-gap が見つかった場合は既存 Task へ統合できる。対象差分による Regression も merge 前に直す。固定されていなかった既存規約は、それだけを理由に Existing-gap や blocker へ昇格させず、Scope revision の明示再承認または別 Issue とする。New-requirement や Optional-hardening を黙って完了条件へ追加してはならない。

## 3. 実装

1. fix は failing test / contract / scenario を先に確認する。
2. guard は、その層が所有する sensitive sink より前に置く。server の拒否は client の外部送信禁止を満たす代替にならない。
3. `Ok(())` や boolean が「利用可能」「延期」「同意待ち」「再試行中」を兼ねる場合は、呼出元が誤って続行できない型付き outcome を使う。
4. global / aggregate 処理では、各対象の検証済み状態を失わず、ある対象の成功で未検証の別対象を有効化しない。
5. 入口、sink、shared helper、状態遷移が増減したら inventory / transition と tests を同じ変更で更新する。
6. 既存 test を削除・弱体化して green にしない。

## 4. PR と検証

PR 本文には次を記録する。

- 対象 Issue、Scope revision、リスク区分
- AC / INVAR ごとの実装箇所と test 名
- inventory の基準、変更前後、追加・削除・分類変更、未分類件数
- shared helper と sensitive sink の全 caller 確認方法
- 状態遷移の positive / negative test
- failing-before / passing-after の証跡
- `REFACTORING.md` の path 別必須 validation と、未実行項目の理由

全体 test 件数や CI green は必要条件だが、AC / INVAR の網羅性を単独では証明しない。

区分 C の PR は自動 Close 文言を使わず `Refs #...` とする。ユーザーが CI 後の自動 merge を承認済みでも、merge 条件は `必須 CI 成功 + PR head の独立監査 PASS` とする。監査後に対象コードが変わった場合は、変更 delta を再監査する。

## 5. 独立監査

区分 C は必須、区分 B は親 Issue、再Open、shared guard を含む場合に実施する。独立とは、実装時の結論や progress 記録を前提にせず、別の担当または別コンテキストが固定 AC / INVAR と対象 commit から再構築することをいう。

監査は次の順で行う。

1. 対象 commit と Scope revision を固定する。
2. 登録点から全入口を再生成し、`入口 -> helper -> sink` を確認する。
3. 各 sensitive sink から caller を逆引きし、inventory との差を確認する。
4. すべての早期 return、fallback、retry、cache、background、global apply を状態遷移表と照合する。
5. AC / INVAR ごとの実装・test・実行結果を確認する。
6. inventory の `未分類` と `不適合` を別々に数える。
7. findings を Existing-gap / Regression / New-requirement / Optional-hardening に分類する。

監査結果は `PASS / FAIL / INCONCLUSIVE` のいずれかとし、少なくとも次を記録する。

```md
- 対象 commit:
- Scope revision:
- リスク区分:
- inventory: 合計 / 適合 / 不適合 / 未分類
- AC / INVAR evidence:
- 実行した validation:
- blocker:
- non-blocker とした事項:
- 判定:
```

### Blocker の条件

次の四つをすべて満たすものだけを Close blocker とする。

1. 固定済み `AC-*` / `INVAR-*` に紐づくか、対象差分が新たに発生させた Regression である。
2. 固定 inventory の入口、または対象差分の caller / sink 逆引きで見つかった影響入口から到達できる。
3. 利用者影響、禁止された外部送信、データ損失、権限拡張、誤った永続 mutation のいずれかがある。
4. symbol / call path / failing test などの具体的証拠がある。

将来 route、未登録機能、一般的な悪意 client、理想的な法的完全性、無関係な hardening は、固定 AC / INVAR に含まれない限り blocker にしない。

### 監査の停止条件

次を満たした時点で監査を終了する。

- 固定 AC / INVAR がすべて evidence に対応している。
- named inventory がすべて適合または明示した Non-goal に分類され、未分類が 0 である。
- 適用可能な transition がすべて test または根拠に対応している。
- blocker が 0 である。

「未知の bug が存在しないこと」は完了条件にしない。監査後に対象 surface へ変更がなければ、同じ全監査を繰り返さず前回 PASS を利用する。変更があれば delta と影響先だけを再監査する。

## 6. Merge、Close、Reopen

区分 C は次の順序を守る。

1. PR head の独立監査が PASS。
2. 必須 CI が成功。
3. 承認された運用に従って merge。
4. merge commit の対象 tree / surface を PR head と比較する。差分があれば、その delta と影響先を merge commit 上で再監査して `PASS` にする。
5. Issue の `Current status` と evidence を更新して Close。

親 Issue は、子 Issue が Closed であるだけでは Close しない。親の各 AC / INVAR に対し、所有する子 Issue、merge commit、監査結果を対応付け、親固有の条件も確認する。

blocker を発見した場合は、影響する最小の子 Issue と必要な祖先だけを Reopen する。完了済みの兄弟 Issueを一括 Reopen しない。残タスクは blocker を再現する transition と、必要最小の修正・回帰 test に限定する。

## Issue 本文の履歴管理

- `Current status` は常に一つだけ置き、現在判定をそこへ集約する。
- 完了条件 checkbox は canonical な AC / INVAR にだけ使う。
- 過去監査は日付・commit・`Superseded` または `当時の判定` を付けた progress 文書か Issue comment に残す。
- 「当時残っていた項目」を完了 checkbox に変えて現在条件と混在させない。
- 大きな監査結果を本文へ追記し続けず、repo 内 progress record を evidence としてリンクする。

## 今回の再発から固定する禁止事項

- command / route の総数だけで「全経路確認済み」としない。
- constructor 直後の test だけで startup 後の setter、retry、scheduler、global reapply を代表させない。
- shared helper の通常経路だけを読み、guard より前の早期 return を未確認のままにしない。
- 一つの対象の成功で global apply される別対象を、単一対象 test だけで安全と判定しない。
- server の 403 を、client が秘密値や本文を送信してよい根拠にしない。
- test 総数、Issue checkbox、自己監査だけを Close 根拠にしない。

## 利用する雛形

- 実装 Issue: `.github/ISSUE_TEMPLATE/engineering-change.yml`
- PR: `.github/PULL_REQUEST_TEMPLATE.md`
- 実装計画: `PLANS.md`
- path 別 validation: `REFACTORING.md`
- command の実行方法: `docs/runbooks/dev.md`
