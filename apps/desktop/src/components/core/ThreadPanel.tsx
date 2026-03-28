import { TimelineFeed } from './TimelineFeed';
import { type PostCardView, type ThreadPanelState } from './types';

type ThreadPanelProps = {
  state: ThreadPanelState;
  posts: PostCardView[];
  onOpenAuthor: (authorPubkey: string) => void;
  onOpenThread: (threadId: string) => void;
  onOpenThreadInTopic?: (threadId: string, topicId: string) => void;
  onReply: (post: PostCardView['post']) => void;
  onRepost?: (post: PostCardView['post']) => void;
  onQuoteRepost?: (post: PostCardView['post']) => void;
};

export function ThreadPanel({
  state,
  posts,
  onOpenAuthor,
  onOpenThread,
  onOpenThreadInTopic,
  onReply,
  onRepost,
  onQuoteRepost,
}: ThreadPanelProps) {
  return (
    <div className='shell-main-stack'>
      <TimelineFeed
        posts={posts}
        emptyCopy={state.emptyCopy}
        listClassName='thread-list'
        itemClassName='thread-item'
        onOpenAuthor={onOpenAuthor}
        onOpenThread={onOpenThread}
        onOpenThreadInTopic={onOpenThreadInTopic}
        onReply={onReply}
        onRepost={onRepost}
        onQuoteRepost={onQuoteRepost}
      />
    </div>
  );
}
