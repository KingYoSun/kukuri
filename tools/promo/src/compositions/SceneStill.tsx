import { AbsoluteFill, Img, staticFile, useVideoConfig } from 'remotion';

import { Overlay } from '../components/Overlay';
import { parseSceneProps } from '../props';

/**
 * 撮影済み静止画 (PNG) を入力に、媒体別の静止画を出力する composition。
 */
export function SceneStill(input: Record<string, unknown>) {
  const props = parseSceneProps(input);
  const { width, height } = useVideoConfig();
  const still = props.manifest.files.still;

  if (!still) {
    throw new Error(
      `promo: scene ${props.manifest.sceneId}/${props.manifest.cutId} に still が無い`
    );
  }

  return (
    <AbsoluteFill style={{ backgroundColor: '#121212' }}>
      <Img src={staticFile(still)} style={{ width: '100%', height: '100%', objectFit: 'contain' }} />
      <Overlay
        manifest={props.manifest}
        caption={props.caption}
        demoBadge={props.demoBadge}
        width={width}
        height={height}
      />
    </AbsoluteFill>
  );
}
