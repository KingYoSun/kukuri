import {
  ChangeEvent,
  FormEvent,
  SyntheticEvent,
  startTransition,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';

import {
  AttachmentView,
  BlobMediaPayload,
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

type DraftMediaItem = {
  id: string;
  source_name: string;
  preview_url: string;
  attachments: CreateAttachmentInput[];
};

type MediaDebugValue = boolean | number | string | null | undefined;
type MediaDebugFields = Record<string, MediaDebugValue>;

const DEFAULT_TOPIC = 'kukuri:topic:demo';
const REFRESH_INTERVAL_MS = 2000;
const VIDEO_POSTER_TIMEOUT_MS = 5000;
const MEDIA_DEBUG_STORAGE_KEY = 'kukuri:media-debug';

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

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
  if (bytes >= 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${bytes} B`;
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = '';
  const chunkSize = 0x8000;
  for (let index = 0; index < bytes.length; index += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(index, index + chunkSize));
  }
  return window.btoa(binary);
}

function base64ToBytes(base64: string): Uint8Array {
  const binary = window.atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

function createObjectUrlFromPayload(payload: BlobMediaPayload): string {
  const bytes = base64ToBytes(payload.bytes_base64);
  const normalizedBytes = new Uint8Array(bytes.length);
  normalizedBytes.set(bytes);
  return URL.createObjectURL(new Blob([normalizedBytes], { type: payload.mime }));
}

function isMediaDebugEnabled(): boolean {
  if (import.meta.env.MODE === 'test') {
    return false;
  }

  if (import.meta.env.DEV) {
    return true;
  }

  try {
    return window.localStorage.getItem(MEDIA_DEBUG_STORAGE_KEY) === '1';
  } catch {
    return false;
  }
}

function logMediaDebug(level: 'info' | 'warn', event: string, fields: MediaDebugFields): void {
  if (!isMediaDebugEnabled()) {
    return;
  }

  const logger = level === 'warn' ? console.warn : console.info;
  logger(`[kukuri.media] ${event}`, fields);
}

function mediaElementDebugFields(media: HTMLMediaElement): MediaDebugFields {
  return {
    current_src: media.currentSrc || media.getAttribute('src') || null,
    current_time: Number.isFinite(media.currentTime) ? media.currentTime : null,
    duration: Number.isFinite(media.duration) ? media.duration : null,
    ended: media.ended,
    error_code: media.error?.code ?? null,
    network_state: media.networkState,
    paused: media.paused,
    ready_state: media.readyState,
  };
}

function attachVideoDebugListeners(
  video: HTMLVideoElement,
  phase: string,
  fields: MediaDebugFields
): () => void {
  const eventNames = [
    'loadstart',
    'loadedmetadata',
    'loadeddata',
    'canplay',
    'durationchange',
    'seeked',
    'playing',
    'error',
  ] as const;
  const removeListeners = eventNames.map((eventName) => {
    const handler = () => {
      logMediaDebug(eventName === 'error' ? 'warn' : 'info', `${phase} ${eventName}`, {
        ...fields,
        ...mediaElementDebugFields(video),
        video_height: video.videoHeight || null,
        video_width: video.videoWidth || null,
      });
    };
    video.addEventListener(eventName, handler);
    return () => {
      video.removeEventListener(eventName, handler);
    };
  });

  return () => {
    for (const removeListener of removeListeners) {
      removeListener();
    }
  };
}

async function blobToCreateAttachment(
  blob: Blob,
  fileName: string,
  role: CreateAttachmentInput['role']
): Promise<CreateAttachmentInput> {
  const bytes = new Uint8Array(await blob.arrayBuffer());
  return {
    file_name: fileName,
    mime: blob.type || 'application/octet-stream',
    byte_size: blob.size,
    data_base64: bytesToBase64(bytes),
    role,
  };
}

async function fileToCreateAttachment(
  file: File,
  role: CreateAttachmentInput['role']
): Promise<CreateAttachmentInput> {
  return blobToCreateAttachment(file, file.name, role);
}

function posterFileName(fileName: string): string {
  const extensionIndex = fileName.lastIndexOf('.');
  const baseName = extensionIndex >= 0 ? fileName.slice(0, extensionIndex) : fileName;
  return `${baseName}.poster.jpg`;
}

function attachHiddenVideo(video: HTMLVideoElement) {
  video.setAttribute('aria-hidden', 'true');
  video.style.position = 'fixed';
  video.style.left = '-9999px';
  video.style.top = '0';
  video.style.width = '1px';
  video.style.height = '1px';
  video.style.opacity = '0';
  video.style.pointerEvents = 'none';
  document.body.appendChild(video);
}

async function waitForPosterFrame(video: HTMLVideoElement): Promise<void> {
  return await new Promise<void>((resolve, reject) => {
    let settled = false;

    const cleanup = () => {
      video.removeEventListener('loadeddata', resolveIfReady);
      video.removeEventListener('canplay', resolveIfReady);
      video.removeEventListener('seeked', resolveIfReady);
      video.removeEventListener('timeupdate', resolveIfReady);
      video.removeEventListener('loadedmetadata', handleMetadata);
      video.removeEventListener('error', fail);
    };

    const finish = () => {
      if (settled) {
        return;
      }
      settled = true;
      cleanup();
      resolve();
    };

    const fail = () => {
      if (settled) {
        return;
      }
      settled = true;
      cleanup();
      reject(new Error('failed to generate video poster'));
    };

    const resolveIfReady = () => {
      if (
        video.videoWidth > 0 &&
        video.videoHeight > 0 &&
        video.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA
      ) {
        finish();
      }
    };

    const handleMetadata = () => {
      resolveIfReady();
      if (settled) {
        return;
      }

      const duration = Number.isFinite(video.duration) ? video.duration : 0;
      const seekTarget = duration > 0 ? Math.min(duration / 2, 0.1) : 0.1;
      if (seekTarget > 0) {
        try {
          video.currentTime = seekTarget;
        } catch {
          // Some platforms reject seek before decode warms up.
        }
      }

      try {
        const playAttempt = video.play();
        if (playAttempt && typeof playAttempt.then === 'function') {
          void playAttempt.then(() => {
            video.pause();
            resolveIfReady();
          });
        }
      } catch {
        // ignore
      }
    };

    video.addEventListener('loadeddata', resolveIfReady);
    video.addEventListener('canplay', resolveIfReady);
    video.addEventListener('seeked', resolveIfReady);
    video.addEventListener('timeupdate', resolveIfReady);
    video.addEventListener('loadedmetadata', handleMetadata);
    video.addEventListener('error', fail, { once: true });
    resolveIfReady();
  });
}

async function generateVideoPoster(file: File): Promise<File> {
  const videoObjectUrl = URL.createObjectURL(file);
  logMediaDebug('info', 'poster generation start', {
    file_name: file.name,
    mime: file.type || null,
    size: file.size,
    video_object_url: videoObjectUrl,
  });

  try {
    return await new Promise<File>((resolve, reject) => {
      const video = document.createElement('video');
      const canvas = document.createElement('canvas');
      let finished = false;
      const removeDebugListeners = attachVideoDebugListeners(video, 'poster', {
        file_name: file.name,
        mime: file.type || null,
        size: file.size,
      });

      const fail = () => {
        if (finished) {
          return;
        }
        finished = true;
        logMediaDebug('warn', 'poster generation failed', {
          file_name: file.name,
          mime: file.type || null,
          size: file.size,
          ...mediaElementDebugFields(video),
          video_height: video.videoHeight || null,
          video_width: video.videoWidth || null,
        });
        reject(new Error('failed to generate video poster'));
      };

      const timeoutId = window.setTimeout(fail, VIDEO_POSTER_TIMEOUT_MS);

      const cleanup = () => {
        window.clearTimeout(timeoutId);
        removeDebugListeners();
        try {
          video.pause();
        } catch {
          // ignore
        }
        video.removeAttribute('src');
        try {
          video.load();
        } catch {
          // ignore
        }
        video.remove();
      };

      video.preload = 'metadata';
      video.muted = true;
      video.playsInline = true;
      attachHiddenVideo(video);

      video.src = videoObjectUrl;
      video.load();

      void waitForPosterFrame(video)
        .then(() => {
          if (finished) {
            return;
          }

          const width = video.videoWidth;
          const height = video.videoHeight;
          if (!width || !height) {
            cleanup();
            fail();
            return;
          }

          logMediaDebug('info', 'poster frame ready', {
            file_name: file.name,
            height,
            mime: file.type || null,
            size: file.size,
            width,
            ...mediaElementDebugFields(video),
          });

          canvas.width = width;
          canvas.height = height;
          const context = canvas.getContext('2d');
          if (!context) {
            cleanup();
            fail();
            return;
          }

          context.drawImage(video, 0, 0, width, height);
          canvas.toBlob(
            (blob) => {
              if (finished) {
                return;
              }
              cleanup();
              if (!blob) {
                fail();
                return;
              }
              finished = true;
              logMediaDebug('info', 'poster generation complete', {
                blob_size: blob.size,
                file_name: file.name,
                mime: file.type || null,
                poster_file_name: posterFileName(file.name),
                size: file.size,
              });
              resolve(
                new File([blob], posterFileName(file.name), {
                  type: 'image/jpeg',
                })
              );
            },
            'image/jpeg',
            0.85
          );
        })
        .catch((error: unknown) => {
          logMediaDebug('warn', 'poster generation exception', {
            error: error instanceof Error ? error.message : 'unknown error',
            file_name: file.name,
            mime: file.type || null,
            size: file.size,
          });
          cleanup();
          fail();
        });
    });
  } finally {
    URL.revokeObjectURL(videoObjectUrl);
  }
}

export function App({ api = runtimeApi }: AppProps) {
  const [trackedTopics, setTrackedTopics] = useState<string[]>([DEFAULT_TOPIC]);
  const [activeTopic, setActiveTopic] = useState(DEFAULT_TOPIC);
  const [topicInput, setTopicInput] = useState('');
  const [composer, setComposer] = useState('');
  const [draftMediaItems, setDraftMediaItems] = useState<DraftMediaItem[]>([]);
  const [attachmentInputKey, setAttachmentInputKey] = useState(0);
  const [timelinesByTopic, setTimelinesByTopic] = useState<Record<string, PostView[]>>({
    [DEFAULT_TOPIC]: [],
  });
  const [thread, setThread] = useState<PostView[]>([]);
  const [selectedThread, setSelectedThread] = useState<string | null>(null);
  const [replyTarget, setReplyTarget] = useState<PostView | null>(null);
  const [peerTicket, setPeerTicket] = useState('');
  const [localPeerTicket, setLocalPeerTicket] = useState<string | null>(null);
  const [mediaObjectUrls, setMediaObjectUrls] = useState<Record<string, string | null>>({});
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
  const [composerError, setComposerError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const draftSequenceRef = useRef(0);
  const mediaFetchAttemptRef = useRef(new Map<string, number>());
  const remoteObjectUrlRef = useRef(new Map<string, string>());
  const draftPreviewUrlRef = useRef(new Map<string, string>());

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
  const previewableMediaAttachments = useMemo(() => {
    const attachments = new Map<string, AttachmentView>();
    for (const post of [...activeTimeline, ...thread]) {
      for (const attachment of [
        selectPrimaryImage(post),
        selectVideoPoster(post),
        selectVideoManifest(post),
      ]) {
        if (attachment) {
          attachments.set(attachment.hash, attachment);
        }
      }
    }
    return [...attachments.values()];
  }, [activeTimeline, thread]);

  const loadTopics = useCallback(
    async (currentTopics: string[], currentActiveTopic: string, currentThread: string | null) => {
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
            Object.fromEntries(timelineViews.map(({ topic, timeline }) => [topic, timeline.items]))
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
    },
    [api]
  );

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
    const remoteObjectUrls = remoteObjectUrlRef.current;
    const draftPreviewUrls = draftPreviewUrlRef.current;

    return () => {
      for (const url of remoteObjectUrls.values()) {
        URL.revokeObjectURL(url);
      }
      remoteObjectUrls.clear();
      for (const url of draftPreviewUrls.values()) {
        URL.revokeObjectURL(url);
      }
      draftPreviewUrls.clear();
    };
  }, []);

  useEffect(() => {
    let disposed = false;

    for (const attachment of previewableMediaAttachments) {
      if (typeof mediaObjectUrls[attachment.hash] === 'string') {
        continue;
      }

      const nextAttempt = (mediaFetchAttemptRef.current.get(attachment.hash) ?? 0) + 1;
      mediaFetchAttemptRef.current.set(attachment.hash, nextAttempt);
      logMediaDebug('info', 'remote media fetch start', {
        attempt: nextAttempt,
        hash: attachment.hash,
        mime: attachment.mime,
        role: attachment.role,
        status: attachment.status,
      });

      void api
        .getBlobMediaPayload(attachment.hash, attachment.mime)
        .then((payload) => {
          const nextUrl = payload ? createObjectUrlFromPayload(payload) : null;
          if (disposed) {
            if (nextUrl) {
              URL.revokeObjectURL(nextUrl);
            }
            return;
          }
          if (!nextUrl) {
            logMediaDebug('warn', 'remote media fetch missing', {
              attempt: nextAttempt,
              hash: attachment.hash,
              mime: attachment.mime,
              role: attachment.role,
              status: attachment.status,
            });
            return;
          }

          logMediaDebug('info', 'remote media fetch hit', {
            attempt: nextAttempt,
            bytes_base64_length: payload?.bytes_base64.length ?? 0,
            hash: attachment.hash,
            mime: attachment.mime,
            object_url: nextUrl,
            role: attachment.role,
            status: attachment.status,
          });

          setMediaObjectUrls((current) => {
            if (current[attachment.hash] !== undefined) {
              if (nextUrl) {
                URL.revokeObjectURL(nextUrl);
              }
              return current;
            }
            if (nextUrl) {
              remoteObjectUrlRef.current.set(attachment.hash, nextUrl);
            }
            return {
              ...current,
              [attachment.hash]: nextUrl,
            };
          });
        })
        .catch((fetchError: unknown) => {
          if (disposed) {
            return;
          }
          logMediaDebug('warn', 'remote media fetch error', {
            attempt: nextAttempt,
            error: fetchError instanceof Error ? fetchError.message : 'unknown error',
            hash: attachment.hash,
            mime: attachment.mime,
            role: attachment.role,
            status: attachment.status,
          });
        });
    }

    return () => {
      disposed = true;
    };
  }, [api, mediaObjectUrls, previewableMediaAttachments]);

  function nextDraftId(): string {
    draftSequenceRef.current += 1;
    return `draft-${draftSequenceRef.current}`;
  }

  function rememberDraftPreview(item: DraftMediaItem) {
    draftPreviewUrlRef.current.set(item.id, item.preview_url);
  }

  function releaseDraftPreview(itemId: string) {
    const previewUrl = draftPreviewUrlRef.current.get(itemId);
    if (!previewUrl) {
      return;
    }
    URL.revokeObjectURL(previewUrl);
    draftPreviewUrlRef.current.delete(itemId);
  }

  function releaseAllDraftPreviews() {
    for (const [itemId, previewUrl] of draftPreviewUrlRef.current.entries()) {
      URL.revokeObjectURL(previewUrl);
      draftPreviewUrlRef.current.delete(itemId);
    }
  }

  async function buildImageDraftItem(file: File): Promise<DraftMediaItem> {
    const attachment = await fileToCreateAttachment(file, 'image_original');
    return {
      id: nextDraftId(),
      source_name: file.name,
      preview_url: URL.createObjectURL(file),
      attachments: [attachment],
    };
  }

  async function buildVideoDraftItem(file: File): Promise<DraftMediaItem> {
    const posterFile = await generateVideoPoster(file);
    return {
      id: nextDraftId(),
      source_name: file.name,
      preview_url: URL.createObjectURL(posterFile),
      attachments: [
        await fileToCreateAttachment(file, 'video_manifest'),
        await blobToCreateAttachment(posterFile, posterFile.name, 'video_poster'),
      ],
    };
  }

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
    const attachments = draftMediaItems.flatMap((item) => item.attachments);
    if (!composer.trim() && attachments.length === 0) {
      return;
    }

    try {
      await api.createPost(activeTopic, composer.trim(), replyTarget?.id ?? null, attachments);
      releaseAllDraftPreviews();
      setComposer('');
      setDraftMediaItems([]);
      setAttachmentInputKey((value) => value + 1);
      setComposerError(null);
      await loadTopics(trackedTopics, activeTopic, selectedThread);
      if (selectedThread) {
        await openThread(selectedThread);
      }
      setReplyTarget(null);
    } catch (publishError) {
      setComposerError(
        publishError instanceof Error ? publishError.message : 'failed to publish'
      );
    }
  }

  async function handleAttachmentSelection(event: ChangeEvent<HTMLInputElement>) {
    const files = Array.from(event.target.files ?? []);
    if (files.length === 0) {
      return;
    }

    const nextItems: DraftMediaItem[] = [];
    const failures: string[] = [];

    for (const file of files) {
      try {
        if (file.type.startsWith('image/')) {
          nextItems.push(await buildImageDraftItem(file));
          continue;
        }
        if (file.type.startsWith('video/')) {
          nextItems.push(await buildVideoDraftItem(file));
          continue;
        }
        failures.push(`unsupported attachment type: ${file.name}`);
      } catch (attachmentError) {
        failures.push(
          attachmentError instanceof Error
            ? attachmentError.message
            : 'failed to generate video poster'
        );
      }
    }

    if (nextItems.length > 0) {
      nextItems.forEach(rememberDraftPreview);
      setDraftMediaItems((current) => [...current, ...nextItems]);
    }

    setComposerError(failures.length > 0 ? failures[0] : null);
    setAttachmentInputKey((value) => value + 1);
  }

  function handleRemoveDraftAttachment(itemId: string) {
    releaseDraftPreview(itemId);
    setDraftMediaItems((current) => current.filter((item) => item.id !== itemId));
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
    const mediaKind = primaryImage ? 'image' : videoManifest || videoPoster ? 'video' : null;
    const mediaMetaAttachment = mediaKind === 'video' ? videoManifest ?? videoPoster : primaryImage;
    const reservedHashes = new Set<string>();
    if (primaryImage) {
      reservedHashes.add(primaryImage.hash);
    }
    if (videoPoster) {
      reservedHashes.add(videoPoster.hash);
    }
    if (videoManifest) {
      reservedHashes.add(videoManifest.hash);
    }
    const extraAttachmentCount = post.attachments.filter(
      (attachment) => !reservedHashes.has(attachment.hash)
    ).length;
    const isPendingText = post.content_status === 'Missing' && post.content === '[blob pending]';
    const imagePreviewSrc =
      primaryImage && typeof mediaObjectUrls[primaryImage.hash] === 'string'
        ? mediaObjectUrls[primaryImage.hash]
        : null;
    const videoPosterPreviewSrc =
      videoPoster && typeof mediaObjectUrls[videoPoster.hash] === 'string'
        ? mediaObjectUrls[videoPoster.hash]
        : null;
    const videoPlaybackSrc =
      videoManifest && typeof mediaObjectUrls[videoManifest.hash] === 'string'
        ? mediaObjectUrls[videoManifest.hash]
        : null;
    const logPlaybackEvent = (eventName: string) => (event: SyntheticEvent<HTMLVideoElement>) => {
      const video = event.currentTarget;
      logMediaDebug(eventName === 'error' ? 'warn' : 'info', `playback ${eventName}`, {
        manifest_hash: videoManifest?.hash ?? null,
        mime: videoManifest?.mime ?? null,
        post_id: post.id,
        poster_hash: videoPoster?.hash ?? null,
        playback_src: videoPlaybackSrc,
        ...mediaElementDebugFields(video),
        video_height: video.videoHeight || null,
        video_width: video.videoWidth || null,
      });
    };
    const mediaStatusLabel =
      mediaKind === 'video'
        ? videoPlaybackSrc
          ? 'playable video'
          : videoPosterPreviewSrc
            ? 'poster ready'
            : 'syncing poster'
        : mediaKind === 'image'
          ? imagePreviewSrc
            ? 'image ready'
            : 'syncing image'
          : null;
    const threadTargetId = post.root_id ?? post.id;

    return (
      <article className={context === 'thread' ? 'post-card post-card-thread' : 'post-card'}>
        <button className='post-link' type='button' onClick={() => void openThread(threadTargetId)}>
          <div className='post-meta'>
            <span>{post.author_npub}</span>
            <span>{new Date(post.created_at * 1000).toLocaleTimeString('ja-JP')}</span>
          </div>
          {mediaKind ? (
            <>
              <div
                className={
                  mediaStatusLabel === 'syncing image' || mediaStatusLabel === 'syncing poster'
                    ? 'media-frame media-frame-loading'
                    : 'media-frame media-frame-ready'
                }
              >
                <div className='media-badges'>
                  {mediaStatusLabel ? <span className='media-status-badge'>{mediaStatusLabel}</span> : null}
                  {mediaKind === 'video' ? <span className='media-type-badge'>video</span> : null}
                  {extraAttachmentCount > 0 ? (
                    <span className='media-count-badge'>+{extraAttachmentCount}</span>
                  ) : null}
                </div>
                {mediaKind === 'video' && videoPlaybackSrc ? (
                  <video
                    className='media-video'
                    controls
                    src={videoPlaybackSrc}
                    onCanPlay={logPlaybackEvent('canplay')}
                    onDurationChange={logPlaybackEvent('durationchange')}
                    onError={logPlaybackEvent('error')}
                    onLoadedData={logPlaybackEvent('loadeddata')}
                    onLoadedMetadata={logPlaybackEvent('loadedmetadata')}
                    onLoadStart={logPlaybackEvent('loadstart')}
                    onPlaying={logPlaybackEvent('playing')}
                    preload='metadata'
                    poster={videoPosterPreviewSrc ?? undefined}
                    data-testid={`media-video-${post.id}`}
                  />
                ) : mediaKind === 'video' && videoPosterPreviewSrc ? (
                  <img
                    className='media-preview'
                    src={videoPosterPreviewSrc}
                    alt={videoPoster?.mime ?? 'video poster'}
                    data-testid={`media-preview-${post.id}`}
                  />
                ) : mediaKind === 'image' && imagePreviewSrc ? (
                  <img
                    className='media-preview'
                    src={imagePreviewSrc}
                    alt={primaryImage?.mime ?? 'image attachment'}
                    data-testid={`media-preview-${post.id}`}
                  />
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
          <button className='button button-secondary' type='button' onClick={() => beginReply(post)}>
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
                  className={topic === activeTopic ? 'topic-item topic-item-active' : 'topic-item'}
                >
                  <button className='topic-link' type='button' onClick={() => void handleSelectTopic(topic)}>
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
                    <span>expected: {topicDiagnostics[topic]?.configured_peer_ids.length ?? 0}</span>
                    <span>missing: {topicDiagnostics[topic]?.missing_peer_ids.length ?? 0}</span>
                  </div>
                  <div className='topic-diagnostic topic-diagnostic-secondary'>
                    <span>{topicDiagnostics[topic]?.status_detail ?? 'No topic diagnostics yet'}</span>
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
                <button className='button button-secondary' type='button' onClick={clearReply}>
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
              <span>Attach</span>
              <input
                key={attachmentInputKey}
                aria-label='Attach'
                type='file'
                accept='image/*,video/*'
                multiple
                onChange={(event) => {
                  void handleAttachmentSelection(event);
                }}
              />
            </label>
            {composerError ? <p className='error error-inline'>{composerError}</p> : null}
            {draftMediaItems.length > 0 ? (
              <ul className='draft-attachment-list'>
                {draftMediaItems.map((item) => (
                  <li key={item.id} className='draft-attachment-item'>
                    <div className='draft-attachment-content'>
                      <div className='draft-preview-frame'>
                        <img
                          className='draft-preview-image'
                          src={item.preview_url}
                          alt={`draft preview ${item.source_name}`}
                        />
                      </div>
                      <div>
                        <strong>{item.source_name}</strong>
                        {item.attachments.map((attachment) => (
                          <small key={`${attachment.role ?? attachment.mime}-${attachment.file_name ?? item.source_name}`}>
                            {attachment.role ?? 'attachment'} · {attachment.mime} ·{' '}
                            {formatBytes(attachment.byte_size)}
                          </small>
                        ))}
                      </div>
                    </div>
                    <button
                      className='button button-secondary'
                      type='button'
                      onClick={() => handleRemoveDraftAttachment(item.id)}
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
