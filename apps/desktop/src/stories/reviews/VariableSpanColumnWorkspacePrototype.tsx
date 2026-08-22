import { useState, type ComponentProps, type FormEvent, type ReactNode } from 'react';
import {
  Bell,
  ChevronLeft,
  ChevronRight,
  GripVertical,
  Menu,
  MessageCircle,
  MoreHorizontal,
  Pin,
  Plus,
  Search,
  Send,
  Settings,
  Unplug,
  X,
} from 'lucide-react';

import { AuthorDetailCard } from '@/components/core/AuthorDetailCard';
import { ThreadPanel } from '@/components/core/ThreadPanel';
import { TimelineFeed } from '@/components/core/TimelineFeed';
import {
  createStoryAuthorDetailView,
  createStoryThreadPanelState,
  createStoryThreadPosts,
  createStoryTimelinePosts,
} from '@/components/storyFixtures';
import { LiveSessionPanel } from '@/components/extended/LiveSessionPanel';
import { MetaverseRoomView } from '@/components/extended/metaverse/MetaverseRoomView';
import { DEFAULT_SHARED_OBJECT } from '@/components/extended/MetaverseSceneModel';
import { Button } from '@/components/ui/button';
import type { GameRoomView } from '@/lib/api';

import './variable-span-column-workspace.css';

export type VariableSpanScenario =
  | 'single'
  | 'multi'
  | 'thread-chain'
  | 'stream'
  | 'metaverse-3'
  | 'metaverse-4'
  | 'mobile'
  | 'states';

type ColumnSpan = 1 | 2 | 3 | 4;

type ReviewColumnProps = {
  active?: boolean;
  children: ReactNode;
  index: number;
  initialPinned?: boolean;
  initialTransient?: boolean;
  primaryAction: string;
  scope: string;
  showDropTargetBefore?: boolean;
  span: ColumnSpan;
  title: string;
  total: number;
};

const noop = () => undefined;
const timelinePosts = createStoryTimelinePosts();
const threadPosts = createStoryThreadPosts();
const threadState = createStoryThreadPanelState();
const authorView = createStoryAuthorDetailView();
const STORY_TIMESTAMP = 1_742_860_800_000;

const liveSessions: ComponentProps<typeof LiveSessionPanel>['sessions'] = [
  {
    session: {
      session_id: 'live-1',
      host_pubkey: 'f'.repeat(64),
      title: 'Launch Party',
      description: 'Watch along with the Core Contributors channel.',
      status: 'Live',
      started_at: STORY_TIMESTAMP,
      ended_at: null,
      viewer_count: 18,
      joined_by_me: true,
      channel_id: 'channel-1',
      audience_label: 'Core Contributors',
    },
    isOwner: true,
    pending: false,
  },
];

const metaverseRoom: GameRoomView = {
  room_id: 'metaverse-room-1',
  host_pubkey: 'f'.repeat(64),
  title: 'Atrium',
  description: 'A shared space for the launch review.',
  status: 'Waiting',
  phase_label: 'metaverse-mvp',
  scores: [],
  room_kind: 'metaverse_room',
  metaverse: {
    world_version: 1,
    max_peers: 8,
    scene: {
      ground: 'default',
      shared_object: DEFAULT_SHARED_OBJECT,
    },
    default_spawn: {
      position: [0, 0, 260],
      rotation: [0, 180, 0],
    },
    asset_refs: [],
    chat_history: [],
  },
  manifest_blob_hash: 'mock-metaverse-room-1',
  updated_at: STORY_TIMESTAMP,
  channel_id: null,
  audience_label: 'Public',
};

