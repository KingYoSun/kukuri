import { type ReactNode, useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';
import { Settings } from 'lucide-react';

import { TimelineFeed } from '@/components/core/TimelineFeed';
import { TimelineWorkspaceHeader } from '@/components/core/TimelineWorkspaceHeader';
import { TopicNavList } from '@/components/core/TopicNavList';
import { type PostCardView, type TopicDiagnosticSummary } from '@/components/core/types';
import { ProfileEditorPanel } from '@/components/extended/ProfileEditorPanel';
import { ProfileOverviewPanel } from '@/components/extended/ProfileOverviewPanel';
import { ShellFrame } from '@/components/shell/ShellFrame';
import { ShellNavRail } from '@/components/shell/ShellNavRail';
import { ShellTopBar } from '@/components/shell/ShellTopBar';
import { type PrimarySection } from '@/components/shell/types';
import { STORY_ACTIVE_TOPIC } from '@/components/storyFixtures';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Select } from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';

const meta = {
  title: 'Review/DesktopShellWorkspaceGallery',
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
    peerCount: 2,
    lastReceivedLabel: '12:45:11',
    channels: [
      {
        channelId: 'channel-1',
        label: 'Core Contributors',
        audienceKind: 'friend_plus',
        active: true,
      },
      {
        channelId: 'channel-2',
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

const PROFILE_TIMELINE_POSTS: PostCardView[] = [
  {
    post: {
      object_id: 'profile-post-1',
      envelope_id: 'env-profile-post-1',
      author_pubkey: 'f'.repeat(64),
      author_name: 'local-author',
      author_display_name: 'Local Author',
      following: false,
      followed_by: false,
      mutual: false,
      friend_of_friend: false,
      object_kind: 'post',
      content: 'Profile overview stays topic-first and only shows my public posts in the active topic.',
      content_status: 'Available',
      attachments: [],
      created_at: 4,
      reply_to: null,
      root_id: 'profile-post-1',
      channel_id: null,
      audience_label: 'Public',
    },
    context: 'timeline',
    authorLabel: 'Local Author',
    authorPicture: null,
    relationshipLabel: null,
    audienceChipLabel: 'Public',
    threadTargetId: 'profile-post-1',
    media: {
      objectId: 'profile-post-1',
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

function ChannelRailControl() {
  const [channelLabel, setChannelLabel] = useState('Core Contributors');
  const [inviteToken, setInviteToken] = useState('share:kukuri:topic:demo:channel-1');

  return (
    <div className='shell-main-stack'>
      <form className='composer composer-compact' onSubmit={(event) => event.preventDefault()}>
        <Label>
          <span>Create Channel</span>
          <Input
            value={channelLabel}
            onChange={(event) => setChannelLabel(event.target.value)}
            placeholder='core contributors'
          />
        </Label>
        <Label>
          <span>Audience</span>
          <Select aria-label='Channel Audience' defaultValue='friend_plus'>
            <option value='invite_only'>Invite only</option>
            <option value='friend_only'>Friends</option>
            <option value='friend_plus'>Friends+</option>
          </Select>
        </Label>
        <div className='discovery-actions'>
          <Button variant='secondary' type='submit'>
            Create Channel
          </Button>
          <Button variant='secondary' type='button'>
            Share
          </Button>
        </div>
      </form>
      <form className='composer composer-compact' onSubmit={(event) => event.preventDefault()}>
        <Label>
          <span>Join</span>
          <Textarea
            value={inviteToken}
            onChange={(event) => setInviteToken(event.target.value)}
            placeholder='paste private channel invite, friend grant, or friends+ share'
          />
        </Label>
        <Button variant='secondary' type='submit'>
          Join
        </Button>
      </form>
      <Notice tone='accent'>
        <strong>Share</strong>
        <code className='extended-inline-code'>share:kukuri:topic:demo:channel-1</code>
      </Notice>
    </div>
  );
}

function ShellSurface({
  activeSection,
  workspace,
  width = 1720,
}: {
  activeSection: PrimarySection;
  workspace: ReactNode;
  width?: number;
}) {
  const [topicInput, setTopicInput] = useState('kukuri:topic:phase2');

  return (
    <div style={{ width: `${width}px`, minWidth: `${width}px`, margin: '0 auto' }}>
      <ShellFrame
        skipTargetId={`review-workspace-${activeSection}`}
        topBar={<ShellTopBar activeTopic={STORY_ACTIVE_TOPIC} />}
        navRail={
          <ShellNavRail
            railId={`review-nav-${activeSection}`}
            open={false}
            onOpenChange={() => undefined}
            headerContent={
              <div className='shell-nav-status'>
                <div className='shell-status-badges'>
                  <StatusBadge label='connected' tone='accent' />
                  <StatusBadge label='2 peers' />
                  <StatusBadge label='seeded dht' />
                </div>
                <Button className='shell-settings-button shell-icon-button' variant='ghost' size='icon' type='button'>
                  <Settings className='size-5' aria-hidden='true' />
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
                  <Button variant='secondary' type='button'>
                    Add
                  </Button>
                </div>
              </Label>
            }
            channelAction={<ChannelRailControl />}
            channelSummary='Core Contributors · Friends+'
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
                activeSection={activeSection}
                items={PRIMARY_ITEMS}
                onSelectSection={() => undefined}
              />
            </Card>
            {workspace}
          </div>
        }
      />
    </div>
  );
}

function TimelineWorkspace() {
  const [composer, setComposer] = useState('Weekly checkpoint is now scoped to Core Contributors.');

  return (
    <>
      <Card className='shell-workspace-card'>
        <div className='shell-workspace-header'>
          <div className='shell-workspace-summary'>
            <span className='relationship-badge'>Viewing: Core Contributors</span>
            <span className='relationship-badge relationship-badge-direct'>
              Posting: Core Contributors
            </span>
          </div>
          <Button variant='secondary' type='button'>
            Refresh
          </Button>
        </div>
      </Card>
      <Card className='shell-workspace-card'>
        <form className='composer' onSubmit={(event) => event.preventDefault()}>
          <Textarea
            value={composer}
            onChange={(event) => setComposer(event.target.value)}
            placeholder='Write a post'
          />
          <div className='composer-footer'>
            <div className='topic-diagnostic topic-diagnostic-secondary'>
              <span>Audience: Core Contributors</span>
            </div>
            <Button type='submit'>Publish</Button>
          </div>
        </form>
      </Card>
      <Card className='shell-workspace-card'>
        <TimelineFeed
          posts={PROFILE_TIMELINE_POSTS}
          emptyCopy='No posts yet for this topic.'
          onOpenAuthor={() => undefined}
          onOpenThread={() => undefined}
          onReply={() => undefined}
        />
      </Card>
    </>
  );
}

function LiveWorkspace() {
  const [title, setTitle] = useState('Launch Party');
  const [description, setDescription] = useState('watch party with topic peers');

  return (
    <>
      <Card className='shell-workspace-card'>
        <div className='panel-header'>
          <div>
            <h3>Live Sessions</h3>
            <small>1 active</small>
          </div>
        </div>
        <form className='composer composer-compact' onSubmit={(event) => event.preventDefault()}>
          <Label>
            <span>Live Title</span>
            <Input
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              placeholder='Friday stream'
            />
          </Label>
          <Label>
            <span>Live Description</span>
            <Textarea
              value={description}
              onChange={(event) => setDescription(event.target.value)}
              placeholder='short session summary'
            />
          </Label>
          <div className='topic-diagnostic topic-diagnostic-secondary'>
            <span>Audience: Core Contributors</span>
          </div>
          <Button type='submit'>Start Live</Button>
        </form>
      </Card>
      <Card className='shell-workspace-card'>
        <ul className='post-list'>
          <li>
            <article className='post-card'>
              <div className='post-meta'>
                <span>Launch Party</span>
                <span>Live</span>
                <span className='reply-chip'>Core Contributors</span>
              </div>
              <div className='post-body'>
                <strong className='post-title'>watch party with topic peers</strong>
              </div>
              <small>live-1</small>
              <div className='topic-diagnostic topic-diagnostic-secondary'>
                <span>viewers: 4</span>
                <span>started: 9:00:00</span>
              </div>
              <div className='post-actions'>
                <Button variant='secondary' type='button'>
                  Leave
                </Button>
                <Button variant='secondary' type='button'>
                  End
                </Button>
              </div>
            </article>
          </li>
        </ul>
      </Card>
    </>
  );
}

function GameWorkspace() {
  const [title, setTitle] = useState('Grand Finals');
  const [description, setDescription] = useState('set one');
  const [participants, setParticipants] = useState('Alice, Bob');
  const [phase, setPhase] = useState('Round 3');
  const [aliceScore, setAliceScore] = useState('2');
  const [bobScore, setBobScore] = useState('1');

  return (
    <>
      <Card className='shell-workspace-card'>
        <div className='panel-header'>
          <div>
            <h3>Game Rooms</h3>
            <small>1 tracked</small>
          </div>
        </div>
        <form className='composer composer-compact' onSubmit={(event) => event.preventDefault()}>
          <Label>
            <span>Game Title</span>
            <Input value={title} onChange={(event) => setTitle(event.target.value)} />
          </Label>
          <Label>
            <span>Game Description</span>
            <Textarea value={description} onChange={(event) => setDescription(event.target.value)} />
          </Label>
          <Label>
            <span>Participants</span>
            <Input
              value={participants}
              onChange={(event) => setParticipants(event.target.value)}
              placeholder='Alice, Bob'
            />
          </Label>
          <div className='topic-diagnostic topic-diagnostic-secondary'>
            <span>Audience: Core Contributors</span>
          </div>
          <Button type='submit'>Create Room</Button>
        </form>
      </Card>
      <Card className='shell-workspace-card'>
        <ul className='post-list'>
          <li>
            <article className='post-card'>
              <div className='post-meta'>
                <span>Grand Finals</span>
                <span>Running</span>
                <span className='reply-chip'>Core Contributors</span>
              </div>
              <div className='post-body'>
                <strong className='post-title'>set one</strong>
              </div>
              <small>game-1</small>
              <div className='topic-diagnostic topic-diagnostic-secondary'>
                <span>phase: Round 3</span>
                <span>updated: 9:15:00</span>
              </div>
              <ul className='draft-attachment-list'>
                <li className='draft-attachment-item score-row'>
                  <div className='draft-attachment-content'>
                    <strong>Alice</strong>
                  </div>
                  <Input aria-label='game-1-Alice-score' value={aliceScore} onChange={(event) => setAliceScore(event.target.value)} />
                </li>
                <li className='draft-attachment-item score-row'>
                  <div className='draft-attachment-content'>
                    <strong>Bob</strong>
                  </div>
                  <Input aria-label='game-1-Bob-score' value={bobScore} onChange={(event) => setBobScore(event.target.value)} />
                </li>
              </ul>
              <div className='composer composer-compact'>
                <Label>
                  <span>Status</span>
                  <Select aria-label='game-1-status' defaultValue='Running'>
                    <option value='Waiting'>Waiting</option>
                    <option value='Running'>Running</option>
                    <option value='Paused'>Paused</option>
                    <option value='Ended'>Ended</option>
                  </Select>
                </Label>
                <Label>
                  <span>Phase</span>
                  <Input aria-label='game-1-phase' value={phase} onChange={(event) => setPhase(event.target.value)} />
                </Label>
                <Button variant='secondary' type='button'>
                  Save Room
                </Button>
              </div>
            </article>
          </li>
        </ul>
      </Card>
    </>
  );
}

function ProfileOverviewWorkspace() {
  return (
    <>
      <ProfileOverviewPanel
        authorLabel='Local Author'
        about='Maintains the desktop shell migration and topic-first review cadence.'
        picture={null}
        status='ready'
        error={null}
        postCount={PROFILE_TIMELINE_POSTS.length}
        onEdit={() => undefined}
      />
      <Card className='shell-workspace-card'>
        <TimelineFeed
          posts={PROFILE_TIMELINE_POSTS}
          emptyCopy='No public posts from you in this topic yet.'
          onOpenAuthor={() => undefined}
          onOpenThread={() => undefined}
          onReply={() => undefined}
        />
      </Card>
    </>
  );
}

function ProfileEditWorkspace() {
  const [fields, setFields] = useState({
    displayName: 'Local Author',
    name: 'local-author',
    about: 'Maintains the desktop shell migration and topic-first review cadence.',
  });

  return (
    <>
      <ProfileEditorPanel
        authorLabel='Local Author'
        status='ready'
        saving={false}
        dirty={true}
        error={null}
        fields={fields}
        picturePreviewSrc='https://example.com/avatar.png'
        hasPicture={true}
        pictureInputKey={0}
        onFieldChange={(field, value) => setFields((current) => ({ ...current, [field]: value }))}
        onPictureSelect={() => undefined}
        onPictureClear={() => undefined}
        onBack={() => undefined}
        onSave={(event) => event.preventDefault()}
        onReset={() => undefined}
      />
      <Card className='shell-workspace-card'>
        <TimelineFeed
          posts={PROFILE_TIMELINE_POSTS}
          emptyCopy='No public posts from you in this topic yet.'
          onOpenAuthor={() => undefined}
          onOpenThread={() => undefined}
          onReply={() => undefined}
        />
      </Card>
    </>
  );
}

function GalleryStory() {
  return (
    <div style={{ display: 'grid', gap: '3rem', padding: '2rem 0 4rem' }}>
      <ShellSurface activeSection='timeline' workspace={<TimelineWorkspace />} />
      <ShellSurface activeSection='live' workspace={<LiveWorkspace />} />
      <ShellSurface activeSection='game' workspace={<GameWorkspace />} />
      <ShellSurface activeSection='profile' workspace={<ProfileOverviewWorkspace />} />
      <ShellSurface activeSection='profile' workspace={<ProfileEditWorkspace />} />
    </div>
  );
}

export const WorkspacePatternsGallery: Story = {
  render: () => <GalleryStory />,
};
