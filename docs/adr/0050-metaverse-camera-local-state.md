# ADR 0050: Metaverse追従カメラのlocal state

## Status

Accepted（#1021、2026-09-14）

## Decision

アバターの描画位置を読み取る追従カメラを追加する。入力のユーザー向け契約は[DESIGN](../../DESIGN.md)を正本とし、カメラ姿勢をavatar transformやDomeSessionInputへ追加しない。

### Feature Data Classification

- Feature 名: Metaverse追従カメラとPointer Lockによるアバター操作
- Durable / Transient: Transient。unmount／再起動時に固定と姿勢を破棄する。
- Canonical Source: ローカル描画groupの位置、camera refのyaw／pitch／zoom、document.pointerLockElement
- Replicated?: いいえ。avatarの既存移動・chat等の通信契約は維持する。
- Rebuildable From: 既存spawn／handoff transform、読み込み済みavatarの描画bounds、canvas寸法
- Public Replica / Private Replica / Local Only: Local Only
- Gossip Hint 必要有無: 不要
- Blob 必要有無: 追加不要。既存avatarモデルの読み込みだけを使用する。
- SQLite projection 必要有無: 不要
- 必須 contract: 初期／reset投影・追従、camera操作のdomain mutationなし、UI focus／IME／非active時の入力停止、所有canvas限定の固定解除
- 必須 scenario: frontendのcamera・input・room view testと実Canvasのbrowser flow。backend scenario追加は不要。Windows WebView2／Ubuntu24 WebKitGTKの実入力確認を行う。

## Consequences

カメラ変更のためにホスト権限・physics・admission・Dome transitionの契約を変更しない。Pointer Lockを取得できないWebViewでは理由と明示再試行を示し、視点調整ボタンとfocus中のkeyboard移動を利用できるようにする。既存のネットワークkeep-aliveをcamera testで停止させない。