function ReviewColumn({
  active = false,
  children,
  index,
  initialPinned = false,
  initialTransient = false,
  primaryAction,
  scope,
  showDropTargetBefore = false,
  span,
  title,
  total,
}: ReviewColumnProps) {
  const [pinned, setPinned] = useState(initialPinned);
  const [transient, setTransient] = useState(initialTransient);
  const [dragging, setDragging] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);

  return (
    <>
      {showDropTargetBefore ? (
        <div className='column-review-drop-target' role='separator' aria-label={`Drop before ${title}`}>
          <span>Drop before Column {index}</span>
        </div>
      ) : null}
      <section
        className='column-review-surface'
        data-active={active || undefined}
        data-dragging={dragging || undefined}
        data-pinned={pinned || undefined}
        data-span={span}
        data-transient={transient || undefined}
        aria-current={active ? 'true' : undefined}
        aria-label={`${title} Column`}
        aria-roledescription='Column'
      >
        <header className='column-review-header'>
          <Button
            className='column-review-grip'
            variant='ghost'
            size='icon'
            type='button'
            aria-label={dragging ? `Stop moving ${title}` : `Move ${title}`}
            aria-pressed={dragging}
            onClick={() => setDragging((current) => !current)}
          >
            <GripVertical className='size-5' aria-hidden='true' />
          </Button>
          <div className='column-review-heading'>
            <div className='column-review-title-row'>
              <h2>{title}</h2>
              {active ? <span className='column-review-state-label'>Active</span> : null}
              {pinned ? <span className='column-review-state-label'>Pinned</span> : null}
              {transient ? <span className='column-review-state-label'>Temporary</span> : null}
              {dragging ? <span className='column-review-state-label'>Moving</span> : null}
            </div>
            <p>{scope}</p>
            <span className='sr-only'>
              Column {index} of {total} · {span} span{span === 1 ? '' : 's'}
            </span>
          </div>
          <Button
            variant='ghost'
            size='icon'
            type='button'
            aria-label={pinned ? `Unpin ${title}` : `Pin ${title}`}
            aria-pressed={pinned}
            onClick={() => setPinned((current) => !current)}
          >
            <Pin className='size-4' aria-hidden='true' />
          </Button>
          <div className='column-review-menu-wrap'>
            <Button
              variant='ghost'
              size='icon'
              type='button'
              aria-label={`${menuOpen ? 'Close' : 'Open'} ${title} menu`}
              aria-expanded={menuOpen}
              onClick={() => setMenuOpen((current) => !current)}
            >
              <MoreHorizontal className='size-5' aria-hidden='true' />
            </Button>
            {menuOpen ? (
              <div className='column-review-menu' role='menu' aria-label={`${title} actions`}>
                <Button variant='ghost' size='sm' type='button' role='menuitem'>
                  <ChevronLeft className='size-4' aria-hidden='true' />
                  Move left
                </Button>
                <Button variant='ghost' size='sm' type='button' role='menuitem'>
                  <ChevronRight className='size-4' aria-hidden='true' />
                  Move right
                </Button>
                <Button
                  variant='ghost'
                  size='sm'
                  type='button'
                  role='menuitem'
                  onClick={() => setTransient((current) => !current)}
                >
                  {transient ? 'Keep Column' : 'Make temporary'}
                </Button>
                <Button variant='ghost' size='sm' type='button' role='menuitem'>
                  <X className='size-4' aria-hidden='true' />
                  Close Column
                </Button>
              </div>
            ) : null}
          </div>
        </header>
        <div className='column-review-body'>{children}</div>
        <footer className='column-review-footer'>
          <Button type='button' aria-label={`${primaryAction} in ${scope}`}>
            <Plus className='size-4' aria-hidden='true' />
            {primaryAction}
          </Button>
          <span aria-hidden='true'>Column {index}/{total}</span>
        </footer>
      </section>
    </>
  );
}

function TimelinePreview() {
  return (
    <TimelineFeed
      posts={timelinePosts}
      emptyCopy='No posts yet.'
      onOpenAuthor={noop}
      onOpenThread={noop}
      onReply={noop}
    />
  );
}

function ThreadPreview() {
  return (
    <ThreadPanel
      state={threadState}
      posts={threadPosts}
      onOpenAuthor={noop}
      onOpenThread={noop}
      onReply={noop}
    />
  );
}

function ProfilePreview() {
  return (
    <AuthorDetailCard
      view={authorView}
      localAuthorPubkey={'f'.repeat(64)}
      onToggleRelationship={noop}
      onToggleMute={noop}
    />
  );
}

function StreamPreview() {
  return (
    <div className='column-review-stream-layout'>
      <div className='column-review-stream-visual' role='img' aria-label='Stream video preview'>
        <span>LIVE</span>
        <strong>Launch Party</strong>
        <p>Video remains the primary surface while chat and session controls stay alongside it.</p>
      </div>
      <LiveSessionPanel
        status='ready'
        error={null}
        audienceLabel='Core Contributors'
        title='Friday stream'
        description='watch party'
        createPending={false}
        sessions={liveSessions}
        onTitleChange={noop}
        onDescriptionChange={noop}
        onSubmit={(event: FormEvent<HTMLFormElement>) => event.preventDefault()}
        onJoin={noop}
        onLeave={noop}
        onEnd={noop}
      />
    </div>
  );
}

