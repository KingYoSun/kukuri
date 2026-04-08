import React from 'react';
import ReactDOM from 'react-dom/client';

import '@/i18n';
import { installWindowDesktopMock } from '@/mocks/installWindowDesktopMock';
import { App } from './App';
import '@/styles/index.css';

if (import.meta.env.VITE_KUKURI_DESKTOP_MOCK === '1') {
  installWindowDesktopMock({
    seedPosts: {
      'kukuri:topic:demo': [
        {
          object_id: 'browser-seed-post',
          envelope_id: 'browser-seed-envelope',
          author_pubkey: 'b'.repeat(64),
          author_name: 'browser peer',
          author_display_name: null,
          following: true,
          followed_by: true,
          mutual: true,
          friend_of_friend: false,
          object_kind: 'post',
          content: 'browser mock peer post',
          content_status: 'Available',
          attachments: [],
          created_at: 1,
          reply_to: null,
          root_id: 'browser-seed-post',
          audience_label: 'Public',
        },
        {
          object_id: 'browser-seed-reply',
          envelope_id: 'browser-seed-reply-envelope',
          author_pubkey: 'b'.repeat(64),
          author_name: 'browser peer',
          author_display_name: null,
          following: true,
          followed_by: true,
          mutual: true,
          friend_of_friend: false,
          object_kind: 'comment',
          content: 'browser mock reply body',
          content_status: 'Available',
          attachments: [],
          created_at: 2,
          reply_to: 'browser-seed-post',
          root_id: 'browser-seed-post',
          audience_label: 'Public',
        },
      ],
    },
    notifications: [
      {
        notification_id: 'browser-notification-reply',
        kind: 'reply',
        actor_pubkey: 'b'.repeat(64),
        actor_name: 'browser peer',
        actor_display_name: null,
        actor_picture: null,
        actor_picture_asset: null,
        source_envelope_id: 'browser-seed-reply-envelope',
        source_replica_id: 'replica:browser-mock',
        topic_id: 'kukuri:topic:demo',
        channel_id: null,
        object_id: 'browser-seed-reply',
        thread_root_object_id: 'browser-seed-post',
        dm_id: null,
        message_id: null,
        preview_text: 'browser mock reply notification',
        created_at: 2,
        received_at: 2,
        read_at: null,
      },
    ],
    authorSocialViews: {
      ['b'.repeat(64)]: {
        name: 'browser peer',
        following: true,
        followed_by: true,
        mutual: true,
      },
      ['f'.repeat(64)]: {
        following: true,
        followed_by: true,
        mutual: true,
      },
    },
  });
}

if (import.meta.env.DEV) {
  console.info('[kukuri.desktop] frontend boot');
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
