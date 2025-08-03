import { useState } from 'react';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Button } from '@/components/ui/button';
import { SmilePlus } from 'lucide-react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { NostrAPI } from '@/lib/api/tauri';
import { toast } from 'sonner';

interface ReactionPickerProps {
  postId: string;
  topicId: string;
}

// よく使われるリアクション絵文字
const POPULAR_REACTIONS = [
  '👍', '❤️', '😄', '😂', '😮', '😢', '😡', '🔥',
  '💯', '🎉', '🚀', '👀', '🤔', '👏', '💪', '🙏',
];

export function ReactionPicker({ postId, topicId }: ReactionPickerProps) {
  const [isOpen, setIsOpen] = useState(false);
  const queryClient = useQueryClient();

  const reactionMutation = useMutation({
    mutationFn: async (reaction: string) => {
      await NostrAPI.sendReaction(postId, reaction);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['timeline'] });
      queryClient.invalidateQueries({ queryKey: ['posts', topicId] });
      toast.success('リアクションを送信しました');
      setIsOpen(false);
    },
    onError: () => {
      toast.error('リアクションの送信に失敗しました');
    },
  });

  const handleReaction = (reaction: string) => {
    reactionMutation.mutate(reaction);
  };

  return (
    <Popover open={isOpen} onOpenChange={setIsOpen}>
      <PopoverTrigger asChild>
        <Button variant="ghost" size="sm" disabled={reactionMutation.isPending}>
          <SmilePlus className="h-4 w-4" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-64 p-2">
        <div className="grid grid-cols-4 gap-1">
          {POPULAR_REACTIONS.map((reaction) => (
            <button
              key={reaction}
              onClick={() => handleReaction(reaction)}
              className="p-2 text-2xl hover:bg-accent rounded transition-colors"
              disabled={reactionMutation.isPending}
            >
              {reaction}
            </button>
          ))}
        </div>
      </PopoverContent>
    </Popover>
  );
}