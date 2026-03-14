import { ChangeEvent, FormEvent, startTransition, useCallback, useEffect, useMemo, useState } from 'react';

import {
  AttachmentView,
  CreateAttachmentInput,
  DesktopApi,
  PostView,
  SyncStatus,
  TopicSyncStatus,
  runtimeApi,
} from './lib/api';

type AppProps = {
  api?: DesktopApi;
};

const DEFAULT_TOPIC = 'kukuri:topic:demo';
const REFRESH_INTERVAL_MS = 2000;

function selectPrimaryImage(post: PostView): AttachmentView | null {
  return post.attachments.find((attachment) => attachment.role === 'image_original') ?? null;
}

function selectVideoPoster(post: PostView): AttachmentView | null {
  return post.attachments.find((attachment) => attachment.role === 'video_poster') ?? null;
}

function selectVideoManifest(post: PostView): AttachmentView | null {
  return (
    post.attachments.find(
      (attachment) =>
        attachment.role === 'video_manifest' || attachment.mime.startsWith('video/')
    ) ?? null
  );
}

function attachmentPreviewSrc(): string | null {
  return null;
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
  if (bytes >= 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${bytes} B`;
}

async function fileToCreateAttachment(
  file: File,
  role: CreateAttachmentInput['role']
): Promise<CreateAttachmentInput> {
  const buffer = await file.arrayBuffer();
  const bytes = new Uint8Array(buffer);
  let binary = '';
  const chunkSize = 0x8000;
  for (let index = 0; index < bytes.length; index += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(index, index + chunkSize));
  }
  return {
    file_name: file.name,
    mime: file.type || 'application/octet-stream',
    byte_size: file.size,
    data_base64: window.btoa(binary),
    role,
  };
}

export function App({ api = runtimeApi }: AppProps) {
  const [trackedTopics, setTrackedTopics] = useState<string[]>([DEFAULT_TOPIC]);
  const [activeTopic, setActiveTopic] = useState(DEFAULT_TOPIC);
  const [topicInput, setTopicInput] = useState('');
  const [composer, setComposer] = useState('');
  const [draftAttachments, setDraftAttachments] = useState<CreateAttachmentInput[]>([]);
  const [attachmentInputKey, setAttachmentInputKey] = useState(0);
  const [timelinesByTopic, setTimelinesByTopic] = useState<Record<string, PostView[]>>({
    [DEFAULT_TOPIC]: [],
  });
  const [thread, setThread] = useState<PostView[]>([]);
  const [selectedThread, setSelectedThread] = useState<string | null>(null);
  const [replyTarget, setReplyTarget] = useState<PostView | null>(null);
  const [peerTicket, setPeerTicket] = useState('');
  const [localPeerTicket, setLocalPeerTicket] = useState<string | null>(null);
  const [blobPreviewUrls, setBlobPreviewUrls] = useState<Record<string, string | null>>({});
  const [syncStatus, setSyncStatus] = useState<SyncStatus>({
    connected: false,
    peer_count: 0,
    pending_events: 0,
    status_detail: 'No peer tickets imported',
    last_error: null,
    configured_peers: [],
    subscribed_topics: [],
    topic_diagnostics: [],
  });
  const [error, setError] = useState<string | null>(null);

  const headline = useMemo(
    () => (syncStatus.connected ? 'Live over static peers' : 'Local-first shell'),
    [syncStatus.connected]
  );

  const activeTimeline = useMemo(
    () => timelinesByTopic[activeTopic] ?? [],
    [activeTopic, timelinesByTopic]
  );
  const topicDiagnostics = useMemo(
    () =>
      Object.fromEntries(
        syncStatus.topic_diagnostics.map((diagnostic) => [diagnostic.topic, diagnostic])
      ) as Record<string, TopicSyncStatus>,
    [syncStatus.topic_diagnostics]
  );

  const loadTopics = useCallback(async (
    currentTopics: string[],
    currentActiveTopic: string,
    currentThread: string | null
  ) => {
    try {
      const [timelineViews, status, ticket, threadView] = await Promise.all([
        Promise.all(
          currentTopics.map(async (topic) => ({
            topic,
            timeline: await api.listTimeline(topic, null, 50),
          }))
        ),
        api.getSyncStatus(),
        api.getLocalPeerTicket(),
        currentThread
          ? api.listThread(currentActiveTopic, currentThread, null, 50)
          : Promise.resolve(null),
      ]);
      startTransition(() => {
        setTimelinesByTopic(
          Object.fromEntries(
            timelineViews.map(({ topic, timeline }) => [topic, timeline.items])
          )
        );
        setSyncStatus(status);
        setLocalPeerTicket(ticket);
        if (threadView) {
          setThread(threadView.items);
        } else if (!currentThread) {
          setThread([]);
        }
        setError(null);
      });
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : 'failed to load topic');
    }
  }, [api]);

  useEffect(() => {
    let disposed = false;

    const refresh = async () => {
      if (disposed) {
        return;
      }
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    };

    void refresh();
    const intervalId = window.setInterval(() => {
      void refresh();
    }, REFRESH_INTERVAL_MS);

    return () => {
      disposed = true;
      window.clearInterval(intervalId);
    };
  }, [activeTopic, loadTopics, selectedThread, trackedTopics]);

  useEffect(() => {
    const posts = [...activeTimeline, ...thread];
    const previewableAttachments = posts
      .flatMap((post) => [
        selectPrimaryImage(post),
        selectVideoPoster(post),
        selectVideoManifest(post),
      ])
      .filter((attachment): attachment is AttachmentView => attachment !== null)
      .filter((attachment) => attachment.status === 'Available' || attachment.status === 'Pinned');

    let disposed = false;
    for (const attachment of previewableAttachments) {
      if (blobPreviewUrls[attachment.hash] !== undefined) {
        continue;
      }
      void api
        .getBlobPreviewUrl(attachment.hash, attachment.mime)
        .then((url) => {
          if (disposed) {
            return;
          }
          setBlobPreviewUrls((current) => {
            if (current[attachment.hash] !== undefined) {
              return current;
            }
            return {
              ...current,
              [attachment.hash]: url,
            };
          });
        })
        .catch(() => {
          if (disposed) {
            return;
          }
          setBlobPreviewUrls((current) => {
            if (current[attachment.hash] !== undefined) {
              return current;
            }
            return {
              ...current,
              [attachment.hash]: null,
            };
          });
        });
    }

    return () => {
      disposed = true;
    };
  }, [activeTimeline, api, blobPreviewUrls, thread]);

  function clearThreadContext() {
    setSelectedThread(null);
    setThread([]);
    setReplyTarget(null);
  }

  async function handleAddTopic() {
    const nextTopic = topicInput.trim();
    if (!nextTopic) {
      return;
    }
    const nextTopics = trackedTopics.includes(nextTopic)
      ? trackedTopics
      : [...trackedTopics, nextTopic];
    setTrackedTopics(nextTopics);
    setActiveTopic(nextTopic);
    setTopicInput('');
    clearThreadContext();
    await loadTopics(nextTopics, nextTopic, null);
  }

  async function handleSelectTopic(topic: string) {
    setActiveTopic(topic);
    clearThreadContext();
    await loadTopics(trackedTopics, topic, null);
  }

  async function handleRemoveTopic(topic: string) {
    if (trackedTopics.length === 1) {
      return;
    }
    const nextTopics = trackedTopics.filter((value) => value !== topic);
    const nextActiveTopic = activeTopic === topic ? nextTopics[0] : activeTopic;
    await api.unsubscribeTopic(topic);
    setTrackedTopics(nextTopics);
    setActiveTopic(nextActiveTopic);
    clearThreadContext();
    await loadTopics(nextTopics, nextActiveTopic, null);
  }

  async function handlePublish(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!composer.trim() && draftAttachments.length === 0) {
      return;
    }

    try {
      await api.createPost(
        activeTopic,
        composer.trim(),
        replyTarget?.id ?? null,
        draftAttachments
      );
      setComposer('');
      setDraftAttachments([]);
      setAttachmentInputKey((value) => value + 1);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
      if (selectedThread) {
        await openThread(selectedThread);
      }
      setReplyTarget(null);
    } catch (publishError) {
      setError(publishError instanceof Error ? publishError.message : 'failed to publish');
    }
  }

  async function handleAttachmentSelection(event: ChangeEvent<HTMLInputElement>) {
    const files = Array.from(event.target.files ?? []).filter((file) =>
      file.type.startsWith('image/')
    );
    if (files.length === 0) {
      return;
    }

    try {
      const nextAttachments = await Promise.all(
        files.map((file) => fileToCreateAttachment(file, 'image_original'))
      );
      setDraftAttachments((current) => [...current, ...nextAttachments]);
      setError(null);
      setAttachmentInputKey((value) => value + 1);
    } catch (attachmentError) {
      setError(
        attachmentError instanceof Error
          ? attachmentError.message
          : 'failed to read image attachment'
      );
    }
  }

  async function handleVideoSelection(event: ChangeEvent<HTMLInputElement>) {
    const files = Array.from(event.target.files ?? []).filter((file) =>
      file.type.startsWith('video/')
    );
    if (files.length === 0) {
      return;
    }

    try {
      const nextAttachments = await Promise.all(
        files.map((file) => fileToCreateAttachment(file, 'video_manifest'))
      );
      setDraftAttachments((current) => [...current, ...nextAttachments]);
      setError(null);
      setAttachmentInputKey((value) => value + 1);
    } catch (attachmentError) {
      setError(
        attachmentError instanceof Error
          ? attachmentError.message
          : 'failed to read video attachment'
      );
    }
  }

  async function handlePosterSelection(event: ChangeEvent<HTMLInputElement>) {
    const files = Array.from(event.target.files ?? []).filter((file) =>
      file.type.startsWith('image/')
    );
    if (files.length === 0) {
      return;
    }

    try {
      const nextAttachments = await Promise.all(
        files.map((file) => fileToCreateAttachment(file, 'video_poster'))
      );
      setDraftAttachments((current) => [...current, ...nextAttachments]);
      setError(null);
      setAttachmentInputKey((value) => value + 1);
    } catch (attachmentError) {
      setError(
        attachmentError instanceof Error
          ? attachmentError.message
          : 'failed to read video poster'
      );
    }
  }

  function handleRemoveDraftAttachment(index: number) {
    setDraftAttachments((current) => current.filter((_, attachmentIndex) => attachmentIndex !== index));
  }

  async function openThread(threadId: string) {
    try {
      const threadView = await api.listThread(activeTopic, threadId, null, 50);
      startTransition(() => {
        setSelectedThread(threadId);
        setThread(threadView.items);
      });
    } catch (threadError) {
      setError(threadError instanceof Error ? threadError.message : 'failed to load thread');
    }
  }

  function beginReply(post: PostView) {
    setReplyTarget(post);
    if (post.root_id) {
      setSelectedThread(post.root_id);
      void openThread(post.root_id);
      return;
    }
    setSelectedThread(post.id);
    void openThread(post.id);
  }

  function clearReply() {
    setReplyTarget(null);
  }

  async function handleImportPeer() {
    if (!peerTicket.trim()) {
      return;
    }
    try {
      await api.importPeerTicket(peerTicket.trim());
      setPeerTicket('');
      await loadTopics(trackedTopics, activeTopic, selectedThread);
    } catch (importError) {
      setError(importError instanceof Error ? importError.message : 'failed to import peer');
    }
  }

  function renderPostCard(post: PostView, context: 'timeline' | 'thread') {
    const primaryImage = selectPrimaryImage(post);
    const videoPoster = selectVideoPoster(post);
    const videoManifest = selectVideoManifest(post);
    const primaryMedia = primaryImage ?? videoPoster ?? videoManifest;
    const mediaKind = primaryImage ? 'image' : videoManifest ? 'video' : null;
    const mediaMetaAttachment = mediaKind === 'video' ? videoManifest ?? primaryMedia : primaryMedia;
    const extraAttachmentCount = primaryMedia
      ? Math.max(post.attachments.filter((attachment) => attachment !== primaryMedia).length, 0)
      : 0;
    const isPendingText = post.content_status === 'Missing' && post.content === '[blob pending]';
    const imagePreviewSrc = primaryImage
      ? blobPreviewUrls[primaryImage.hash] ?? attachmentPreviewSrc()
      : null;
    const videoPosterPreviewSrc = videoPoster
      ? blobPreviewUrls[videoPoster.hash] ?? attachmentPreviewSrc()
      : null;
    const videoPreviewSrc = videoManifest
      ? blobPreviewUrls[videoManifest.hash] ?? attachmentPreviewSrc()
      : null;
    const mediaPreviewSrc =
      mediaKind === 'video' ? videoPosterPreviewSrc : imagePreviewSrc;
    const mediaIsReady = primaryMedia ? primaryMedia.status !== 'Missing' : false;
    const mediaStatusLabel =
      mediaKind === 'video'
        ? mediaIsReady
          ? 'video ready'
          : 'syncing poster'
        : mediaIsReady
          ? 'image ready'
          : 'syncing image';
    const threadTargetId = post.root_id ?? post.id;

    return (
      <article className={context === 'thread' ? 'post-card post-card-thread' : 'post-card'}>
        <button
          className='post-link'
          type='button'
          onClick={() => void openThread(threadTargetId)}
        >
          <div className='post-meta'>
            <span>{post.author_npub}</span>
            <span>{new Date(post.created_at * 1000).toLocaleTimeString('ja-JP')}</span>
          </div>
          {primaryMedia ? (
            <>
              <div
                className={mediaIsReady ? 'media-frame media-frame-ready' : 'media-frame media-frame-loading'}
              >
                <div className='media-badges'>
                  <span className='media-status-badge'>{mediaStatusLabel}</span>
                  {mediaKind === 'video' ? <span className='media-type-badge'>video</span> : null}
                  {extraAttachmentCount > 0 ? (
                    <span className='media-count-badge'>+{extraAttachmentCount}</span>
                  ) : null}
                </div>
                {mediaKind === 'video' && videoPreviewSrc ? (
                  <video
                    className='media-video'
                    controls
                    preload='metadata'
                    poster={videoPosterPreviewSrc ?? undefined}
                    data-testid={`media-video-${post.id}`}
                  >
                    <source src={videoPreviewSrc} type={videoManifest?.mime ?? 'video/mp4'} />
                  </video>
                ) : mediaIsReady && mediaPreviewSrc ? (
                  <img
                    className='media-preview'
                    src={mediaPreviewSrc}
                    alt={primaryMedia.mime}
                    data-testid={`media-preview-${post.id}`}
                  />
                ) : mediaIsReady ? (
                  <div
                    className='media-ready-placeholder'
                    data-testid={`media-ready-${post.id}`}
                  >
                    <span>{mediaKind === 'video' ? 'poster preview' : 'preview pending'}</span>
                  </div>
                ) : (
                  <div
                    className='media-skeleton'
                    data-testid={`media-skeleton-${post.id}`}
                    aria-hidden='true'
                  />
                )}
              </div>
              {mediaMetaAttachment ? (
                <div className='media-meta'>
                  <span>{mediaMetaAttachment.mime}</span>
                  <span>{formatBytes(mediaMetaAttachment.bytes)}</span>
                </div>
              ) : null}
            </>
          ) : null}
          <div className='post-body'>
            {isPendingText ? (
              <div
                className='text-skeleton-group'
                data-testid={`text-skeleton-${post.id}`}
                aria-hidden='true'
              >
                <span className='text-skeleton text-skeleton-line' />
                <span className='text-skeleton text-skeleton-line text-skeleton-line-short' />
              </div>
            ) : (
              <strong className='post-title'>{post.content}</strong>
            )}
          </div>
          <small>{post.note_id}</small>
          {post.reply_to ? <em className='reply-chip'>Reply</em> : null}
        </button>
        <div className='post-actions'>
          <button
            className='button button-secondary'
            type='button'
            onClick={() => beginReply(post)}
          >
            Reply
          </button>
        </div>
      </article>
    );
  }

  return (
    <main className='shell'>
      <section className='hero'>
        <div>
          <p className='eyebrow'>kukuri rebuild</p>
          <h1>{headline}</h1>
          <p className='lede'>
            topic, timeline, composer, thread, sync status の5要素だけで Linux MVP を成立させる
            desktop shell
          </p>
        </div>
        <label className='field'>
          <span>Add Topic</span>
          <div className='topic-input-row'>
            <input
              value={topicInput}
              onChange={(e) => setTopicInput(e.target.value)}
              placeholder='kukuri:topic:demo'
            />
            <button
              className='button button-secondary'
              type='button'
              onClick={() => void handleAddTopic()}
            >
              Add
            </button>
          </div>
        </label>
      </section>

      <section className='grid'>
        <aside className='panel panel-accent'>
          <h2>Sync Status</h2>
          <dl className='status-grid'>
            <div>
              <dt>Connected</dt>
              <dd>{syncStatus.connected ? 'yes' : 'no'}</dd>
            </div>
            <div>
              <dt>Peers</dt>
              <dd>{syncStatus.peer_count}</dd>
            </div>
            <div>
              <dt>Pending</dt>
              <dd>{syncStatus.pending_events}</dd>
            </div>
          </dl>
          <div className='diagnostic-block'>
            <strong>Configured Peers</strong>
            <p>{syncStatus.configured_peers.length > 0 ? syncStatus.configured_peers.join(', ') : 'none'}</p>
          </div>
          <div className='diagnostic-block'>
            <strong>Connection Detail</strong>
            <p>{syncStatus.status_detail}</p>
          </div>
          <div className='diagnostic-block'>
            <strong>Connected Peers</strong>
            <p>
              {syncStatus.topic_diagnostics
                .flatMap((diagnostic) => diagnostic.connected_peers)
                .filter((peer, index, peers) => peers.indexOf(peer) === index)
                .join(', ') || 'none'}
            </p>
          </div>
          <div className='diagnostic-block'>
            <strong>Last Error</strong>
            <p className={syncStatus.last_error ? 'diagnostic-error' : undefined}>
              {syncStatus.last_error ?? 'none'}
            </p>
          </div>
          <label className='field'>
            <span>Your Ticket</span>
            <textarea readOnly value={localPeerTicket ?? ''} className='ticket-output' />
          </label>
          <label className='field'>
            <span>Peer Ticket</span>
            <input
              value={peerTicket}
              onChange={(e) => setPeerTicket(e.target.value)}
              placeholder='nodeid@127.0.0.1:7777'
            />
          </label>
          <button className='button button-secondary' onClick={() => void handleImportPeer()}>
            Import Peer
          </button>
          <section className='topic-list'>
            <div className='panel-header'>
              <h3>Tracked Topics</h3>
              <small>{syncStatus.subscribed_topics.length} active</small>
            </div>
            <ul>
              {trackedTopics.map((topic) => (
                <li
                  key={topic}
                  className={
                    topic === activeTopic ? 'topic-item topic-item-active' : 'topic-item'
                  }
                >
                  <button
                    className='topic-link'
                    type='button'
                    onClick={() => void handleSelectTopic(topic)}
                  >
                    {topic}
                  </button>
                  {trackedTopics.length > 1 ? (
                    <button
                      className='topic-remove'
                      type='button'
                      onClick={() => void handleRemoveTopic(topic)}
                    >
                      x
                    </button>
                  ) : null}
                  <div className='topic-diagnostic'>
                    <span>
                      {topicDiagnostics[topic]?.joined ? 'joined' : 'idle'} / peers:{' '}
                      {topicDiagnostics[topic]?.peer_count ?? 0}
                    </span>
                    <small>
                      {topicDiagnostics[topic]?.last_received_at
                        ? new Date(topicDiagnostics[topic].last_received_at!).toLocaleTimeString('ja-JP')
                        : 'no events'}
                    </small>
                  </div>
                  <div className='topic-diagnostic topic-diagnostic-secondary'>
                    <span>
                      expected:{' '}
                      {(topicDiagnostics[topic]?.configured_peer_ids.length ?? 0)}
                    </span>
                    <span>
                      missing:{' '}
                      {(topicDiagnostics[topic]?.missing_peer_ids.length ?? 0)}
                    </span>
                  </div>
                  <div className='topic-diagnostic topic-diagnostic-secondary'>
                    <span>
                      {topicDiagnostics[topic]?.status_detail ?? 'No topic diagnostics yet'}
                    </span>
                  </div>
                  {topicDiagnostics[topic]?.last_error ? (
                    <div className='topic-diagnostic topic-diagnostic-error'>
                      <span>error: {topicDiagnostics[topic].last_error}</span>
                    </div>
                  ) : null}
                </li>
              ))}
            </ul>
          </section>
        </aside>

        <section className='panel'>
          <div className='panel-header'>
            <h2>Timeline</h2>
            <span className='active-topic-label'>{activeTopic}</span>
            <button
              className='button button-secondary'
              onClick={() => void loadTopics(trackedTopics, activeTopic, selectedThread)}
            >
              Refresh
            </button>
          </div>
          <form className='composer' onSubmit={handlePublish}>
            {replyTarget ? (
              <div className='reply-banner'>
                <div>
                  <strong>Replying</strong>
                  <p>{replyTarget.content}</p>
                </div>
                <button
                  className='button button-secondary'
                  type='button'
                  onClick={clearReply}
                >
                  Clear
                </button>
              </div>
            ) : null}
            <textarea
              value={composer}
              onChange={(e) => setComposer(e.target.value)}
              placeholder={replyTarget ? 'Write a reply' : 'Write a post'}
            />
            <label className='field file-field'>
              <span>Attach Images</span>
              <input
                key={attachmentInputKey}
                aria-label='Attach Images'
                type='file'
                accept='image/*'
                multiple
                onChange={(event) => {
                  void handleAttachmentSelection(event);
                }}
              />
            </label>
            <label className='field file-field'>
              <span>Attach Videos</span>
              <input
                key={`video-${attachmentInputKey}`}
                aria-label='Attach Videos'
                type='file'
                accept='video/*'
                multiple
                onChange={(event) => {
                  void handleVideoSelection(event);
                }}
              />
            </label>
            <label className='field file-field'>
              <span>Attach Video Posters</span>
              <input
                key={`poster-${attachmentInputKey}`}
                aria-label='Attach Video Posters'
                type='file'
                accept='image/*'
                multiple
                onChange={(event) => {
                  void handlePosterSelection(event);
                }}
              />
            </label>
            {draftAttachments.length > 0 ? (
              <ul className='draft-attachment-list'>
                {draftAttachments.map((attachment, index) => (
                  <li key={`${attachment.file_name ?? attachment.mime}-${index}`} className='draft-attachment-item'>
                    <div>
                      <strong>{attachment.file_name ?? attachment.mime}</strong>
                      <small>
                        {attachment.role ?? 'attachment'} · {formatBytes(attachment.byte_size)}
                      </small>
                    </div>
                    <button
                      className='button button-secondary'
                      type='button'
                      onClick={() => handleRemoveDraftAttachment(index)}
                    >
                      Remove
                    </button>
                  </li>
                ))}
              </ul>
            ) : null}
            <button className='button' type='submit'>
              {replyTarget ? 'Reply' : 'Publish'}
            </button>
          </form>
          <ul className='post-list'>
            {activeTimeline.map((post) => (
              <li key={post.id}>{renderPostCard(post, 'timeline')}</li>
            ))}
          </ul>
        </section>

        <section className='panel'>
          <div className='panel-header'>
            <h2>Thread</h2>
            {selectedThread ? (
              <button
                className='button button-secondary'
                type='button'
                onClick={() => {
                  setSelectedThread(null);
                  setThread([]);
                  clearReply();
                }}
              >
                Close
              </button>
            ) : null}
          </div>
          {selectedThread ? (
            <ul className='thread-list'>
              {thread.map((post) => (
                <li key={post.id} className='thread-item'>
                  {renderPostCard(post, 'thread')}
                </li>
              ))}
            </ul>
          ) : (
            <p className='empty'>Select a post to inspect the thread.</p>
          )}
        </section>
      </section>

      {error ? <p className='error'>{error}</p> : null}
    </main>
  );
}
