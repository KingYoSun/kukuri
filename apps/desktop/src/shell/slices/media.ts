/// メディア(blob object URL・非対応動画)(WP-H6 PR3 のドメインスライス)。
export type MediaSliceState = {
  mediaObjectUrls: Record<string, string | null>;
  unsupportedVideoManifests: Record<string, true>;
  // #858: 成人向け表現の表示設定(既定 OFF)。canonical source は Rust 側の
  // ローカル JSON で、ここは表示・プリフェッチ判定用の mirror。
  adultContentEnabled: boolean;
};

export function createInitialMediaSlice(): MediaSliceState {
  return {
    mediaObjectUrls: {},
    unsupportedVideoManifests: {},
    adultContentEnabled: false,
  };
}
