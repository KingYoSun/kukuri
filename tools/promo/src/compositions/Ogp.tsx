import { AbsoluteFill, Img, staticFile } from 'remotion';

/**
 * OGP 画像 (1200×630)。LP の共有用。
 *
 * 画面は撮影済みの原素材を使い、アプリの画面を Remotion 内に作り直さない。
 * brief の方針どおり、OGP には Dome を入れない。
 */

export type OgpProps = {
  still: string;
  title: string;
  subtitle: string;
  demoLabel: string;
};

export function parseOgpProps(input: Record<string, unknown>): OgpProps {
  for (const key of ['still', 'title', 'subtitle', 'demoLabel']) {
    if (typeof input[key] !== 'string' || (input[key] as string).length === 0) {
      throw new Error(`ogp props: ${key} が無い`);
    }
  }
  return input as unknown as OgpProps;
}

export function Ogp(input: Record<string, unknown>) {
  const props = parseOgpProps(input);
  return (
    <AbsoluteFill
      style={{
        backgroundColor: '#f6f4f1',
        fontFamily: 'system-ui, sans-serif',
        display: 'flex',
        flexDirection: 'row',
      }}
    >
      <div
        style={{
          width: 470,
          padding: '64px 40px 56px 64px',
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'space-between',
        }}
      >
        <div style={{ fontSize: 34, fontWeight: 800, color: '#d77d45', letterSpacing: '0.02em' }}>
          kukuri
        </div>
        <div>
          <div
            style={{
              fontSize: 40,
              fontWeight: 800,
              lineHeight: 1.3,
              color: '#1c1a17',
              // 日本語は語の途中で折り返さないよう、title の改行 (\n) で区切る。
              whiteSpace: 'pre-line',
            }}
          >
            {props.title}
          </div>
          <div style={{ marginTop: 20, fontSize: 22, lineHeight: 1.5, color: '#4a4540' }}>
            {props.subtitle}
          </div>
        </div>
        <div style={{ fontSize: 18, color: '#6b645c' }}>Builder Preview · Windows / Linux</div>
      </div>
      <div style={{ position: 'relative', flex: 1, overflow: 'hidden' }}>
        <Img
          src={staticFile(props.still)}
          style={{
            position: 'absolute',
            top: 56,
            left: 0,
            height: 518,
            borderRadius: 18,
            boxShadow: '0 18px 48px rgba(28, 26, 23, 0.28)',
          }}
        />
        <div
          style={{
            position: 'absolute',
            right: 24,
            top: 24,
            backgroundColor: '#d77d45',
            color: '#20160e',
            fontSize: 18,
            fontWeight: 700,
            padding: '6px 14px',
            borderRadius: 8,
          }}
        >
          {props.demoLabel}
        </div>
      </div>
    </AbsoluteFill>
  );
}
