import type { Meta, StoryObj } from '@storybook/react-vite';

import { PostCard } from './PostCard';
import { type PostCardView } from './types';

const basePost = {
  object_id: 'post-1',
  envelope_id: 'envelope-post-1',
  author_pubkey: 'a'.repeat(64),
  author_name: 'alice',
  author_display_name: 'Alice',
  following: false,
  followed_by: false,
  mutual: false,
  friend_of_friend: false,
  object_kind: 'post',
  content: 'Core product flow draft post',
  content_status: 'Available' as const,
  attachments: [],
  created_at: 1,
  reply_to: null,
  root_id: 'post-1',
  channel_id: null,
  audience_label: 'Public',
};

const inviteTokenPostContent = JSON.stringify({
  envelope: {
    kind: 'channel-invite',
    pubkey: 'b'.repeat(64),
    content: JSON.stringify({
      channel_id: 'channel-core',
      topic_id: 'kukuri:topic:demo',
      channel_label: 'Core Contributors',
      owner_pubkey: 'b'.repeat(64),
      epoch_id: 'epoch-1',
    }),
  },
});

function createView(overrides?: Partial<PostCardView>): PostCardView {
  return {
    post: basePost,
    context: 'timeline',
    authorLabel: 'Alice',
    authorPicture: null,
    relationshipLabel: null,
    audienceChipLabel: 'Public',
    threadTargetId: 'post-1',
    media: {
      objectId: 'post-1',
      kind: null,
      extraAttachmentCount: 0,
      state: 'ready',
      metaMime: null,
      metaBytesLabel: null,
      imagePreviewSrc: null,
      videoPosterPreviewSrc: null,
      videoPlaybackSrc: null,
      videoUnsupportedOnClient: false,
    },
    ...overrides,
  };
}

const meta = {
  title: 'Core/PostCard',
  parameters: {
    layout: 'centered',
  },
  render: (args) => (
    <div className='w-[min(42rem,calc(100vw-2rem))]'>
      <PostCard
        view={args.view}
        onOpenAuthor={() => undefined}
        onOpenThread={() => undefined}
        onReply={() => undefined}
      />
    </div>
  ),
} satisfies Meta<{ view: PostCardView }>;

export default meta;

type Story = StoryObj<typeof meta>;

export const ImagePending: Story = {
  args: {
    view: createView({
      media: {
        objectId: 'image-post',
        kind: 'image',
        extraAttachmentCount: 0,
        state: 'loading',
        metaMime: 'image/png',
        metaBytesLabel: '2.0 KB',
        imagePreviewSrc: null,
        videoPosterPreviewSrc: null,
        videoPlaybackSrc: null,
        videoUnsupportedOnClient: false,
      },
    }),
  },
};

export const ImageReady: Story = {
  args: {
    view: createView({
      media: {
        objectId: 'image-post',
        kind: 'image',
        extraAttachmentCount: 0,
        state: 'ready',
        metaMime: 'image/png',
        metaBytesLabel: '2.0 KB',
        imagePreviewSrc:
          'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360"><rect width="640" height="360" fill="%2300b3a4"/><circle cx="180" cy="120" r="42" fill="%23ffd36e"/><path d="M0 280l120-100 80 70 110-130 130 160H0z" fill="%230f2231"/></svg>',
        imageGalleryItems: [
          {
            hash: 'image-1',
            src: 'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360"><rect width="640" height="360" fill="%2300b3a4"/><circle cx="180" cy="120" r="42" fill="%23ffd36e"/><path d="M0 280l120-100 80 70 110-130 130 160H0z" fill="%230f2231"/></svg>',
            mime: 'image/png',
          },
          {
            hash: 'image-2',
            src: 'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360"><rect width="640" height="360" fill="%23f59d62"/><path d="M0 260l140-90 120 80 140-130 240 140H0z" fill="%23101923"/></svg>',
            mime: 'image/png',
          },
        ],
        currentImageIndex: 0,
        videoPosterPreviewSrc: null,
        videoPlaybackSrc: null,
        videoUnsupportedOnClient: false,
      },
    }),
  },
};

export const VideoPosterOnly: Story = {
  args: {
    view: createView({
      relationshipLabel: 'friend of friend',
      media: {
        objectId: 'video-post',
        kind: 'video',
        extraAttachmentCount: 1,
        state: 'ready',
        metaMime: 'video/mp4',
        metaBytesLabel: '8.0 KB',
        imagePreviewSrc: null,
        videoPosterPreviewSrc:
          'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360"><rect width="640" height="360" fill="%23101923"/><rect x="110" y="70" width="420" height="220" rx="24" fill="%23f59d62"/><polygon points="285,145 285,215 355,180" fill="%23101923"/></svg>',
        videoPlaybackSrc: null,
        videoUnsupportedOnClient: false,
      },
    }),
  },
};