function MetaversePreview() {
  return (
    <MetaverseRoomView
      room={metaverseRoom}
      activeTopic='kukuri:topic:demo'
      localPeerId='local-endpoint-a:story'
      remoteTransforms={{}}
      peerPresence={{}}
      sharedObject={DEFAULT_SHARED_OBJECT}
      avatarAssetUrl={null}
      latestChatByPeer={{}}
      connectionState='live'
      now={STORY_TIMESTAMP}
      knownPeerCount={2}
      lastSentSeq={12}
      lastReceivedAt={STORY_TIMESTAMP - 1_000}
      remoteAnimationSummary='remote-peer:walk'
      avatarAssetStatus='sample-vrm'
      localAvatarAssetRef={null}
      communityAssistAvailable={true}
      locale='en'
      pending={false}
      messages={[
        {
          roomId: metaverseRoom.room_id,
          messageId: 'story-message-1',
          authorPeerId: 'remote-peer',
          displayName: 'Remote Friend',
          body: 'Welcome to the room.',
          createdAt: STORY_TIMESTAMP - 2_000,
        },
      ]}
      messageDraft=''
      initialHudOpen={true}
      initialHudDebugOpen={false}
      initialChatOpen={true}
      onLocalTransform={noop}
      onAvatarAssetStatus={noop}
      onLeaveRoom={noop}
      onImportAvatar={noop}
      onImportDefaultAvatar={noop}
      onMoveSharedObject={noop}
      onMessageDraftChange={noop}
      onSendMessage={(event) => event.preventDefault()}
    />
  );
}

function PlaceholderPreview({ label }: { label: string }) {
  return (
    <div className='column-review-placeholder'>
      <strong>{label}</strong>
      <p>State styling remains readable without relying on color alone.</p>
    </div>
  );
}

function ControlCenter({ open, onClose }: { open: boolean; onClose: () => void }) {
  if (!open) {
    return null;
  }

  return (
    <aside className='column-review-control-center' aria-label='Control Center'>
      <header>
        <div>
          <p className='eyebrow'>Control Center</p>
          <h2>Move without losing context</h2>
        </div>
        <Button variant='ghost' size='icon' type='button' aria-label='Close Control Center' onClick={onClose}>
          <X className='size-5' aria-hidden='true' />
        </Button>
      </header>
      <div className='column-review-control-grid'>
        <section aria-labelledby='control-columns'>
          <h3 id='control-columns'>Columns</h3>
          <Button variant='secondary' type='button'><Plus className='size-4' aria-hidden='true' />Add Column</Button>
          <Button variant='ghost' type='button'><Menu className='size-4' aria-hidden='true' />Column list</Button>
        </section>
        <section aria-labelledby='control-places'>
          <h3 id='control-places'>Places</h3>
          <Button variant='secondary' type='button'><Search className='size-4' aria-hidden='true' />Find topic or channel</Button>
          <Button variant='ghost' type='button'><Send className='size-4' aria-hidden='true' />Join or Share</Button>
        </section>
        <section aria-labelledby='control-activity'>
          <h3 id='control-activity'>Activity</h3>
          <Button variant='secondary' type='button'><Bell className='size-4' aria-hidden='true' />Notifications <span>3</span></Button>
          <Button variant='ghost' type='button'><MessageCircle className='size-4' aria-hidden='true' />Messages</Button>
        </section>
        <section aria-labelledby='control-system'>
          <h3 id='control-system'>System</h3>
          <Button variant='secondary' type='button'><Unplug className='size-4' aria-hidden='true' />Connected · Direct P2P</Button>
          <Button variant='ghost' type='button'><Settings className='size-4' aria-hidden='true' />Settings</Button>
        </section>
      </div>
    </aside>
  );
}

type ColumnDescriptor = Omit<ReviewColumnProps, 'index' | 'total'>;

