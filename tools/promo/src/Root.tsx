import { Composition } from 'remotion';

import { PromoStill, parsePromoStillProps } from './compositions/PromoStill';
import { SceneClip } from './compositions/SceneClip';
import { SceneStill } from './compositions/SceneStill';
import { DEFAULT_FPS, clipDurationInFrames, parseSceneProps } from './props';

/**
 * 寸法と長さは props の manifest から決める。composition 側に固定値を持たせると、
 * 縮小された録画や長さの合わない clip を、そのまま成功として出力してしまう。
 */
const calculateMetadata = ({ props }: { props: Record<string, unknown> }) => {
  const parsed = parseSceneProps(props);
  return {
    width: parsed.manifest.viewport.width,
    height: parsed.manifest.viewport.height,
    fps: parsed.fps,
    durationInFrames: clipDurationInFrames(parsed),
  };
};

export function RemotionRoot() {
  return (
    <>
      <Composition
        id='SceneClip'
        component={SceneClip}
        // defaultProps は Studio で開いたときの見本。render では必ず --props で上書きする。
        defaultProps={{}}
        calculateMetadata={calculateMetadata}
        width={1600}
        height={1000}
        fps={DEFAULT_FPS}
        durationInFrames={1}
      />
      <Composition
        id='PromoStill'
        component={PromoStill}
        defaultProps={{}}
        // 媒体ごとの寸法は preset (tools/promo/presets/) が props で渡す。
        calculateMetadata={({ props }: { props: Record<string, unknown> }) => {
          const parsed = parsePromoStillProps(props);
          return { width: parsed.width, height: parsed.height, fps: DEFAULT_FPS, durationInFrames: 1 };
        }}
        width={1270}
        height={760}
        fps={DEFAULT_FPS}
        durationInFrames={1}
      />
      <Composition
        id='SceneStill'
        component={SceneStill}
        defaultProps={{}}
        calculateMetadata={calculateMetadata}
        width={1600}
        height={1000}
        fps={DEFAULT_FPS}
        durationInFrames={1}
      />
    </>
  );
}
