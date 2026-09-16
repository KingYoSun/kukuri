import type { PostView } from '@/lib/api';

/// メディア(blob object URL・非対応動画)(WP-H6 PR3 のドメインスライス)。
export type MediaSliceState = {
  mediaObjectUrls: Record<string, string | null>;
  unsupportedVideoManifests: Record<string, true>;
  // #858: 成人向け表現の表示設定(既定 OFF)。canonical source は Rust 側の
  // ローカル JSON で、ここは表示・プリフェッチ判定用の mirror。
  adultContentEnabled: boolean;
  // #1052: 「見つける」Column が表示中の、ローカル解決済み(署名付き)投稿。
  // メディアのプリフェッチ対象と成人向け取得ゲートの判定に使う一時状態で、
  // 結果の失効・Column の終了で空になる(永続化しない)。
  communityIndexResolvedPosts: PostView[];
};

export function createInitialMediaSlice(): MediaSliceState {
  return {
    mediaObjectUrls: {},
    unsupportedVideoManifests: {},
    adultContentEnabled: false,
    communityIndexResolvedPosts: [],
  };
}