function scenarioColumns(scenario: VariableSpanScenario): ColumnDescriptor[] {
  switch (scenario) {
    case 'multi':
      return [
        { active: false, children: <TimelinePreview />, primaryAction: 'Create post', scope: 'Public · kukuri:topic:demo', span: 1, title: 'Timeline' },
        { active: true, children: <ThreadPreview />, primaryAction: 'Reply', scope: 'Thread · Launch planning', span: 1, title: 'Thread' },
        { children: <ProfilePreview />, initialTransient: true, primaryAction: 'Message', scope: 'Profile · bob', span: 1, title: 'Profile' },
      ];
    case 'thread-chain':
      return [
        { children: <TimelinePreview />, primaryAction: 'Create post', scope: 'Public · kukuri:topic:demo', span: 1, title: 'Timeline' },
        { children: <ThreadPreview />, initialPinned: true, primaryAction: 'Reply', scope: 'Thread · Launch planning', span: 1, title: 'Thread' },
        { active: true, children: <ThreadPreview />, initialTransient: true, primaryAction: 'Reply', scope: 'Reply branch · dan', span: 1, title: 'Reply thread' },
      ];
    case 'stream':
      return [
        { children: <TimelinePreview />, primaryAction: 'Create post', scope: 'Public · kukuri:topic:demo', span: 1, title: 'Timeline' },
        { active: true, children: <StreamPreview />, primaryAction: 'Leave Stream', scope: 'Core Contributors · channel-1', span: 2, title: 'Stream' },
      ];
    case 'metaverse-3':
      return [
        { children: <TimelinePreview />, primaryAction: 'Create post', scope: 'Public · kukuri:topic:demo', span: 1, title: 'Timeline' },
        { active: true, children: <MetaversePreview />, primaryAction: 'Leave room', scope: 'Public · Atrium', span: 3, title: 'Metaverse' },
      ];
    case 'metaverse-4':
      return [
        { active: true, children: <MetaversePreview />, initialPinned: true, primaryAction: 'Leave room', scope: 'Public · Atrium · Focused', span: 4, title: 'Metaverse focused' },
      ];
    case 'mobile':
      return [
        { active: true, children: <TimelinePreview />, primaryAction: 'Create post', scope: 'Public · kukuri:topic:demo', span: 1, title: 'Timeline' },
        { children: <ThreadPreview />, primaryAction: 'Reply', scope: 'Thread · Launch planning', span: 1, title: 'Thread' },
        { children: <ProfilePreview />, initialTransient: true, primaryAction: 'Message', scope: 'Profile · bob', span: 1, title: 'Profile' },
      ];
    case 'states':
      return [
        { active: true, children: <PlaceholderPreview label='Active Column' />, initialPinned: true, primaryAction: 'Create post', scope: 'Public · demo', span: 1, title: 'Active' },
        { children: <PlaceholderPreview label='Temporary Column' />, initialTransient: true, primaryAction: 'Reply', scope: 'Thread · temporary', span: 1, title: 'Temporary' },
        { children: <PlaceholderPreview label='Drop target' />, primaryAction: 'Message', scope: 'Profile · carol', showDropTargetBefore: true, span: 1, title: 'Drop target' },
      ];
    case 'single':
    default:
      return [
        { active: true, children: <TimelinePreview />, primaryAction: 'Create post', scope: 'Public · kukuri:topic:demo', span: 1, title: 'Timeline' },
      ];
  }
}

export function VariableSpanColumnWorkspacePrototype({
  initialControlCenterOpen = false,
  reducedMotion = false,
  scenario = 'single',
}: {
  initialControlCenterOpen?: boolean;
  reducedMotion?: boolean;
  scenario?: VariableSpanScenario;
}) {
  const [controlCenterOpen, setControlCenterOpen] = useState(initialControlCenterOpen);
  const columns = scenarioColumns(scenario);

  return (
    <main
      className='variable-column-review'
      data-mobile={scenario === 'mobile' || undefined}
      data-reduced-motion={reducedMotion ? 'reduce' : 'full'}
    >
      <a className='column-review-skip-link' href='#column-review-canvas'>Skip to Columns</a>
      <div className='column-review-backdrop' aria-hidden='true' />
      <div
        id='column-review-canvas'
        className='column-review-canvas'
        role='region'
        aria-label='Column Canvas'
      >
        {columns.map((column, columnIndex) => (
          <ReviewColumn
            key={`${scenario}-${column.title}`}
            {...column}
            index={columnIndex + 1}
            total={columns.length}
          />
        ))}
      </div>
      {scenario === 'mobile' ? (
        <nav className='column-review-mobile-indicator' aria-label='Column position'>
          <Button variant='ghost' size='icon' type='button' aria-label='Previous Column' disabled>
            <ChevronLeft className='size-4' aria-hidden='true' />
          </Button>
          <span>1 / {columns.length}</span>
          <Button variant='ghost' size='icon' type='button' aria-label='Next Column'>
            <ChevronRight className='size-4' aria-hidden='true' />
          </Button>
        </nav>
      ) : null}
      <Button
        className='column-review-control-trigger'
        type='button'
        aria-label='Open Control Center'
        aria-expanded={controlCenterOpen}
        aria-controls='column-review-control-center'
        onClick={() => setControlCenterOpen(true)}
      >
        <Menu className='size-5' aria-hidden='true' />
        Control Center
        <span className='column-review-status-dot' aria-label='Connected' />
      </Button>
      <div id='column-review-control-center'>
        <ControlCenter open={controlCenterOpen} onClose={() => setControlCenterOpen(false)} />
      </div>
    </main>
  );
}
