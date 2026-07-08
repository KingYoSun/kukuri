/// メディア(blob object URL・非対応動画)(WP-H6 PR3 のドメインスライス)。
export type MediaSliceState = {
  mediaObjectUrls: Record<string, string | null>;
  unsupportedVideoManifests: Record<string, true>;
};

export function createInitialMediaSlice(): MediaSliceState {
  return {
    mediaObjectUrls: {},
    unsupportedVideoManifests: {},
  };
}
