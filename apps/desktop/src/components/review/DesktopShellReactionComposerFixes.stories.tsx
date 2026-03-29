import type { Meta, StoryObj } from '@storybook/react-vite';
import { Lock, Plus, Repeat2, Search, Settings, SmilePlus } from 'lucide-react';

import { ComposerPanel } from '@/components/core/ComposerPanel';
import { PostCard } from '@/components/core/PostCard';
import { TimelineWorkspaceHeader } from '@/components/core/TimelineWorkspaceHeader';
import { TopicNavList } from '@/components/core/TopicNavList';
import { type PostCardView, type TopicDiagnosticSummary } from '@/components/core/types';
import { StatusBadge } from '@/components/StatusBadge';
import { ShellFrame } from '@/components/shell/ShellFrame';
import { ShellNavRail } from '@/components/shell/ShellNavRail';
import { ShellTopBar } from '@/components/shell/ShellTopBar';
import { type PrimarySection } from '@/components/shell/types';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import {
  STORY_ACTIVE_TOPIC,
  STORY_IMAGE_PREVIEW,
  STORY_VIDEO_POSTER_PREVIEW,
} from '@/components/storyFixtures';
import type { CustomReactionAssetView, RecentReactionView } from '@/lib/api';

const meta = {
  title: 'Review/DesktopShellReactionComposerFixes',
  parameters: {
    layout: 'fullscreen',
  },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

const PRIMARY_ITEMS: Array<{ id: PrimarySection; label: string }> = [
  { id: 'timeline', label: 'Timeline' },
  { id: 'live', label: 'Live' },
  { id: 'game', label: 'Game' },
  { id: 'profile', label: 'Profile' },
];

const TOPIC_ITEMS: TopicDiagnosticSummary[] = [
  {
    topic: STORY_ACTIVE_TOPIC,
    active: true,
    publicActive: false,
    removable: false,
    connectionLabel: 'joined',
    peerCount: 4,
    lastReceivedLabel: '14:30:21',
    channels: [
      {
        channelId: 'channel-core',
        label: 'Core Contributors Coordination',
        audienceKind: 'friend_plus',
        active: true,
      },
      {
        channelId: 'channel-review',
        label: 'Review Room',
        audienceKind: 'invite_only',
        active: false,
      },
    ],
  },
  {
    topic: 'kukuri:topic:relay',
    active: false,
    publicActive: false,
    removable: true,
    connectionLabel: 'relay-assisted',
    peerCount: 1,
    lastReceivedLabel: 'no events',
  },
];

const PARTY_PARROT_ASSET: CustomReactionAssetView = {
  asset_id: 'asset-party-parrot',
  owner_pubkey: 'b'.repeat(64),
  blob_hash: 'blob-party-parrot',
  search_key: 'party-parrot',
  mime: 'image/png',
  bytes: 128,
  width: 128,
  height: 128,
};

const SAVED_CAT_ASSET: CustomReactionAssetView = {
  asset_id: 'asset-saved-cat',
  owner_pubkey: 'c'.repeat(64),
  blob_hash: 'blob-saved-cat',
  search_key: 'saved-cat',
  mime: 'image/gif',
  bytes: 128,
  width: 128,
  height: 128,
};

const REACTION_MEDIA_URLS = {
  'blob-party-parrot':
    'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128"><rect width="128" height="128" rx="28" fill="%23ffd36e"/><circle cx="64" cy="64" r="42" fill="%23101923"/><circle cx="50" cy="54" r="8" fill="%23f6f1e8"/><circle cx="78" cy="54" r="8" fill="%23f6f1e8"/><path d="M40 84c14-18 34-18 48 0" fill="none" stroke="%2300b3a4" stroke-width="10" stroke-linecap="round"/></svg>',
  'blob-saved-cat':
    'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128"><rect width="128" height="128" rx="28" fill="%2300b3a4"/><path d="M30 48l16-22 18 18 18-18 16 22v42H30z" fill="%23f6f1e8"/><circle cx="52" cy="70" r="6" fill="%23101923"/><circle cx="76" cy="70" r="6" fill="%23101923"/><path d="M52 90c8 6 16 6 24 0" fill="none" stroke="%23f59d62" stroke-width="8" stroke-linecap="round"/></svg>',
};

const RECENT_REACTIONS: RecentReactionView[] = [
  {
    reaction_key_kind: 'emoji',
    normalized_reaction_key: 'emoji:🔥',
    emoji: '🔥',
    custom_asset: null,
    updated_at: 5,
  },
  {
    reaction_key_kind: 'emoji',
    normalized_reaction_key: 'emoji:👏',
    emoji: '👏',
    custom_asset: null,
    updated_at: 4,
  },
  {
    reaction_key_kind: 'custom_asset',
    normalized_reaction_key: 'custom_asset:asset-party-parrot',
    emoji: null,
    custom_asset: PARTY_PARROT_ASSET,
    updated_at: 3,
  },
  {
    reaction_key_kind: 'custom_asset',
    normalized_reaction_key: 'custom_asset:asset-saved-cat',
    emoji: null,
    custom_asset: SAVED_CAT_ASSET,
    updated_at: 2,
  },
];

const TIMELINE_POST: PostCardView = {
  post: {
    object_id: 'post-desktop-fixes',
    envelope_id: 'env-desktop-fixes',
    author_pubkey: 'a'.repeat(64),
    author_name: 'alice',
    author_display_name: 'Alice',
    following: false,
    followed_by: true,
    mutual: false,
    friend_of_friend: false,
    object_kind: 'post',
    content:
      'Reaction picking moved into a popover, repost actions are now readable, and the floating composer stays visible over the shell background.',
    content_status: 'Available',
    attachments: [],
    created_at: 4,
    reply_to: null,
    root_id: 'post-desktop-fixes',
    published_topic_id: STORY_ACTIVE_TOPIC,
    origin_topic_id: STORY_ACTIVE_TOPIC,
    repost_of: null,
    repost_commentary: null,
    is_threadable: true,
    channel_id: 'channel-core',
    audience_label: 'Core Contributors',
    reaction_summary: [
      {
        reaction_key_kind: 'emoji',
        normalized_reaction_key: 'emoji:👍',
        emoji: '👍',
        custom_asset: null,
        count: 12,
      },
      {
        reaction_key_kind: 'custom_asset',
        normalized_reaction_key: 'custom_asset:asset-party-parrot',
        emoji: null,
        custom_asset: PARTY_PARROT_ASSET,
        count: 3,
      },
    ],
    my_reactions: [
      {
        reaction_key_kind: 'emoji',
        normalized_reaction_key: 'emoji:👍',
        emoji: '👍',
        custom_asset: null,
      },
    ],
  },
  context: 'timeline',
  authorLabel: 'Alice',
  authorPicture: null,
  relationshipLabel: 'follows you',
  audienceChipLabel: 'Core Contributors',
  threadTargetId: 'post-desktop-fixes',
  canRepost: true,
  media: {
    objectId: 'post-desktop-fixes',
    kind: 'image',
    statusLabel: 'image ready',
    extraAttachmentCount: 0,
    state: 'ready',
    metaMime: 'image/png',
    metaBytesLabel: '144 KB',
    imagePreviewSrc: STORY_IMAGE_PREVIEW,
    videoPosterPreviewSrc: null,
    videoPlaybackSrc: null,
    videoUnsupportedOnClient: false,
  },
};

const SOURCE_PREVIEW: PostCardView = {
  post: {
    object_id: 'source-post-1',
    envelope_id: 'env-source-post-1',
    author_pubkey: 'd'.repeat(64),
    author_name: 'dana',
    author_display_name: 'Dana',
    following: false,
    followed_by: false,
    mutual: true,
    friend_of_friend: false,
    object_kind: 'post',
    content:
      'Reply and quote repost composer now show the full source post, including author, audience, and attached media.',
    content_status: 'Available',
    attachments: [],
    created_at: 3,
    reply_to: null,
    root_id: 'source-post-1',
    published_topic_id: STORY_ACTIVE_TOPIC,
    origin_topic_id: STORY_ACTIVE_TOPIC,
    repost_of: null,
    repost_commentary: null,
    is_threadable: true,
    channel_id: 'channel-core',
    audience_label: 'Imported',
    reaction_summary: [],
    my_reactions: [],
  },
  context: 'timeline',
  authorLabel: 'Dana',
  authorPicture: null,
  relationshipLabel: 'mutual',
  audienceChipLabel: 'Imported',
  threadTargetId: 'source-post-1',
  media: {
    objectId: 'source-post-1',
    kind: 'video',
    statusLabel: 'poster ready',
    extraAttachmentCount: 0,
    state: 'ready',
    metaMime: 'video/mp4',
    metaBytesLabel: '8.0 KB',
    imagePreviewSrc: null,
    videoPosterPreviewSrc: STORY_VIDEO_POSTER_PREVIEW,
    videoPlaybackSrc: null,
    videoUnsupportedOnClient: false,
  },
};

function ComposerPreviewPane() {
  return (
    <Card as='aside' className='shell-detail-pane shell-main-stack'>
      <div className='shell-pane-header'>
        <div>
          <p className='eyebrow'>Composer modal</p>
          <h3 className='shell-pane-heading'>Quote repost with source preview</h3>
          <p className='shell-pane-copy'>
            元投稿の author / audience / media を同一 surface で確認できる状態。
          </p>
        </div>
      </div>
      <ComposerPanel
        value='Adding commentary while keeping the source preview visible in the modal.'
        onChange={() => undefined}
        onSubmit={(event) => event.preventDefault()}
        attachmentInputKey={0}
        onAttachmentSelection={() => undefined}
        draftMediaItems={[]}
        onRemoveDraftAttachment={() => undefined}
        composerError={null}
        audienceLabel='Core Contributors'
        repostTarget={{
          content: SOURCE_PREVIEW.post.content,
          authorLabel: SOURCE_PREVIEW.authorLabel,
        }}
        sourcePreview={SOURCE_PREVIEW}
        onClearReply={() => undefined}
        onClearRepost={() => undefined}
        attachmentsDisabled
      />
    </Card>
  );
}

function ReactionPreviewButton({
  label,
  previewUrl,
}: {
  label: string;
  previewUrl?: string | null;
}) {
  return (
    <button
      className='post-reaction-picker-button post-reaction-picker-button-detailed'
      type='button'
      title={label}
    >
      {previewUrl ? <img className='post-reaction-chip-image' src={previewUrl} alt={label} /> : null}
      <span className='post-reaction-picker-label'>{label}</span>
    </button>
  );
}

function ActionPopoverPreviewPane() {
  return (
    <Card as='aside' className='shell-detail-pane shell-main-stack'>
      <div className='shell-pane-header'>
        <div>
          <p className='eyebrow'>Action overlays</p>
          <h3 className='shell-pane-heading'>Solid popover surfaces</h3>
          <p className='shell-pane-copy'>
            repost / reaction は透過せず、同じ panel token で可読性を維持する。
          </p>
        </div>
      </div>

      <section className='shell-main-stack'>
        <div className='post-actions'>
          <Button
            variant='secondary'
            size='icon'
            className='post-action-button'
            type='button'
            aria-label='React'
          >
            <SmilePlus className='size-4' aria-hidden='true' />
          </Button>
          <Button
            variant='secondary'
            size='icon'
            className='post-action-button'
            type='button'
            aria-label='Repost'
          >
            <Repeat2 className='size-4' aria-hidden='true' />
          </Button>
        </div>

        <div className='ui-popover-content panel post-action-popover'>
          <div className='post-action-popover-stack'>
            <Button variant='secondary' type='button'>
              Repost now
            </Button>
            <Button type='button'>Add quote</Button>
          </div>
        </div>

        <div className='ui-popover-content panel post-action-popover post-reaction-popover'>
          <div className='post-reaction-search'>
            <Search className='post-reaction-search-icon size-4' aria-hidden='true' />
            <Input
              value='party'
              onChange={() => undefined}
              placeholder='Search reactions'
              aria-label='Search reactions'
              readOnly
            />
          </div>

          <section className='post-reaction-section'>
            <p className='post-reaction-section-title'>Recent</p>
            <div className='post-reaction-picker-grid'>
              <ReactionPreviewButton label='🔥' />
              <ReactionPreviewButton label='👏' />
              <ReactionPreviewButton
                label='party-parrot'
                previewUrl={REACTION_MEDIA_URLS['blob-party-parrot']}
              />
              <ReactionPreviewButton
                label='saved-cat'
                previewUrl={REACTION_MEDIA_URLS['blob-saved-cat']}
              />
            </div>
          </section>

          <section className='post-reaction-section'>
            <p className='post-reaction-section-title'>Custom results</p>
            <div className='post-reaction-picker-grid'>
              <button
                className='post-reaction-picker-button post-reaction-picker-button-detailed'
                type='button'
                title='party-parrot'
              >
                <img
                  className='post-reaction-chip-image'
                  src={REACTION_MEDIA_URLS['blob-party-parrot']}
                  alt='party-parrot'
                />
                <span className='post-reaction-picker-copy'>
                  <strong>party-parrot</strong>
                  <small>{PARTY_PARROT_ASSET.asset_id}</small>
                </span>
              </button>
              <button
                className='post-reaction-picker-button post-reaction-picker-button-detailed'
                type='button'
                title='saved-cat'
              >
                <img
                  className='post-reaction-chip-image'
                  src={REACTION_MEDIA_URLS['blob-saved-cat']}
                  alt='saved-cat'
                />
                <span className='post-reaction-picker-copy'>
                  <strong>saved-cat</strong>
                  <small>{SAVED_CAT_ASSET.asset_id}</small>
                </span>
              </button>
            </div>
          </section>
        </div>
      </section>
    </Card>
  );
}

function ReviewSurface() {
  return (
    <div style={{ minWidth: '1720px', minHeight: '1080px', padding: '1rem 1rem 6rem' }}>
      <ShellFrame
        skipTargetId='desktop-shell-fixes-review'
        topBar={<ShellTopBar activeTopic={STORY_ACTIVE_TOPIC} />}
        navRail={
          <ShellNavRail
            railId='desktop-shell-fixes-nav'
            open={false}
            onOpenChange={() => undefined}
            headerContent={
              <div className='shell-nav-status'>
                <div className='shell-status-badges'>
                  <StatusBadge label='connected' tone='accent' />
                  <StatusBadge label='4 peers' />
                  <StatusBadge label='seeded dht' />
                </div>
                <Button
                  className='shell-settings-button shell-icon-button'
                  variant='ghost'
                  size='icon'
                  type='button'
                >
                  <Settings className='size-5' aria-hidden='true' />
                </Button>
              </div>
            }
            addTopicControl={
              <label>
                <span>Add Topic</span>
                <div className='topic-input-row'>
                  <Input value='kukuri:topic:phase2' onChange={() => undefined} readOnly />
                  <Button variant='secondary' type='button'>
                    Add
                  </Button>
                </div>
              </label>
            }
            channelSummary='Core Contributors Coordination Room · Friends+'
            channelAction={
              <Button
                className='shell-icon-button shell-nav-channel-action'
                variant='secondary'
                size='icon'
                type='button'
                aria-label='Private channels'
              >
                <Lock className='size-4' aria-hidden='true' />
              </Button>
            }
            topicList={
              <TopicNavList
                items={TOPIC_ITEMS}
                onSelectTopic={() => undefined}
                onSelectChannel={() => undefined}
                onRemoveTopic={() => undefined}
              />
            }
            topicCount={TOPIC_ITEMS.length}
          />
        }
        workspace={
          <div className='shell-main-stack'>
            <Card className='shell-workspace-card shell-workspace-header-card'>
              <TimelineWorkspaceHeader
                activeSection='timeline'
                items={PRIMARY_ITEMS}
                onSelectSection={() => undefined}
              />
            </Card>
            <Card className='shell-workspace-card'>
              <div className='shell-workspace-header'>
                <div className='shell-workspace-summary'>
                  <span className='relationship-badge'>Viewing: Core Contributors</span>
                  <span className='relationship-badge relationship-badge-direct'>
                    Main lane max width: 60vw
                  </span>
                </div>
                <Button variant='secondary' type='button'>
                  Refresh
                </Button>
              </div>
            </Card>
            <Card className='shell-workspace-card'>
              <ul className='post-list'>
                <li>
                  <PostCard
                    view={TIMELINE_POST}
                    onOpenAuthor={() => undefined}
                    onOpenThread={() => undefined}
                    onReply={() => undefined}
                    onRepost={() => undefined}
                    onQuoteRepost={() => undefined}
                    localAuthorPubkey={'a'.repeat(64)}
                    mediaObjectUrls={REACTION_MEDIA_URLS}
                    ownedReactionAssets={[PARTY_PARROT_ASSET]}
                    bookmarkedReactionAssets={[SAVED_CAT_ASSET]}
                    recentReactions={RECENT_REACTIONS}
                    onToggleReaction={() => undefined}
                    onBookmarkCustomReaction={() => undefined}
                  />
                </li>
              </ul>
            </Card>
          </div>
        }
        detailPaneStack={
          <>
            <ComposerPreviewPane />
            <ActionPopoverPreviewPane />
          </>
        }
        detailPaneCount={2}
      />

      <Button className='shell-fab' variant='primary' size='icon' type='button' aria-label='Create post'>
        <Plus className='size-5' aria-hidden='true' />
      </Button>
    </div>
  );
}

export const WorkspaceReview: Story = {
  render: () => <ReviewSurface />,
};
