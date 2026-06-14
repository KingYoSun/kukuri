import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from 'react';

import { buildMentionToken } from '@/lib/internalLinks';

import { type MentionCandidate } from './types';

const DEFAULT_MAX_ITEMS = 8;

type ActiveMention = {
  start: number;
  query: string;
  queryEnd: number;
};

type MentionState = ActiveMention | null;

export function detectActiveMention(value: string, caret: number): ActiveMention | null {
  let index = caret - 1;
  while (index >= 0) {
    const char = value[index];
    if (char === '@') {
      const prev = index > 0 ? value[index - 1] : '';
      if (index === 0 || /\s/.test(prev)) {
        return { start: index, query: value.slice(index + 1, caret), queryEnd: caret };
      }
      return null;
    }
    if (/\s/.test(char)) {
      return null;
    }
    index -= 1;
  }
  return null;
}

function filterCandidates(
  candidates: MentionCandidate[],
  query: string,
  maxItems: number
): MentionCandidate[] {
  const normalized = query.toLowerCase();
  const matched =
    normalized.length === 0
      ? candidates
      : candidates.filter(
          (candidate) =>
            candidate.label.toLowerCase().includes(normalized) ||
            (candidate.displayName?.toLowerCase().includes(normalized) ?? false) ||
            (candidate.name?.toLowerCase().includes(normalized) ?? false) ||
            candidate.pubkey.toLowerCase().startsWith(normalized)
        );
  return matched.slice(0, maxItems);
}

type UseMentionAutocompleteArgs = {
  value: string;
  candidates: MentionCandidate[];
  onValueChange?: (next: string) => void;
  maxItems?: number;
};

export function useMentionAutocomplete({
  value,
  candidates,
  onValueChange,
  maxItems = DEFAULT_MAX_ITEMS,
}: UseMentionAutocompleteArgs) {
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const pendingCaretRef = useRef<number | null>(null);
  // The token the user dismissed with Escape; suppresses re-opening on the same
  // token until the query changes or the caret moves elsewhere.
  const dismissedRef = useRef<string | null>(null);
  const [mention, setMention] = useState<MentionState>(null);
  const [activeIndex, setActiveIndex] = useState(0);
  const enabled = Boolean(onValueChange);

  const items = useMemo(
    () => (mention ? filterCandidates(candidates, mention.query, maxItems) : []),
    [candidates, mention, maxItems]
  );
  const isOpen = enabled && mention !== null && items.length > 0;
  const safeActiveIndex = items.length > 0 ? Math.min(activeIndex, items.length - 1) : 0;

  // Controlled textareas reset the caret to the end after a programmatic value
  // change; restore the intended caret position once the new value is applied.
  useEffect(() => {
    if (pendingCaretRef.current === null) {
      return;
    }
    const caret = pendingCaretRef.current;
    pendingCaretRef.current = null;
    const element = textareaRef.current;
    if (!element) {
      return;
    }
    const frame = requestAnimationFrame(() => {
      element.focus();
      element.setSelectionRange(caret, caret);
    });
    return () => cancelAnimationFrame(frame);
  }, [value]);

  const recompute = () => {
    // Skip while a programmatic value change is in flight: the DOM value/caret
    // still reflect the pre-insertion text and would re-open the dropdown.
    if (!enabled || pendingCaretRef.current !== null) {
      return;
    }
    const element = textareaRef.current;
    if (!element) {
      return;
    }
    const caret = element.selectionStart ?? element.value.length;
    const detected = detectActiveMention(element.value, caret);
    if (detected) {
      const key = `${detected.start}:${detected.query}`;
      if (dismissedRef.current === key) {
        setMention((previous) => (previous === null ? previous : null));
        return;
      }
      dismissedRef.current = null;
    } else {
      dismissedRef.current = null;
    }
    setMention((previous) => {
      if (!detected) {
        return previous === null ? previous : null;
      }
      if (previous && previous.start === detected.start && previous.query === detected.query) {
        return previous;
      }
      return detected;
    });
    if (detected && (!mention || mention.start !== detected.start)) {
      setActiveIndex(0);
    }
  };

  const selectCandidate = (candidate: MentionCandidate) => {
    const element = textareaRef.current;
    if (!enabled || !onValueChange || !element || !mention) {
      return;
    }
    const token = buildMentionToken(candidate.label, candidate.pubkey);
    const insertText = `${token} `;
    const before = element.value.slice(0, mention.start);
    const after = element.value.slice(mention.queryEnd);
    pendingCaretRef.current = mention.start + insertText.length;
    onValueChange(`${before}${insertText}${after}`);
    setMention(null);
    setActiveIndex(0);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (!isOpen) {
      return;
    }
    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        setActiveIndex((index) => (index + 1) % items.length);
        break;
      case 'ArrowUp':
        event.preventDefault();
        setActiveIndex((index) => (index - 1 + items.length) % items.length);
        break;
      case 'Enter':
      case 'Tab':
        event.preventDefault();
        event.stopPropagation();
        selectCandidate(items[safeActiveIndex]);
        break;
      case 'Escape':
        event.preventDefault();
        dismissedRef.current = mention ? `${mention.start}:${mention.query}` : null;
        setMention(null);
        setActiveIndex(0);
        break;
      default:
        break;
    }
  };

  return {
    textareaRef,
    isOpen,
    items,
    activeIndex: safeActiveIndex,
    query: mention?.query ?? null,
    onKeyDown,
    onSelectionChange: recompute,
    selectCandidate,
    setActiveIndex,
  };
}
