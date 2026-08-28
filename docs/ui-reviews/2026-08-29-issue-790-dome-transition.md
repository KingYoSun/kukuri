# Issue #790 Seamless Dome transition UI review

- PR: https://github.com/KingYoSun/kukuri/pull/821
- Preview: Storybook `Extended/MetaverseRoomPanel/ReadyNorthTransition`
- Summary: active方向だけに15 mのconnection zoneとneighbor Domeを描画し、loading/error/full等では中心線barrier、readyでは連続空間を表示する。移動中は環境値を補間し、画面切替を挟まずavatar transformをhandoffする。
- Review result: 既存の固定Dome操作、HUD、chatを維持し、境界状態をgeometryで判別できる構成を採用した。
- Exceptions: 実験機能のため旧metaverse schemaとの後方互換表示は提供しない。
- Validation: Storybook build、Vitestのboundary/center-crossing/handoff、desktop typecheck、deterministic harness scenarioを実行する。
