# Issue #793 Dome prop/layout retention UI review

## 対象

Metaverse roomのHosting panelと3D scene。persistent propはorange、guest propはblueで区別し、ownerだけにpersistent mutationとlayout保存を表示する。active session参加者には5分TTLのguest prop追加とsnapshot resyncを表示する。

## Shneiderman checklist

- 一貫性: 既存の`Card`、`Button`、`Notice`、Hosting state表示を再利用し、新操作を同じpanelへ集約した。
- フィードバック: 操作中は`aria-busy`とdisabledを適用し、commitのcommitted/no-op、resync件数、失敗理由をpanel内へ表示する。
- 取り消し: persistent mutationはsession内だけで、明示保存までは再始動で破棄できる。durable保存後も直近3 revisionをrollback pinとして保持する。
- エラー防止: owner以外にpersistent mutation/layout保存を表示せず、host停止・移管中はsession mutationをdisableする。30秒rate limitはbackendでも強制する。
- 状態可視性: host state、epoch、session、participants、sleep、manifest revision、対象cache上限を常時表示し、persistent/guestを色で区別する。
- アクセシビリティ: native button、既存focus style、テキストラベルを維持し、色だけに依存せず操作名にもprop種別を含める。animationは追加していない。

## 検証

Vitestでscene model、session hook、room panelを確認し、desktop typecheck/lintとworkspace testをCI validationに含める。
