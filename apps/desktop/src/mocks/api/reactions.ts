import {
  type CustomReactionAssetView,
  type CustomReactionCropRect,
  type DesktopApi,
} from '@/lib/api';

import {
  normalizedReactionKey,
  pushRecentReaction,
  reactionStateForPost,
  withSocialPostDefaults,
} from '../desktopMockModel';
import { type MockRuntime } from '../mockRuntime';

type ReactionsMock = Pick<
  DesktopApi,
  | 'toggleReaction'
  | 'listMyCustomReactionAssets'
  | 'listRecentReactions'
  | 'createCustomReactionAsset'
  | 'listBookmarkedCustomReactions'
  | 'bookmarkCustomReaction'
  | 'removeBookmarkedCustomReaction'
>;

export function createReactionsMock(runtime: MockRuntime): ReactionsMock {
  const { postsByTopic, syncStatus, ownedCustomReactionAssets, bookmarkedCustomReactionAssets } =
    runtime;

  return {
    async toggleReaction(targetTopicId, targetObjectId, reactionKey) {
      const normalizedKey = normalizedReactionKey(reactionKey);
      const posts = postsByTopic[targetTopicId] ?? [];
      const index = posts.findIndex((post) => post.object_id === targetObjectId);
      if (index < 0) {
        throw new Error('reaction target was not found');
      }
      const post = withSocialPostDefaults(posts[index]);
      const myReactions = new Map(
        (post.my_reactions ?? []).map((reaction) => [reaction.normalized_reaction_key, reaction])
      );
      const summary = new Map(
        (post.reaction_summary ?? []).map((reaction) => [
          reaction.normalized_reaction_key,
          { ...reaction },
        ])
      );
      if (myReactions.has(normalizedKey)) {
        myReactions.delete(normalizedKey);
        const current = summary.get(normalizedKey);
        if (current) {
          const nextCount = current.count - 1;
          if (nextCount <= 0) {
            summary.delete(normalizedKey);
          } else {
            current.count = nextCount;
          }
        }
      } else {
        const keyView =
          reactionKey.kind === 'emoji'
            ? {
                reaction_key_kind: 'emoji',
                normalized_reaction_key: normalizedKey,
                emoji: reactionKey.emoji.trim(),
                custom_asset: null,
              }
            : {
                reaction_key_kind: 'custom_asset',
                normalized_reaction_key: normalizedKey,
                emoji: null,
                custom_asset: { ...reactionKey.asset },
              };
        myReactions.set(normalizedKey, keyView);
        const current = summary.get(normalizedKey);
        summary.set(normalizedKey, {
          ...(current ?? keyView),
          count: (current?.count ?? 0) + 1,
        });
      }
      const nextPost = withSocialPostDefaults({
        ...post,
        reaction_summary: Array.from(summary.values()),
        my_reactions: Array.from(myReactions.values()),
      });
      postsByTopic[targetTopicId] = posts.map((candidate) =>
        candidate.object_id === targetObjectId ? nextPost : candidate
      );
      runtime.recentReactions = pushRecentReaction(runtime.recentReactions, reactionKey, Date.now());
      return reactionStateForPost(nextPost);
    },
    async listMyCustomReactionAssets() {
      return ownedCustomReactionAssets.map((asset) => ({ ...asset }));
    },
    async listRecentReactions(limit = 8) {
      return runtime.recentReactions.slice(0, limit).map((reaction) => ({
        ...reaction,
        custom_asset: reaction.custom_asset ? { ...reaction.custom_asset } : null,
      }));
    },
    async createCustomReactionAsset(upload, cropRect: CustomReactionCropRect, searchKey: string) {
      void upload;
      void cropRect;
      runtime.sequence += 1;
      const asset: CustomReactionAssetView = {
        asset_id: `asset-${runtime.sequence}`,
        owner_pubkey: syncStatus.local_author_pubkey,
        blob_hash: `blob-${runtime.sequence}`,
        search_key: searchKey.trim() || `asset-${runtime.sequence}`,
        mime: 'image/png',
        bytes: 128,
        width: 128,
        height: 128,
      };
      ownedCustomReactionAssets.unshift(asset);
      return { ...asset };
    },
    async listBookmarkedCustomReactions() {
      return bookmarkedCustomReactionAssets.map((asset) => ({ ...asset }));
    },
    async bookmarkCustomReaction(asset) {
      const existing = bookmarkedCustomReactionAssets.find(
        (candidate) => candidate.asset_id === asset.asset_id
      );
      if (existing) {
        return { ...existing };
      }
      const bookmarked = { ...asset };
      bookmarkedCustomReactionAssets.unshift(bookmarked);
      return bookmarked;
    },
    async removeBookmarkedCustomReaction(assetId) {
      const index = bookmarkedCustomReactionAssets.findIndex((asset) => asset.asset_id === assetId);
      if (index >= 0) {
        bookmarkedCustomReactionAssets.splice(index, 1);
      }
    },
  };
}
