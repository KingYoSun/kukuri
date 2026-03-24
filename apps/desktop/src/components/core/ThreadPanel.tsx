import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';

import { TimelineFeed } from './TimelineFeed';
import { type PostCardView, type ThreadPanelState } from './types';

type ThreadPanelProps = {
  state: ThreadPanelState;
  posts: PostCardView[];
  onClearThread: () => void;
  onOpenAuthor: (authorPubkey: string) => void;
  onOpenThread: (threadId: string) => void;
  onReply: (post: PostCardView['post']) => void;
};

export function ThreadPanel({
  state,
  posts,
  onClearThread,
  onOpenAuthor,
  onOpenThread,
  onReply,
}: ThreadPanelProps) {
  return (
    <div className='shell-main-stack'>
      <Card>
        <CardHeader>
          <h3>Thread</h3>
          {state.selectedThreadId ? (
            <Button variant='secondary' type='button' onClick={onClearThread}>
              Clear Thread
            </Button>
          ) : null}
        </CardHeader>
        <p className='lede'>{state.summary}</p>
      </Card>

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
