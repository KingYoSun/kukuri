import { useMemo, useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { DEFAULT_PUBLIC_TOPIC_ID } from '@/constants/topics';
import i18n from '@/i18n';
import { Check, ChevronsUpDown, PlusCircle } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
} from '@/components/ui/command';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import type { PendingTopic } from '@/lib/api/tauri';
import { cn } from '@/lib/utils';
import { useTopicStore } from '@/stores';

interface TopicSelectorProps {
  value?: string;
  onValueChange: (value: string) => void;
  disabled?: boolean;
  placeholder?: string;
  onCreateTopicRequest?: () => void;
  dataTestId?: string;
}

export function TopicSelector({
  value,
  onValueChange,
  disabled = false,
  placeholder,
  onCreateTopicRequest,
  dataTestId,
}: TopicSelectorProps) {
  const { t } = useTranslation();
  const defaultPlaceholder = placeholder ?? t('topics.selector.selectPlaceholder');
  const [open, setOpen] = useState(false);
  const { topics, joinedTopics, pendingTopics } = useTopicStore();

  // 参加しているトピックのみフィルタリング
  const availableTopics = useMemo(
    () => Array.from(topics.values()).filter((topic) => joinedTopics.includes(topic.id)),
    [topics, joinedTopics],
  );
  const pendingTopicEntries = useMemo(
    () =>
      Array.from(pendingTopics.values()).sort((a, b) => (b.created_at ?? 0) - (a.created_at ?? 0)),
    [pendingTopics],
  );

  const selectedTopic = value ? topics.get(value) : null;

  const handleCreateTopic = useCallback(() => {
    if (!onCreateTopicRequest) {
      return;
    }
    onCreateTopicRequest();
    setOpen(false);
  }, [onCreateTopicRequest]);

  const renderPendingStatus = (status: PendingTopic['status']) => {
    switch (status) {
      case 'synced':
        return t('topics.selector.synced');
      case 'failed':
        return t('topics.selector.failed');
      default:
        return t('topics.selector.waiting');
    }
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          role="combobox"
          aria-expanded={open}
          className="w-full justify-between"
          disabled={disabled}
          data-testid={dataTestId}
        >
          {selectedTopic ? selectedTopic.name : defaultPlaceholder}
          <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-full p-0">
        <Command>
          <CommandInput placeholder={t('topics.selector.searchPlaceholder')} />
          <CommandEmpty>
            <div className="text-center text-sm space-y-2 p-4">
              <p>{t('topics.selector.notFound')}</p>
              {onCreateTopicRequest && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleCreateTopic}
                  data-testid="topic-selector-create-empty"
                >
                  <PlusCircle className="h-4 w-4 mr-1" />
                  {t('topics.selector.addNew')}
                </Button>
              )}
            </div>
          </CommandEmpty>
          {pendingTopicEntries.length > 0 && (
            <CommandGroup heading={t('topics.selector.pendingHeading')}>
              {pendingTopicEntries.map((pending) => (
                <CommandItem key={pending.pending_id} value={pending.name} disabled>
                  <div className="flex flex-col gap-0.5">
                    <div className="font-medium">{pending.name}</div>
                    <div className="text-xs text-muted-foreground">
                      {pending.status === 'failed'
                        ? pending.error_message || t('topics.selector.failedMessage')
                        : pending.status === 'synced'
                          ? t('topics.selector.syncedMessage')
                          : t('topics.selector.waitingMessage')}
                    </div>
                  </div>
                  <Badge
                    variant={
                      pending.status === 'failed'
                        ? 'destructive'
                        : pending.status === 'synced'
                          ? 'secondary'
                          : 'outline'
                    }
                  >
                    {renderPendingStatus(pending.status)}
                  </Badge>
                </CommandItem>
              ))}
            </CommandGroup>
          )}
          <CommandGroup>
            {availableTopics.length === 0 ? (
              <div className="p-2 text-sm text-muted-foreground text-center">
                {t('topics.selector.noJoinedTopics')}
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
                    {(() => {
                      const isPublicTopic = topic.id === DEFAULT_PUBLIC_TOPIC_ID;
                      const displayDescription = isPublicTopic
                        ? i18n.t('topics.publicTimeline')
                        : topic.description;
                      return (
                        displayDescription && (
                          <div className="text-xs text-muted-foreground">{displayDescription}</div>
                        )
                      );
                    })()}
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
                {t('topics.selector.addNew')}
              </CommandItem>
            )}
          </CommandGroup>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
