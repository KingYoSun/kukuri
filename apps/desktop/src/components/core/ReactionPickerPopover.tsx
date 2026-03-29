import { useState } from 'react';

import { Search, SmilePlus } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import type {
  CustomReactionAssetView,
  ReactionKeyInput,
  ReactionKeyView,
  RecentReactionView,
} from '@/lib/api';

import { type PostCardView } from './types';

type ReactionPickerPopoverProps = {
  post: PostCardView['post'];
  recentReactions?: RecentReactionView[];
  assets?: CustomReactionAssetView[];
  mediaObjectUrls?: Record<string, string | null>;
  onToggleReaction?: (post: PostCardView['post'], reactionKey: ReactionKeyInput) => void;
};

function reactionKeyInputFromView(reaction: ReactionKeyView): ReactionKeyInput | null {
  if (reaction.reaction_key_kind === 'emoji' && reaction.emoji?.trim()) {
    return { kind: 'emoji', emoji: reaction.emoji };
  }
  if (reaction.reaction_key_kind === 'custom_asset' && reaction.custom_asset) {
    return { kind: 'custom_asset', asset: reaction.custom_asset };
  }
  return null;
}

function assetSearchLabel(asset: CustomReactionAssetView): string {
  return asset.search_key.trim() || asset.asset_id;
}

function looksLikeEmojiQuery(query: string): boolean {
  return /\p{Extended_Pictographic}/u.test(query);
}

export function ReactionPickerPopover({
  post,
  recentReactions = [],
  assets = [],
  mediaObjectUrls = {},
  onToggleReaction,
}: ReactionPickerPopoverProps) {
  const { t } = useTranslation('common');
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const trimmedQuery = query.trim();
  const normalizedQuery = trimmedQuery.toLowerCase();
  const filteredAssets = normalizedQuery
    ? assets.filter((asset) =>
        [asset.search_key, asset.asset_id].some((value) => value.toLowerCase().includes(normalizedQuery))
      )
    : [];
  const canUseEmojiQuery = trimmedQuery.length > 0 && looksLikeEmojiQuery(trimmedQuery);

  const handleSelect = (reactionKey: ReactionKeyInput) => {
    onToggleReaction?.(post, reactionKey);
    setOpen(false);
    setQuery('');
  };

  return (
    <Popover
      open={open}
      onOpenChange={(nextOpen) => {
        setOpen(nextOpen);
        if (!nextOpen) {
          setQuery('');
        }
      }}
    >
      <PopoverTrigger asChild>
        <Button
          variant='secondary'
          size='icon'
          className='post-action-button'
          type='button'
          aria-label={t('actions.react')}
          disabled={!onToggleReaction}
        >
          <SmilePlus className='size-4' aria-hidden='true' />
        </Button>
      </PopoverTrigger>
      <PopoverContent align='end' className='post-action-popover post-reaction-popover'>
        <div className='post-reaction-search'>
          <Search className='post-reaction-search-icon size-4' aria-hidden='true' />
          <Input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t('reactions.searchPlaceholder')}
            aria-label={t('reactions.searchPlaceholder')}
          />
        </div>

        {recentReactions.length > 0 ? (
          <section className='post-reaction-section'>
            <p className='post-reaction-section-title'>{t('reactions.recent')}</p>
            <div className='post-reaction-picker-grid'>
              {recentReactions.map((reaction) => {
                const reactionKey = reactionKeyInputFromView(reaction);
                const asset = reaction.custom_asset ?? null;
                const previewUrl =
                  asset && typeof mediaObjectUrls[asset.blob_hash] === 'string'
                    ? mediaObjectUrls[asset.blob_hash]
                    : null;
                const label = asset ? assetSearchLabel(asset) : (reaction.emoji ?? reaction.normalized_reaction_key);
                return (
                  <button
                    key={reaction.normalized_reaction_key}
                    className='post-reaction-picker-button post-reaction-picker-button-detailed'
                    type='button'
                    onClick={() => {
                      if (reactionKey) {
                        handleSelect(reactionKey);
                      }
                    }}
                    disabled={!reactionKey}
                    title={label}
                  >
                    {previewUrl ? (
                      <img className='post-reaction-chip-image' src={previewUrl} alt={label} />
                    ) : null}
                    <span className='post-reaction-picker-label'>{reaction.emoji ?? label}</span>
                  </button>
                );
              })}
            </div>
          </section>
        ) : null}

        {trimmedQuery ? (
          <section className='post-reaction-section'>
            <p className='post-reaction-section-title'>{t('reactions.customResults')}</p>
            <div className='post-reaction-picker-grid'>
              {canUseEmojiQuery ? (
                <button
                  className='post-reaction-picker-button post-reaction-picker-button-detailed'
                  type='button'
                  onClick={() => handleSelect({ kind: 'emoji', emoji: trimmedQuery })}
                  title={trimmedQuery}
                >
                  <span className='post-reaction-picker-label'>{trimmedQuery}</span>
                </button>
              ) : null}
              {filteredAssets.map((asset) => {
                const previewUrl =
                  typeof mediaObjectUrls[asset.blob_hash] === 'string'
                    ? mediaObjectUrls[asset.blob_hash]
                    : null;
                const label = assetSearchLabel(asset);
                return (
                  <button
                    key={asset.asset_id}
                    className='post-reaction-picker-button post-reaction-picker-button-detailed'
                    type='button'
                    onClick={() => handleSelect({ kind: 'custom_asset', asset })}
                    title={label}
                  >
                    {previewUrl ? (
                      <img className='post-reaction-chip-image' src={previewUrl} alt={label} />
                    ) : (
                      <span className='post-reaction-picker-fallback'>{label.slice(0, 2)}</span>
                    )}
                    <span className='post-reaction-picker-copy'>
                      <strong>{label}</strong>
                      <small>{asset.asset_id}</small>
                    </span>
                  </button>
                );
              })}
            </div>
            {!canUseEmojiQuery && filteredAssets.length === 0 ? (
              <p className='post-reaction-picker-empty'>{t('reactions.noResults')}</p>
            ) : null}
          </section>
        ) : recentReactions.length === 0 ? (
          <p className='post-reaction-picker-empty'>{t('reactions.searchHint')}</p>
        ) : null}
      </PopoverContent>
    </Popover>
  );
}
