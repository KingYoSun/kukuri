import { useMemo, useState, useCallback } from 'react';
import { Check, ChevronsUpDown, PlusCircle } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
} from '@/components/ui/command';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { useTopicStore } from '@/stores';

interface TopicSelectorProps {
  value?: string;
  onValueChange: (value: string) => void;
  disabled?: boolean;
  placeholder?: string;
  onCreateTopicRequest?: () => void;
}

export function TopicSelector({
  value,
  onValueChange,
  disabled = false,
  placeholder = 'トピックを選択',
  onCreateTopicRequest,
}: TopicSelectorProps) {
  const [open, setOpen] = useState(false);
  const { topics, joinedTopics } = useTopicStore();

  // 参加しているトピックのみフィルタリング
  const availableTopics = useMemo(
    () => Array.from(topics.values()).filter((topic) => joinedTopics.includes(topic.id)),
    [topics, joinedTopics],
  );

  const selectedTopic = value ? topics.get(value) : null;

  const handleCreateTopic = useCallback(() => {
    if (!onCreateTopicRequest) {
      return;
    }
    onCreateTopicRequest();
    setOpen(false);
  }, [onCreateTopicRequest]);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          role="combobox"
          aria-expanded={open}
          className="w-full justify-between"
          disabled={disabled}
        >
          {selectedTopic ? selectedTopic.name : placeholder}
          <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-full p-0">
        <Command>
          <CommandInput placeholder="トピックを検索..." />
          <CommandEmpty>
            <div className="text-center text-sm space-y-2 p-4">
              <p>トピックが見つかりません</p>
              {onCreateTopicRequest && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleCreateTopic}
                  data-testid="topic-selector-create-empty"
                >
                  <PlusCircle className="h-4 w-4 mr-1" />
                  新しいトピックを作成
                </Button>
              )}
            </div>
          </CommandEmpty>
          <CommandGroup>
            {availableTopics.length === 0 ? (
              <div className="p-2 text-sm text-muted-foreground text-center">
                参加しているトピックがありません
              </div>
            ) : (
              availableTopics.map((topic) => (
                <CommandItem
                  key={topic.id}
                  value={topic.name}
                  onSelect={() => {
                    onValueChange(topic.id);
                    setOpen(false);
                  }}
                >
                  <Check
                    className={cn('mr-2 h-4 w-4', value === topic.id ? 'opacity-100' : 'opacity-0')}
                  />
                  <div className="flex-1">
                    <div className="font-medium">{topic.name}</div>
                    {topic.description && (
                      <div className="text-xs text-muted-foreground">{topic.description}</div>
                    )}
                  </div>
                </CommandItem>
              ))
            )}
            {onCreateTopicRequest && (
              <CommandItem
                value="__create_topic__"
                onSelect={handleCreateTopic}
                className="text-primary focus:text-primary"
                data-testid="topic-selector-create"
              >
                <PlusCircle className="mr-2 h-4 w-4" />
                新しいトピックを作成
              </CommandItem>
            )}
          </CommandGroup>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
