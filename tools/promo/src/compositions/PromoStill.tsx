import { useCallback, useEffect, useRef, useState } from 'react';
import { AbsoluteFill, Img, continueRender, delayRender, staticFile } from 'remotion';

import { theme, typeScale } from '../components/typography';

/**
 * 媒体別の静止画 (#1041)。Product Hunt・note・X・OGP を、同じ部品と props で作る。
 *
 * - 画面は撮影済みの原素材をそのまま使い、切り抜き (crop) と拡大だけを行う。UI を作り直さない
 * - 文字が枠からあふれたら render を失敗させ、文字切れの画像を完成扱いにしない (TR-2)
 * - Dome を含む素材はここでは扱わない。Dome の予告は LP の SceneStill だけで出す
 */

export type Crop = { x: number; y: number; width: number; height: number };

export type PromoStillProps = {
  width: number;
  height: number;
  /** split: 文字と画面を左右に並べる / header: 見出し中心 (OGP・note の見出し) / icon: アイコンだけ */
  layout: 'split' | 'header' | 'icon';
  locale: 'ja' | 'en';
  eyebrow?: string;
  headline?: string;
  subhead?: string;
  footer?: string;
  demoLabel?: string;
  screen?: { still: string; sourceWidth: number; sourceHeight: number; crop?: Crop };
  icon?: string;
};

export function parsePromoStillProps(input: Record<string, unknown>): PromoStillProps {
  const props = input as unknown as PromoStillProps;
  if (!(props.width > 0) || !(props.height > 0)) {
    throw new Error('promo still: width / height が無い');
  }
  if (props.layout === 'icon') {
    if (!props.icon) throw new Error('promo still: icon layout に icon が無い');
    return props;
  }
  if (!props.headline) throw new Error('promo still: headline が無い');
  if (!props.screen?.still) throw new Error('promo still: screen.still が無い');
  if (!props.demoLabel) throw new Error('promo still: 画面を載せる画像には demoLabel が要る');
  return props;
}

/** 画面の一部を、枠いっぱいに拡大して見せる。原素材の外側を見せないよう cover で合わせる。 */
function Screen({
  screen,
  frameWidth,
  frameHeight,
  demoLabel,
}: {
  screen: NonNullable<PromoStillProps['screen']>;
  frameWidth: number;
  frameHeight: number;
  demoLabel: string;
}) {
  const crop = screen.crop ?? { x: 0, y: 0, width: screen.sourceWidth, height: screen.sourceHeight };
  const scale = Math.max(frameWidth / crop.width, frameHeight / crop.height);
  const offsetX = (frameWidth - crop.width * scale) / 2 - crop.x * scale;
  const offsetY = (frameHeight - crop.height * scale) / 2 - crop.y * scale;
  return (
    <div
      style={{
        position: 'relative',
        width: frameWidth,
        height: frameHeight,
        overflow: 'hidden',
        borderRadius: 20,
        backgroundColor: '#121212',
        boxShadow: '0 22px 60px rgba(28, 26, 23, 0.30)',
      }}
    >
      <Img
        src={staticFile(screen.still)}
        style={{
          position: 'absolute',
          left: offsetX,
          top: offsetY,
          width: screen.sourceWidth * scale,
          height: screen.sourceHeight * scale,
          maxWidth: 'none',
        }}
      />
      <div
        style={{
          // 画面の見出しや投稿に重ならないよう、右下に置く。
          position: 'absolute',
          right: 16,
          bottom: 16,
          backgroundColor: theme.accent,
          color: theme.onAccent,
          fontWeight: 800,
          fontSize: Math.round(frameHeight * 0.042),
          padding: '6px 14px',
          borderRadius: 10,
        }}
      >
        {demoLabel}
      </div>
    </div>
  );
}

