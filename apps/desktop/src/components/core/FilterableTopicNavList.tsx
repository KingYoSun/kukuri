import { useId, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { Input } from '@/components/ui/input';
import { Select } from '@/components/ui/select';

import { TopicNavList } from './TopicNavList';
import { type TopicDiagnosticSummary } from './types';

type TopicNavFilter = 'all' | 'connected' | 'disconnected';
type TopicNavSort = 'added' | 'name' | 'updated';

type TopicNavListProps = Parameters<typeof TopicNavList>[0];

// Wraps TopicNavList with search / filter / sort controls. The list grows as
// topics are tracked, so these controls keep it manageable. State is kept local
// (session-only); `items` is expected in added order (trackedTopics order).
export function FilterableTopicNavList({ items, ...listProps }: TopicNavListProps) {
  const { t } = useTranslation('shell');
  const [searchQuery, setSearchQuery] = useState('');
  const [filter, setFilter] = useState<TopicNavFilter>('all');
  const [sort, setSort] = useState<TopicNavSort>('added');
  const filterId = useId();
  const sortId = useId();

  const showControls = items.length > 1;

  const visibleItems = useMemo(() => {
    if (!showControls) {
      return items;
    }

    const query = searchQuery.trim().toLowerCase();
    const matchesQuery = (item: TopicDiagnosticSummary) => {
      if (!query) {
        return true;
      }
      if (item.topic.toLowerCase().includes(query)) {
        return true;
      }
      return Boolean(item.channels?.some((channel) => channel.label.toLowerCase().includes(query)));
    };

    const matchesFilter = (item: TopicDiagnosticSummary) => {
      const connected = item.gossipJoined !== false;
      if (filter === 'connected') {
        return connected;
      }
      if (filter === 'disconnected') {
        return !connected;
      }
      return true;
    };

    // Keep the original index so "added" sort and stable tie-breaking work even
    // after filtering reorders the array.
    const filtered = items
      .map((item, index) => ({ item, index }))
      .filter(({ item }) => matchesQuery(item) && matchesFilter(item));

    filtered.sort((a, b) => {
      if (sort === 'name') {
        return a.item.topic.localeCompare(b.item.topic) || a.index - b.index;
      }
      if (sort === 'updated') {
        const aAt = a.item.lastReceivedAt ?? null;
        const bAt = b.item.lastReceivedAt ?? null;
        if (aAt === bAt) {
          return a.index - b.index;
        }
        if (aAt === null) {
          return 1;
        }
        if (bAt === null) {
          return -1;
        }
        return bAt - aAt;
      }
      return a.index - b.index;
    });

    return filtered.map(({ item }) => item);
  }, [filter, items, searchQuery, showControls, sort]);

  return (
    <div className='topic-nav-browser'>
      {showControls ? (
        <div className='topic-nav-controls'>
          <Input
            type='search'
            className='topic-nav-search'
            value={searchQuery}
            onChange={(event) => setSearchQuery(event.target.value)}
            placeholder={t('navigation.searchPlaceholder')}
            aria-label={t('navigation.searchPlaceholder')}
          />
          <div className='topic-nav-control-row'>
            <Select
              id={filterId}
              className='topic-nav-select'
              value={filter}
              onChange={(event) => setFilter(event.target.value as TopicNavFilter)}
              aria-label={t('navigation.filterLabel')}
            >
              <option value='all'>{t('navigation.filterAll')}</option>
              <option value='connected'>{t('navigation.filterConnected')}</option>
              <option value='disconnected'>{t('navigation.filterDisconnected')}</option>
            </Select>
            <Select
              id={sortId}
              className='topic-nav-select'
              value={sort}
              onChange={(event) => setSort(event.target.value as TopicNavSort)}
              aria-label={t('navigation.sortLabel')}
            >
              <option value='added'>{t('navigation.sortAdded')}</option>
              <option value='name'>{t('navigation.sortName')}</option>
              <option value='updated'>{t('navigation.sortUpdated')}</option>
            </Select>
          </div>
        </div>
      ) : null}

      {visibleItems.length > 0 ? (
        <TopicNavList items={visibleItems} {...listProps} />
      ) : (
        <p className='topic-nav-empty'>{t('navigation.noFilteredTopics')}</p>
      )}
    </div>
  );
}
