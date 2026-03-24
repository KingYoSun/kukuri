import { PostCard } from './PostCard';
import { type PostCardView } from './types';

type TimelineFeedProps = {
  posts: PostCardView[];
  emptyCopy: string;
  listClassName?: string;
  itemClassName?: string;
  onOpenAuthor: (authorPubkey: string) => void;
  onOpenThread: (threadId: string) => void;
  onReply: (post: PostCardView['post']) => void;
};

export function TimelineFeed({
  posts,
  emptyCopy,
  listClassName = 'post-list',
  itemClassName,
  onOpenAuthor,
  onOpenThread,
  onReply,
}: TimelineFeedProps) {
  if (posts.length === 0) {
    return <p className='empty'>{emptyCopy}</p>;
  }

  return (
    <ul className={listClassName}>
      {posts.map((view) => (
        <li key={view.post.object_id} className={itemClassName}>
          <PostCard
            view={view}
            onOpenAuthor={onOpenAuthor}
            onOpenThread={onOpenThread}
            onReply={onReply}
          />
        </li>
      ))}
    </ul>
  );
}
