import { FormEvent, startTransition, useCallback, useEffect, useMemo, useState } from 'react';

import { DesktopApi, PostView, SyncStatus, runtimeApi } from './lib/api';

type AppProps = {
  api?: DesktopApi;
};

const DEFAULT_TOPIC = 'kukuri:topic:demo';
const REFRESH_INTERVAL_MS = 2000;

export function App({ api = runtimeApi }: AppProps) {
  const [topic, setTopic] = useState(DEFAULT_TOPIC);
  const [composer, setComposer] = useState('');
  const [timeline, setTimeline] = useState<PostView[]>([]);
  const [thread, setThread] = useState<PostView[]>([]);
  const [selectedThread, setSelectedThread] = useState<string | null>(null);
  const [peerTicket, setPeerTicket] = useState('');
  const [localPeerTicket, setLocalPeerTicket] = useState<string | null>(null);
  const [syncStatus, setSyncStatus] = useState<SyncStatus>({
    connected: false,
    peer_count: 0,
    pending_events: 0,
    subscribed_topics: [],
  });
  const [error, setError] = useState<string | null>(null);

  const headline = useMemo(
    () => (syncStatus.connected ? 'Live over static peers' : 'Local-first shell'),
    [syncStatus.connected]
  );

  const loadTopic = useCallback(async (currentTopic: string) => {
    try {
      const [timelineView, status, ticket] = await Promise.all([
        api.listTimeline(currentTopic, null, 50),
        api.getSyncStatus(),
        api.getLocalPeerTicket(),
      ]);
      startTransition(() => {
        setTimeline(timelineView.items);
        setSyncStatus(status);
        setLocalPeerTicket(ticket);
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
      await loadTopic(topic);
    };

    void refresh();
    const intervalId = window.setInterval(() => {
      void refresh();
    }, REFRESH_INTERVAL_MS);

    return () => {
      disposed = true;
      window.clearInterval(intervalId);
    };
  }, [loadTopic, topic]);

  async function handlePublish(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!composer.trim()) {
      return;
    }

    try {
      await api.createPost(topic, composer.trim(), selectedThread);
      setComposer('');
      await loadTopic(topic);
      if (selectedThread) {
        await openThread(selectedThread);
      }
    } catch (publishError) {
      setError(publishError instanceof Error ? publishError.message : 'failed to publish');
    }
  }

  async function openThread(threadId: string) {
    try {
      const threadView = await api.listThread(topic, threadId, null, 50);
      startTransition(() => {
        setSelectedThread(threadId);
        setThread(threadView.items);
      });
    } catch (threadError) {
      setError(threadError instanceof Error ? threadError.message : 'failed to load thread');
    }
  }

  async function handleImportPeer() {
    if (!peerTicket.trim()) {
      return;
    }
    try {
      await api.importPeerTicket(peerTicket.trim());
      setPeerTicket('');
      await loadTopic(topic);
    } catch (importError) {
      setError(importError instanceof Error ? importError.message : 'failed to import peer');
    }
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
          <span>Topic</span>
          <input value={topic} onChange={(e) => setTopic(e.target.value)} />
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
        </aside>

        <section className='panel'>
          <div className='panel-header'>
            <h2>Timeline</h2>
            <button className='button button-secondary' onClick={() => void loadTopic(topic)}>
              Refresh
            </button>
          </div>
          <form className='composer' onSubmit={handlePublish}>
            <textarea
              value={composer}
              onChange={(e) => setComposer(e.target.value)}
              placeholder='Write a post'
            />
            <button className='button' type='submit'>
              Publish
            </button>
          </form>
          <ul className='post-list'>
            {timeline.map((post) => (
              <li key={post.id}>
                <button className='post-card' onClick={() => void openThread(post.root_id ?? post.id)}>
                  <div className='post-meta'>
                    <span>{post.author_npub}</span>
                    <span>{new Date(post.created_at * 1000).toLocaleTimeString('ja-JP')}</span>
                  </div>
                  <strong>{post.content}</strong>
                  <small>{post.note_id}</small>
                </button>
              </li>
            ))}
          </ul>
        </section>

        <section className='panel'>
          <h2>Thread</h2>
          {selectedThread ? (
            <ul className='thread-list'>
              {thread.map((post) => (
                <li key={post.id} className='thread-item'>
                  <strong>{post.content}</strong>
                  <small>{post.author_npub}</small>
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
