import { type DesktopApi } from '@/lib/api';

import { cloneAuthorView, withDefaultAuthorView } from '../desktopMockModel';
import { type MockRuntime } from '../mockRuntime';

type ProfileSocialMock = Pick<
  DesktopApi,
  | 'getMyProfile'
  | 'setMyProfile'
  | 'followAuthor'
  | 'unfollowAuthor'
  | 'getAuthorSocialView'
  | 'muteAuthor'
  | 'unmuteAuthor'
  | 'blockAuthor'
  | 'unblockAuthor'
  | 'listSocialConnections'
>;

export function createProfileSocialMock(runtime: MockRuntime): ProfileSocialMock {
  const { options, authorSocialViews, listConnections } = runtime;

  return {
    async getMyProfile() {
      if (options?.myProfileError) {
        throw new Error(options.myProfileError);
      }
      return runtime.myProfile;
    },
    async setMyProfile(input) {
      const nextPictureAsset =
        input.clear_picture
          ? null
          : input.picture_upload
            ? {
                hash: `profile-avatar-${runtime.myProfile.updated_at + 1}`,
                mime: input.picture_upload.mime,
                bytes: input.picture_upload.byte_size,
                role: 'profile_avatar' as const,
              }
            : runtime.myProfile.picture_asset ?? null;
      runtime.myProfile = {
        ...runtime.myProfile,
        ...input,
        picture_asset: nextPictureAsset,
        updated_at: runtime.myProfile.updated_at + 1,
      };
      authorSocialViews[runtime.myProfile.pubkey] = withDefaultAuthorView(runtime.myProfile.pubkey, {
        name: runtime.myProfile.name ?? null,
        display_name: runtime.myProfile.display_name ?? null,
        about: runtime.myProfile.about ?? null,
        picture_asset: runtime.myProfile.picture_asset ?? null,
        updated_at: runtime.myProfile.updated_at,
      });
      return runtime.myProfile;
    },
    async followAuthor(pubkey) {
      const existing = withDefaultAuthorView(pubkey, authorSocialViews[pubkey]);
      const next = { ...existing, following: true, mutual: existing.followed_by };
      authorSocialViews[pubkey] = next;
      return cloneAuthorView(next);
    },
    async unfollowAuthor(pubkey) {
      const existing = withDefaultAuthorView(pubkey, authorSocialViews[pubkey]);
      const next = { ...existing, following: false, mutual: false };
      authorSocialViews[pubkey] = next;
      return cloneAuthorView(next);
    },
    async getAuthorSocialView(pubkey) {
      if (pubkey === runtime.myProfile.pubkey) {
        return cloneAuthorView(withDefaultAuthorView(runtime.myProfile.pubkey, {
          name: runtime.myProfile.name ?? null,
          display_name: runtime.myProfile.display_name ?? null,
          about: runtime.myProfile.about ?? null,
          picture_asset: runtime.myProfile.picture_asset ?? null,
          updated_at: runtime.myProfile.updated_at,
        }));
      }
      return cloneAuthorView(withDefaultAuthorView(pubkey, authorSocialViews[pubkey]));
    },
    async muteAuthor(pubkey) {
      const existing = withDefaultAuthorView(pubkey, authorSocialViews[pubkey]);
      const next = { ...existing, muted: true };
      authorSocialViews[pubkey] = next;
      return cloneAuthorView(next);
    },
    async unmuteAuthor(pubkey) {
      const existing = withDefaultAuthorView(pubkey, authorSocialViews[pubkey]);
      const next = { ...existing, muted: false };
      authorSocialViews[pubkey] = next;
      return cloneAuthorView(next);
    },
    async blockAuthor(pubkey) {
      const view = withDefaultAuthorView(pubkey, authorSocialViews[pubkey]);
      view.blocking = true;
      authorSocialViews[pubkey] = view;
      return cloneAuthorView(view);
    },
    async unblockAuthor(pubkey) {
      const view = withDefaultAuthorView(pubkey, authorSocialViews[pubkey]);
      view.blocking = false;
      authorSocialViews[pubkey] = view;
      return cloneAuthorView(view);
    },
    async listSocialConnections(kind) {
      return listConnections(kind);
    },
  };
}
