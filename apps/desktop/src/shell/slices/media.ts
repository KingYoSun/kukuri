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
  // #1055: 設定済み Community Node の content advisory が付いた添付 blob hash のうち、
  // 表示設定 OFF でゲート中のもの。Rust 側取得ゲート(`blob_media_payload`)の client 側 mirror で、
  // プリフェッチの起点をどこに持つ表示経路からも要求しないために使う(ADR 0046 §6.2)。
  // 一時状態であり永続化しない(ADR 0028 §8.10)。
  advisoryGatedMediaHashes: string[];
};

export function createInitialMediaSlice(): MediaSliceState {
  return {
    mediaObjectUrls: {},
    unsupportedVideoManifests: {},
    adultContentEnabled: false,
    communityIndexResolvedPosts: [],
    advisoryGatedMediaHashes: [],
  };
}
