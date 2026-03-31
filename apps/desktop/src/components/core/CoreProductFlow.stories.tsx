import { useMemo, useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';
import { Settings } from 'lucide-react';

import { AuthorDetailCard } from '@/components/core/AuthorDetailCard';
import { ComposerPanel } from '@/components/core/ComposerPanel';
import { ThreadPanel } from '@/components/core/ThreadPanel';
import { TimelineFeed } from '@/components/core/TimelineFeed';
import { TimelineWorkspaceHeader } from '@/components/core/TimelineWorkspaceHeader';
import { TopicNavList } from '@/components/core/TopicNavList';
import { ContextPane } from '@/components/shell/ContextPane';
import { ShellFrame } from '@/components/shell/ShellFrame';
import { ShellNavRail } from '@/components/shell/ShellNavRail';
import { ShellTopBar } from '@/components/shell/ShellTopBar';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

import {
  type AuthorDetailView,
  type ComposerDraftMediaView,
  type PostCardView,
  type TopicDiagnosticSummary,
} from './types';

const meta = {
  title: 'Core/CoreProductFlow',
  parameters: {
    layout: 'fullscreen',
  },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

const TOPIC_ITEMS: TopicDiagnosticSummary[] = [
  {
    topic: 'kukuri:topic:demo',
    active: true,
    publicActive: true,
    removable: false,
    connectionLabel: 'joined',
    peerCount: 2,
    lastReceivedLabel: '12:45:11',
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

const DRAFT_ITEMS: ComposerDraftMediaView[] = [
  {
    id: 'draft-1',
    sourceName: 'roadmap.png',
    previewUrl:
      'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 320"><rect width="320" height="320" fill="%23f59d62"/><rect x="56" y="60" width="208" height="200" rx="18" fill="%23101923"/></svg>',
    attachments: [
      {
        key: 'image_original-roadmap',
        label: 'image_original',
        mime: 'image/png',
        byteSizeLabel: '96 KB',
      },
    ],
  },
];

const TIMELINE_POSTS: PostCardView[] = [
  {
    post: {
      object_id: 'timeline-post-1',
      envelope_id: 'env-timeline-post-1',
      author_pubkey: 'b'.repeat(64),
      author_name: 'bob',
      author_display_name: null,
      following: true,
      followed_by: true,
      mutual: true,
      friend_of_friend: false,
      object_kind: 'post',
      content: 'Core workspace now owns topic switching, composer, and thread context.',
      content_status: 'Available',
      attachments: [],
      created_at: 1,
      reply_to: null,
      root_id: 'timeline-post-1',
      channel_id: null,
      audience_label: 'Public',
    },
    context: 'timeline',
    authorLabel: 'bob',
    authorPicture: null,
    relationshipLabel: 'mutual',
    audienceChipLabel: 'Public',
    threadTargetId: 'timeline-post-1',
    media: {
      objectId: 'timeline-post-1',
      kind: null,
      statusLabel: null,
      extraAttachmentCount: 0,
      state: 'ready',
      metaMime: null,
      metaBytesLabel: null,
      imagePreviewSrc: null,
      videoPosterPreviewSrc: null,
      videoPlaybackSrc: null,
      videoUnsupportedOnClient: false,
    },
  },
  {
    post: {
      object_id: 'timeline-post-2',
      envelope_id: 'env-timeline-post-2',
      author_pubkey: 'c'.repeat(64),
      author_name: 'carol',
      author_display_name: 'Carol',
      following: false,
      followed_by: false,
      mutual: false,
      friend_of_friend: true,
      object_kind: 'post',
      content: 'Image preview stays visible before the blob finishes syncing.',
      content_status: 'Available',
      attachments: [],
      created_at: 2,
      reply_to: null,
      root_id: 'timeline-post-2',
      channel_id: null,
      audience_label: 'Public',
    },
    context: 'timeline',
    authorLabel: 'Carol',
    authorPicture: null,
    relationshipLabel: 'friend of friend',
    audienceChipLabel: 'Public',
    threadTargetId: 'timeline-post-2',
    media: {
      objectId: 'timeline-post-2',
      kind: 'image',
      statusLabel: 'image ready',
      extraAttachmentCount: 0,
      state: 'ready',
      metaMime: 'image/png',
      metaBytesLabel: '144 KB',
      imagePreviewSrc:
        'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360"><rect width="640" height="360" fill="%2300b3a4"/><path d="M0 300l140-130 100 80 110-120 150 170H0z" fill="%230f2231"/></svg>',
      videoPosterPreviewSrc: null,
      videoPlaybackSrc: null,
      videoUnsupportedOnClient: false,
    },
  },
];

const THREAD_POSTS: PostCardView[] = [
  {
    ...TIMELINE_POSTS[0],
    context: 'thread',
  },
  {
    post: {
      object_id: 'thread-reply-1',
      envelope_id: 'env-thread-reply-1',
      author_pubkey: 'd'.repeat(64),
      author_name: 'dan',
      author_display_name: null,
      following: false,
      followed_by: true,
      mutual: false,
      friend_of_friend: false,
      object_kind: 'comment',
      content: 'Reply mode keeps the audience label and lets the user publish in place.',
      content_status: 'Available',
      attachments: [],
      created_at: 3,
      reply_to: 'timeline-post-1',
      root_id: 'timeline-post-1',
      channel_id: null,
      audience_label: 'Public',
    },
    context: 'thread',
    authorLabel: 'dan',
    authorPicture: null,
    relationshipLabel: 'follows you',
    audienceChipLabel: 'Public',
    threadTargetId: 'timeline-post-1',
    media: {
      objectId: 'thread-reply-1',
      kind: null,
      statusLabel: null,
      extraAttachmentCount: 0,
      state: 'ready',
      metaMime: null,
      metaBytesLabel: null,
      imagePreviewSrc: null,
      videoPosterPreviewSrc: null,
      videoPlaybackSrc: null,
      videoUnsupportedOnClient: false,
    },
  },
];

const AUTHOR_VIEW: AuthorDetailView = {
  author: {
    author_pubkey: 'b'.repeat(64),
    name: 'bob',
    display_name: null,
    about: 'Maintains the topic-first shell and community-node connectivity reviews.',
    picture: null,
    updated_at: 1,
    following: true,
    followed_by: true,
    mutual: true,
    friend_of_friend: false,
    friend_of_friend_via_pubkeys: [],
  },
  displayLabel: 'bob',
  summary: {
    label: 'mutual',
    following: true,
    followedBy: true,
    mutual: true,
    friendOfFriend: false,
    viaPubkeys: [],
    isSelf: false,
    canFollow: true,
    followActionLabel: 'Unfollow',
  },
  authorError: null,
};

function CoreProductFlowStory({ width }: { width: number }) {
  const [topicInput, setTopicInput] = useState('kukuri:topic:phase2');
  const primaryItems = useMemo(
    () => [
      { id: 'timeline' as const, label: 'Timeline' },
      { id: 'live' as const, label: 'Live' },
      { id: 'game' as const, label: 'Game' },
      { id: 'profile' as const, label: 'Profile' },
    ],
    []
  );

  return (
    <div style={{ width: `${width}px`, minWidth: `${width}px`, margin: '0 auto' }}>
      <ShellFrame
        skipTargetId='core-story-workspace'
        topBar={<ShellTopBar activeTopic='kukuri:topic:demo' />}
        navRail={
          <ShellNavRail
            railId='core-story-nav'
            open={false}
            onOpenChange={() => undefined}
            headerContent={
              <div className='shell-nav-status'>
                <div className='shell-status-badges'>
                  <StatusBadge label='connected' tone='accent' />
                  <StatusBadge label='2 peers' />
                  <StatusBadge label='seeded dht' />
                </div>
                <Button className='shell-settings-button' variant='ghost' size='icon' type='button'>
                  <Settings className='size-6' aria-hidden='true' />
                </Button>
              </div>
            }
            addTopicControl={
              <Label>
                <span>Add Topic</span>
                <div className='topic-input-row'>
                  <Input
                    value={topicInput}
                    onChange={(event) => setTopicInput(event.target.value)}
                    placeholder='kukuri:topic:demo'
                  />
                  <Button variant='secondary'>Add</Button>
                </div>
              </Label>
            }
            channelAction={<Button variant='secondary'>Private Channels</Button>}
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
                items={primaryItems}
                onSelectSection={() => undefined}
              />
            </Card>
            <Card className='shell-workspace-card'>
              <section className='shell-section' tabIndex={-1}>
                <ComposerPanel
                  value='Sharing the Phase 2 shell review draft.'
                  onChange={() => undefined}
                  onSubmit={(event) => event.preventDefault()}
                  attachmentInputKey={0}
                  onAttachmentSelection={() => undefined}
                  draftMediaItems={DRAFT_ITEMS}
                  onRemoveDraftAttachment={() => undefined}
                  composerError={null}
                  audienceLabel='Imported'
                  replyTarget={{
                    content: 'Reply target stays in the core workspace.',
                    audienceLabel: 'Imported',
                  }}
                  onClearReply={() => undefined}
                />
                <TimelineFeed
                  posts={TIMELINE_POSTS}
                  emptyCopy='No posts yet for this topic.'
                  onOpenAuthor={() => undefined}
                  onOpenThread={() => undefined}
                  onReply={() => undefined}
                />
              </section>
            </Card>
          </div>
        }
        detailPaneStack={
          <>
            <ContextPane
              paneId='core-story-thread'
              title='Thread'
              summary='2 posts in thread'
              onClose={() => undefined}
            >
              <ThreadPanel
                state={{
                  selectedThreadId: 'timeline-post-1',
                  summary: '2 posts in thread',
                  emptyCopy: 'Select a post to inspect the thread.',
                }}
                posts={THREAD_POSTS}
                onOpenAuthor={() => undefined}
                onOpenThread={() => undefined}
                onReply={() => undefined}
              />
            </ContextPane>
            <ContextPane
              paneId='core-story-author'
              title='Author'
              summary='bob'
              onClose={() => undefined}
            >
              <AuthorDetailCard
                view={AUTHOR_VIEW}
                localAuthorPubkey={'f'.repeat(64)}
                onToggleRelationship={() => undefined}
                onToggleMute={() => undefined}
              />
            </ContextPane>
          </>
        }
        detailPaneCount={2}
      />
    </div>
  );
}

export const WideWorkspace: Story = {
  render: () => <CoreProductFlowStory width={1720} />,
};

export const NarrowWorkspace: Story = {
  render: () => <CoreProductFlowStory width={760} />,
};
