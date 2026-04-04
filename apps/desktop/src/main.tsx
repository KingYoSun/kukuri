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
      ],
    },
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
