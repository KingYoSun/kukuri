import { describe, it, expect, beforeEach } from 'vitest';
import { mapUserProfileToUser } from '@/lib/profile/profileMapper';
import type { UserProfile } from '@/lib/api/tauri';
import { useAuthStore } from '@/stores';

describe('mapUserProfileToUser', () => {
  beforeEach(() => {
    useAuthStore.setState({
      isAuthenticated: false,
      currentUser: null,
      privateKey: null,
    });
  });

  it('UserProfileをUser型に変換する', () => {
    const profile: UserProfile = {
      npub: 'npub1example',
      pubkey: 'pubkey123',
      name: 'alice',
      display_name: 'Alice',
      about: 'hello',
      picture: 'https://example.com/avatar.png',
      banner: null,
      website: 'https://alice.example',
      nip05: 'alice@example.com',
    };

    const result = mapUserProfileToUser(profile);

    expect(result.id).toBe('pubkey123');
    expect(result.pubkey).toBe('pubkey123');
    expect(result.npub).toBe('npub1example');
    expect(result.displayName).toBe('Alice');
    expect(result.name).toBe('alice');
    expect(result.about).toBe('hello');
    expect(result.picture).toBe('https://example.com/avatar.png');
    expect(result.nip05).toBe('alice@example.com');
  });

  it('現在のユーザー情報を優先して適用する', () => {
    useAuthStore.setState({
      isAuthenticated: true,
      currentUser: {
        id: 'pubkey123',
        pubkey: 'pubkey123',
        npub: 'npub1example',
        name: 'current',
        displayName: 'Current Display',
        picture: 'https://example.com/current.png',
        about: 'from auth store',
        nip05: 'current@example.com',
        avatar: null,
      },
      privateKey: 'priv',
    });

    const profile: UserProfile = {
      npub: 'npub1example',
      pubkey: 'pubkey123',
      name: 'alice',
      display_name: 'Alice',
      about: 'hello',
      picture: null,
      banner: null,
      website: null,
      nip05: null,
    };

    const result = mapUserProfileToUser(profile);

    expect(result.displayName).toBe('Current Display');
    expect(result.name).toBe('current');
    expect(result.picture).toBe('https://example.com/current.png');
    expect(result.about).toBe('from auth store');
    expect(result.nip05).toBe('current@example.com');
  });
});
