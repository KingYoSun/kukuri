import { AbsoluteFill, OffthreadVideo, staticFile, useVideoConfig } from 'remotion';

import { Overlay } from '../components/Overlay';
import { parseSceneProps } from '../props';

/**
 * 撮影済み動画 (webm) を入力に、媒体別の MP4 を出力する composition。
 * 実アプリを Remotion 内へ複製せず、取得済みメディアの編集だけを行う。
 */
export function SceneClip(input: Record<string, unknown>) {
  const props = parseSceneProps(input);
  const { width, height } = useVideoConfig();
  const video = props.manifest.files.video;

  if (!video) {
    // 欠落入力を黒画面の成功として出力しない。
    throw new Error(
      `promo: scene ${props.manifest.sceneId}/${props.manifest.cutId} に video が無い。` +
        `still だけの素材は SceneStill を使う`
    );
  }

  return (
    <AbsoluteFill style={{ backgroundColor: '#121212' }}>
      <OffthreadVideo src={staticFile(video)} />
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