export const VideoPlayable: Story = {
  args: {
    view: createView({
      relationshipLabel: 'mutual',
      media: {
        objectId: 'video-post',
        kind: 'video',
        extraAttachmentCount: 0,
        state: 'ready',
        metaMime: 'video/mp4',
        metaBytesLabel: '8.0 KB',
        imagePreviewSrc: null,
        videoPosterPreviewSrc:
          'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360"><rect width="640" height="360" fill="%23101923"/><rect x="110" y="70" width="420" height="220" rx="24" fill="%2300b3a4"/></svg>',
        videoPlaybackSrc:
          'data:video/mp4;base64,AAAAIGZ0eXBpc29tAAACAGlzb21pc28yYXZjMW1wNDEAAABsbXZoZAAAAAAAAAAAAAAAAAAAA+gAAAPoAAEAAAEAAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAIAAAIVdHJhawAAAFx0a2hkAAAAAwAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAEAAAAAAAAAAAAAAAAAAAAAAAEAAAAAAAAAAAAAAAAAAAABAAAAAAAAAAAAAAAAAABAAAAAAQAAAEAAAAAAAAkZWR0cwAAABxlbHN0AAAAAAAAAAEAAAPoAAAAAAABAAAAAAEAAAAqbWRpYQAAACBtZGhkAAAAAAAAAAAAAAAAAAAyAAAAMgBVxAAAAAAALWhkbHIAAAAAAAAAAHZpZGUAAAAAAAAAAAAAAABWaWRlb0hhbmRsZXIAAAAClW1pbmYAAAAUdm1oZAAAAAEAAAAAAAAAAAAAACRkaW5mAAAAHGRyZWYAAAAAAAAAAQAAAAx1cmwgAAAAAQAAAl1zdGJsAAAArXN0c2QAAAAAAAAAAQAAAJ1hdmMxAAAAAAAAAAEAAAAAAAAAAAAAAAAAAAAAAQABAAABAAABAAAAAAAAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABj//wAAADNhdmNDAWQAHv/hABdnZAAerNlChAAAAMAEAAAAwHihUkA=',
        videoUnsupportedOnClient: false,
      },
    }),
  },
};

export const UnsupportedVideo: Story = {
  args: {
    view: createView({
      media: {
        objectId: 'video-post',
        kind: 'video',
        extraAttachmentCount: 0,
        state: 'ready',
        metaMime: 'video/mp4',
        metaBytesLabel: '8.0 KB',
        imagePreviewSrc: null,
        videoPosterPreviewSrc:
          'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360"><rect width="640" height="360" fill="%23101923"/><text x="50%" y="52%" dominant-baseline="middle" text-anchor="middle" fill="%23f6f1e8" font-size="40">unsupported</text></svg>',
        videoPlaybackSrc: null,
        videoUnsupportedOnClient: true,
      },
    }),
  },
};

export const ChannelAccessToken: Story = {
  args: {
    view: createView({
      post: {
        ...basePost,
        object_id: 'token-post',
        envelope_id: 'envelope-token-post',
        content: inviteTokenPostContent,
      },
      threadTargetId: 'token-post',
    }),
  },
};

export const PureRepost: Story = {
  args: {
    view: createView({
      canReply: false,
      threadTargetId: 'source-root',
      threadTopicId: 'kukuri:topic:source',
      repostSourceAuthor: { pubkey: 'b'.repeat(64), label: 'Source Author', picture: null },
      post: {
        ...basePost,
        object_kind: 'repost',
        content: '',
        repost_commentary: null,
        repost_of: {
          source_object_id: 'source-1',
          source_topic_id: 'kukuri:topic:source',
          source_author_pubkey: 'b'.repeat(64),
          source_author_display_name: 'Source Author',
          source_author_name: null,
          source_object_kind: 'post',
          content: 'The original post is now the star of a repost, with a subtle attribution above.',
          attachments: [],
          reply_to: null,
          root_id: 'source-root',
        },
      },
    }),
  },
};

export const QuoteRepost: Story = {
  args: {
    view: createView({
      repostSourceAuthor: { pubkey: 'b'.repeat(64), label: 'Source Author', picture: null },
      post: {
        ...basePost,
        object_kind: 'repost',
        content: 'Adding my own take on top of the original.',
        repost_commentary: 'Adding my own take on top of the original.',
        repost_of: {
          source_object_id: 'source-1',
          source_topic_id: 'kukuri:topic:source',
          source_author_pubkey: 'b'.repeat(64),
          source_author_display_name: 'Source Author',
          source_author_name: null,
          source_object_kind: 'post',
          content: 'The quoted post sits in an embedded card with the author avatar.',
          attachments: [],
          reply_to: null,
          root_id: 'source-1',
        },
      },
    }),
  },
};

export const Reply: Story = {
  args: {
    view: createView({
      replyParentAuthor: { pubkey: 'b'.repeat(64), label: 'Parent Author', picture: null },
      post: {
        ...basePost,
        content: 'This reply shows a compact context line above its own body.',
        reply_to: 'parent-1',
        reply_preview: {
          object_id: 'parent-1',
          topic: 'kukuri:topic:demo',
          author: {
            pubkey: 'b'.repeat(64),
            name: 'parent-author',
            display_name: 'Parent Author',
            picture: null,
            picture_asset: null,
          },
          content: 'The original post being replied to.',
          attachments: [],
          root_id: 'parent-1',
          reply_to: null,
        },
      },
    }),
  },
};