/** 枠からあふれた文字を検出し、render を失敗させる。 */
function useOverflowGuard() {
  const ref = useRef<HTMLDivElement>(null);
  const [handle] = useState(() => delayRender('promo still: 文字あふれの確認'));
  const [error, setError] = useState<string | null>(null);
  const check = useCallback(() => {
    const element = ref.current;
    if (!element) return;
    const overflow =
      element.scrollHeight > element.clientHeight + 1 || element.scrollWidth > element.clientWidth + 1;
    if (overflow) {
      setError(
        `promo still: 文字が枠からあふれている (${element.scrollWidth}x${element.scrollHeight} > ` +
          `${element.clientWidth}x${element.clientHeight})。文言を短くするか preset の寸法を見直す`
      );
      return;
    }
    continueRender(handle);
  }, [handle]);
  useEffect(() => {
    document.fonts.ready.then(check);
  }, [check]);
  if (error) {
    throw new Error(error);
  }
  return ref;
}

function TextBlock({ props, width, height }: { props: PromoStillProps; width: number; height: number }) {
  const ref = useOverflowGuard();
  const size = typeScale(props.width, props.layout);
  // 日本語は文節の切れ目で折り返し、語の途中（「起点／の」など）で改行しない。
  const wordBreak = props.locale === 'ja' ? 'auto-phrase' : 'normal';
  return (
    <div
      ref={ref}
      // word-break: auto-phrase は、要素の言語が日本語と分かっているときだけ文節で折り返す。
      lang={props.locale}
      style={{
        width,
        height,
        overflow: 'hidden',
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'space-between',
      }}
    >
      <div style={{ fontSize: size.brand, fontWeight: 800, color: theme.accent, letterSpacing: '0.02em' }}>
        kukuri
      </div>
      <div>
        {props.eyebrow ? (
          <div style={{ fontSize: size.eyebrow, fontWeight: 800, color: theme.accentStrong, marginBottom: size.eyebrow * 0.5 }}>
            {props.eyebrow}
          </div>
        ) : null}
        <div
          style={{
            fontSize: size.headline,
            fontWeight: 800,
            lineHeight: 1.28,
            color: theme.text,
            // 文言側の改行 (\n) は、見出しを意図した位置で区切るために使う。
            whiteSpace: 'pre-line',
            wordBreak,
          }}
        >
          {props.headline}
        </div>
        {props.subhead ? (
          <div style={{ marginTop: size.subhead * 0.9, fontSize: size.subhead, lineHeight: 1.55, color: theme.textMuted, whiteSpace: 'pre-line', wordBreak }}>
            {props.subhead}
          </div>
        ) : null}
      </div>
      <div style={{ fontSize: size.footer, color: theme.textSoft }}>{props.footer ?? ''}</div>
    </div>
  );
}

export function PromoStill(input: Record<string, unknown>) {
  const props = parsePromoStillProps(input);
  const { width, height } = props;

  if (props.layout === 'icon') {
    return (
      <AbsoluteFill style={{ backgroundColor: theme.bg, alignItems: 'center', justifyContent: 'center' }}>
        <Img src={staticFile(props.icon as string)} style={{ width: width * 0.86, height: height * 0.86 }} />
      </AbsoluteFill>
    );
  }

  const padding = Math.round(width * 0.05);
  const textWidth = Math.round(width * (props.layout === 'header' ? 0.36 : 0.37));
  const gap = Math.round(width * 0.035);
  const screenWidth = width - padding * 2 - textWidth - gap + (props.layout === 'header' ? padding : 0);
  const screenHeight = height - padding * 2;

  return (
    <AbsoluteFill
      style={{
        backgroundColor: theme.bg,
        fontFamily: theme.fontFamily,
        flexDirection: 'row',
        padding,
        paddingRight: props.layout === 'header' ? 0 : padding,
        gap,
      }}
    >
      <TextBlock props={props} width={textWidth} height={screenHeight} />
      <div style={{ display: 'flex', alignItems: 'center' }}>
        <Screen
          screen={props.screen as NonNullable<PromoStillProps['screen']>}
          frameWidth={screenWidth}
          frameHeight={screenHeight}
          demoLabel={props.demoLabel as string}
        />
      </div>
    </AbsoluteFill>
  );
}
