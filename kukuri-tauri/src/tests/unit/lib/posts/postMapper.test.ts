import { beforeEach, describe, expect, it, vi } from 'vitest';
import { mapPostResponseToDomain } from '@/lib/posts/postMapper';
import { TauriApi } from '@/lib/api/tauri';

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    getUserProfileByPubkey: vi.fn(),
    getUserProfile: vi.fn(),
  },
}));

vi.mock('@/lib/utils/nostr', () => ({
  pubkeyToNpub: vi.fn(async (pubkey: string) => `npub1${pubkey.slice(0, 10)}`),
}));

const buildApiPost = (overrides: Partial<Record<string, unknown>> = {}) => ({
  id: 'post-1',
  content: 'hello',
  author_pubkey: '0830776847a7987c050fe9e6d466c155335a01d17c1844877e4b1fdc17bc446a',
  author_npub: 'npub1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq',
  topic_id: 'kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0',
  created_at: 1_706_000_000,
  likes: 0,
  boosts: 0,
  replies: 0,
  is_synced: true,
  ...overrides,
});

describe('postMapper', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('author pubkey でプロフィール取得できる場合は表示名を反映する', async () => {
    vi.mocked(TauriApi.getUserProfileByPubkey).mockResolvedValue({
      npub: 'npub1alice',
      pubkey: '0830776847a7987c050fe9e6d466c155335a01d17c1844877e4b1fdc17bc446a',
      name: 'alice',
      display_name: 'Alice',
      about: 'profile',
      picture: 'https://example.com/avatar.png',
      banner: null,
      website: null,
      nip05: 'alice@example.com',
      is_profile_public: true,
      show_online_status: false,
    });
    vi.mocked(TauriApi.getUserProfile).mockResolvedValue(null);

    const mapped = await mapPostResponseToDomain(buildApiPost());

    expect(mapped.author.displayName).toBe('Alice');
    expect(mapped.author.name).toBe('alice');
    expect(mapped.author.pubkey).toBe(
      '0830776847a7987c050fe9e6d466c155335a01d17c1844877e4b1fdc17bc446a',
    );
  });

  it('プロフィール未取得時は短縮IDフォールバックを返す', async () => {
    vi.mocked(TauriApi.getUserProfileByPubkey).mockResolvedValue(null);
    vi.mocked(TauriApi.getUserProfile).mockResolvedValue(null);

    const mapped = await mapPostResponseToDomain(
      buildApiPost({
        author_pubkey: '0026537e52ee230f079a41b94d1ae0d73bf4dc8a783f3275562efe033298c945',
        author_npub: 'npub1abcdefghijklmnopqrstuvwxyzaaaaa',
      }),
    );

    expect(mapped.author.displayName).toBe('npub1abc...aaaa');
    expect(mapped.author.name).toBe('npub1abc...aaaa');
  });
});
