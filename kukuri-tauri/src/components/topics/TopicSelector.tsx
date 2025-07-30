import { useState } from 'react';
import { Check, ChevronsUpDown } from 'lucide-react';
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
}

export function TopicSelector({
  value,
  onValueChange,
  disabled = false,
  placeholder = 'トピックを選択',
}: TopicSelectorProps) {
  const [open, setOpen] = useState(false);
  const { topics, joinedTopics } = useTopicStore();

  // 参加しているトピックのみフィルタリング
  const availableTopics = Array.from(topics.values()).filter((topic) =>
    joinedTopics.includes(topic.id),
  );

  const selectedTopic = value ? topics.get(value) : null;

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
          <CommandEmpty>トピックが見つかりません</CommandEmpty>
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
          </CommandGroup>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
