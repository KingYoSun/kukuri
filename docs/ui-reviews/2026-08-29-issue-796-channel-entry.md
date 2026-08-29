# Issue #796 Channel entry Dome UI review

Preview: [Channel entry Dome selection](./2026-08-29-issue-796-channel-entry.png)

## Shneiderman eight golden rules

- Consistency: 既存のcomposer、Select、Button、room cardを再利用した。
- Shortcuts: Context open時は優先候補へ自動admissionし、一覧操作を必須にしない。
- Feedback: resolvingとadmittingをNoticeで表示し、host停止はcard上の`Host unavailable`で示す。
- Closure: host snapshotでlocal avatarを確認した時点だけjoined stateへ進む。
- Error prevention: offline hostのJoinをdisabledにし、channel owner以外は設定をread-only表示にする。
- Reversal: Channel entry Domeは`No configured Dome`へ戻して解除できる。
- User control: 自動候補が使えない場合も利用可能なDome一覧から明示選択できる。
- Memory load: entry候補の優先順位とlast visitはsystem側で保持し、利用者にInstance IDの記憶を要求しない。

Keyboard操作ではSelect、Save、Joinの順に到達でき、設定名はvisible labelを持つ。色だけに依存する状態表現は追加していない。
