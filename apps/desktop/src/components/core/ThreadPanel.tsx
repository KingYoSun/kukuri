import { TimelineFeed } from './TimelineFeed';
import { type PostCardView, type ThreadPanelState } from './types';

type ThreadPanelProps = {
  state: ThreadPanelState;
  posts: PostCardView[];
  onOpenAuthor: (authorPubkey: string) => void;
  onOpenThread: (threadId: string) => void;
  onReply: (post: PostCardView['post']) => void;
};

export function ThreadPanel({
  state,
  posts,
  onOpenAuthor,
  onOpenThread,
  onReply,
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
        onReply={onReply}
      />
    </div>
  );
}
