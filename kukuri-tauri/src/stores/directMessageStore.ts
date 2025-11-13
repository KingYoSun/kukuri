import { create } from 'zustand';

import { errorHandler } from '@/lib/errorHandler';
import type { DirectMessageItem } from '@/lib/api/tauri';

export type DirectMessageDeliveryStatus = 'pending' | 'sent' | 'failed';

export interface DirectMessageModel {
  eventId: string | null;
  clientMessageId: string;
  senderNpub: string;
  recipientNpub: string;
  content: string;
  createdAt: number;
  status: DirectMessageDeliveryStatus;
}

export const mapApiMessageToModel = (item: DirectMessageItem): DirectMessageModel => ({
  eventId: item.eventId,
  clientMessageId:
    item.clientMessageId ?? item.eventId ?? `generated-${item.senderNpub}-${item.createdAt}`,
  senderNpub: item.senderNpub,
  recipientNpub: item.recipientNpub,
  content: item.content,
  createdAt: item.createdAt,
  status: item.delivered ? 'sent' : 'pending',
});

interface SetMessagesOptions {
  replace?: boolean;
}

interface ReceiveMessageOptions {
  incrementUnread?: boolean;
  incrementAmount?: number;
}

interface DirectMessageConversationHydration {
  conversationNpub: string;
  unreadCount: number;
  lastReadAt: number;
  lastMessage?: DirectMessageModel;
}

interface DirectMessageStoreState {
  isDialogOpen: boolean;
  isInboxOpen: boolean;
  activeConversationNpub: string | null;
  messageDraft: string;
  isSending: boolean;
  conversations: Record<string, DirectMessageModel[]>;
  optimisticMessages: Record<string, DirectMessageModel[]>;
  unreadCounts: Record<string, number>;
  conversationReadTimestamps: Record<string, number>;
  openDialog: (conversationNpub: string) => void;
  closeDialog: () => void;
  openInbox: () => void;
  closeInbox: () => void;
  setDraft: (draft: string) => void;
  setIsSending: (isSending: boolean) => void;
  appendOptimisticMessage: (conversationNpub: string, message: DirectMessageModel) => void;
  resolveOptimisticMessage: (
    conversationNpub: string,
    clientMessageId: string,
    eventId?: string | null,
  ) => void;
  failOptimisticMessage: (
    conversationNpub: string,
    clientMessageId: string,
    error?: unknown,
  ) => void;
  setMessages: (
    conversationNpub: string,
    messages: DirectMessageModel[],
    options?: SetMessagesOptions,
  ) => void;
  receiveIncomingMessage: (
    conversationNpub: string,
    message: DirectMessageModel,
    options?: ReceiveMessageOptions,
  ) => void;
  removeOptimisticMessage: (conversationNpub: string, clientMessageId: string) => void;
  markConversationAsRead: (conversationNpub: string, lastReadAt?: number | null) => void;
  incrementUnreadCount: (conversationNpub: string, amount?: number) => void;
  hydrateConversations: (summaries: DirectMessageConversationHydration[]) => void;
  reset: () => void;
}

const createDialogState = () => ({
  isDialogOpen: false,
  activeConversationNpub: null as string | null,
  messageDraft: '',
  isSending: false,
});

const createInitialState = (): Omit<
  DirectMessageStoreState,
  | 'openDialog'
  | 'closeDialog'
  | 'openInbox'
  | 'closeInbox'
  | 'setDraft'
  | 'setIsSending'
  | 'appendOptimisticMessage'
  | 'resolveOptimisticMessage'
  | 'failOptimisticMessage'
  | 'setMessages'
  | 'receiveIncomingMessage'
  | 'removeOptimisticMessage'
  | 'markConversationAsRead'
  | 'incrementUnreadCount'
  | 'hydrateConversations'
  | 'reset'
> => ({
  ...createDialogState(),
  isInboxOpen: false,
  conversations: {},
  optimisticMessages: {},
  unreadCounts: {},
  conversationReadTimestamps: {},
});

const dedupeMessages = (messages: DirectMessageModel[]) => {
  const seen = new Set<string>();
  const sorted = [...messages].sort((a, b) => a.createdAt - b.createdAt);
  const result: DirectMessageModel[] = [];

  for (const message of sorted) {
    const dedupeKey = `${message.eventId ?? 'pending'}:${message.clientMessageId}`;
    if (seen.has(dedupeKey)) {
      continue;
    }
    seen.add(dedupeKey);
    result.push(message);
  }

  return result;
};

export const getDirectMessageInitialState = () => createInitialState();

export const useDirectMessageStore = create<DirectMessageStoreState>((set, _get) => ({
  ...createInitialState(),
  openDialog: (conversationNpub) =>
    set((state) => ({
      ...state,
      isDialogOpen: true,
      isInboxOpen: false,
      activeConversationNpub: conversationNpub,
      unreadCounts: {
        ...state.unreadCounts,
        [conversationNpub]: 0,
      },
    })),
  closeDialog: () => set((state) => ({ ...state, ...createDialogState() })),
  openInbox: () =>
    set((state) => ({
      ...state,
      isInboxOpen: true,
    })),
  closeInbox: () =>
    set((state) => ({
      ...state,
      isInboxOpen: false,
    })),
  setDraft: (draft) => set({ messageDraft: draft }),
  setIsSending: (isSending) => set({ isSending }),
  appendOptimisticMessage: (conversationNpub, message) =>
    set((state) => {
      const existing = state.optimisticMessages[conversationNpub] ?? [];
      return {
        ...state,
        optimisticMessages: {
          ...state.optimisticMessages,
          [conversationNpub]: [...existing, message],
        },
      };
    }),
  resolveOptimisticMessage: (conversationNpub, clientMessageId, eventId) =>
    set((state) => {
      const queue = state.optimisticMessages[conversationNpub] ?? [];
      const target = queue.find((message) => message.clientMessageId === clientMessageId);
      const remaining = queue.filter((message) => message.clientMessageId !== clientMessageId);
      const confirmed = target
        ? [
            ...(state.conversations[conversationNpub] ?? []),
            {
              ...target,
              status: 'sent' as const,
              eventId: eventId ?? target.eventId,
            },
          ]
        : (state.conversations[conversationNpub] ?? []);

      return {
        ...state,
        conversations: {
          ...state.conversations,
          [conversationNpub]: dedupeMessages(confirmed),
        },
        optimisticMessages: {
          ...state.optimisticMessages,
          [conversationNpub]: remaining,
        },
        messageDraft: state.activeConversationNpub === conversationNpub ? '' : state.messageDraft,
        isSending: state.activeConversationNpub === conversationNpub ? false : state.isSending,
      };
    }),
  failOptimisticMessage: (conversationNpub, clientMessageId, error) =>
    set((state) => {
      if (error) {
        errorHandler.log('DirectMessageStore.sendFailed', error, {
          context: 'DirectMessageStore.failOptimisticMessage',
          metadata: { conversationNpub, clientMessageId },
        });
      }
      const queue = state.optimisticMessages[conversationNpub] ?? [];
      return {
        ...state,
        optimisticMessages: {
          ...state.optimisticMessages,
          [conversationNpub]: queue.map((message) =>
            message.clientMessageId === clientMessageId
              ? { ...message, status: 'failed' as const }
              : message,
          ),
        },
        isSending: state.activeConversationNpub === conversationNpub ? false : state.isSending,
      };
    }),
  setMessages: (conversationNpub, messages, options = {}) =>
    set((state) => {
      const existing = state.conversations[conversationNpub] ?? [];
      const next = options.replace ? messages : [...existing, ...messages];
      return {
        ...state,
        conversations: {
          ...state.conversations,
          [conversationNpub]: dedupeMessages(next),
        },
      };
    }),
  receiveIncomingMessage: (conversationNpub, message, options = {}) =>
    set((state) => {
      const existingMessages = state.conversations[conversationNpub] ?? [];
      const mergedMessages = dedupeMessages([...existingMessages, message]);

      const nextConversations = {
        ...state.conversations,
        [conversationNpub]: mergedMessages,
      };

      const nextOptimistic = { ...state.optimisticMessages };
      if (message.clientMessageId) {
        const queue = nextOptimistic[conversationNpub];
        if (queue) {
          const filtered = queue.filter((item) => item.clientMessageId !== message.clientMessageId);
          if (filtered.length > 0) {
            nextOptimistic[conversationNpub] = filtered;
          } else {
            delete nextOptimistic[conversationNpub];
          }
        }
      }

      const nextUnread = { ...state.unreadCounts };
      const nextReadTimestamps = { ...state.conversationReadTimestamps };
      const shouldMarkAsRead =
        state.isDialogOpen && state.activeConversationNpub === conversationNpub;

      if (shouldMarkAsRead) {
        nextUnread[conversationNpub] = 0;
        if (typeof message.createdAt === 'number') {
          const current = nextReadTimestamps[conversationNpub] ?? 0;
          if (message.createdAt > current) {
            nextReadTimestamps[conversationNpub] = message.createdAt;
          }
        }
      } else if (options.incrementUnread !== false) {
        nextUnread[conversationNpub] =
          (nextUnread[conversationNpub] ?? 0) + (options.incrementAmount ?? 1);
      }

      return {
        ...state,
        conversations: nextConversations,
        optimisticMessages: nextOptimistic,
        unreadCounts: nextUnread,
        conversationReadTimestamps: nextReadTimestamps,
      };
    }),
  removeOptimisticMessage: (conversationNpub, clientMessageId) =>
    set((state) => {
      const queue = state.optimisticMessages[conversationNpub];
      if (!queue) {
        return state;
      }
      const filtered = queue.filter((message) => message.clientMessageId !== clientMessageId);
      const nextOptimistic = { ...state.optimisticMessages };
      if (filtered.length > 0) {
        nextOptimistic[conversationNpub] = filtered;
      } else {
        delete nextOptimistic[conversationNpub];
      }
      return {
        ...state,
        optimisticMessages: nextOptimistic,
      };
    }),
  markConversationAsRead: (conversationNpub, lastReadAt) =>
    set((state) => {
      const nextState: typeof state = {
        ...state,
        unreadCounts: { ...state.unreadCounts, [conversationNpub]: 0 },
      };

      if (typeof lastReadAt === 'number') {
        const current = state.conversationReadTimestamps[conversationNpub] ?? 0;
        if (lastReadAt > current) {
          nextState.conversationReadTimestamps = {
            ...state.conversationReadTimestamps,
            [conversationNpub]: lastReadAt,
          };
        }
      }

      return nextState;
    }),
  incrementUnreadCount: (conversationNpub, amount = 1) =>
    set((state) => ({
      ...state,
      unreadCounts: {
        ...state.unreadCounts,
        [conversationNpub]: (state.unreadCounts[conversationNpub] ?? 0) + amount,
      },
    })),
  hydrateConversations: (summaries) =>
    set((state) => {
      if (!Array.isArray(summaries) || summaries.length === 0) {
        return state;
      }

      const nextConversations = { ...state.conversations };
      const nextUnreadCounts = { ...state.unreadCounts };
      const nextReadTimestamps = { ...state.conversationReadTimestamps };

      for (const summary of summaries) {
        nextUnreadCounts[summary.conversationNpub] = summary.unreadCount;
        if (summary.lastReadAt >= 0) {
          const current = nextReadTimestamps[summary.conversationNpub] ?? 0;
          if (summary.lastReadAt > current) {
            nextReadTimestamps[summary.conversationNpub] = summary.lastReadAt;
          }
        }
        if (summary.lastMessage) {
          const existing = nextConversations[summary.conversationNpub] ?? [];
          if (existing.length === 0) {
            nextConversations[summary.conversationNpub] = dedupeMessages([summary.lastMessage]);
          }
        }
      }

      return {
        ...state,
        conversations: nextConversations,
        unreadCounts: nextUnreadCounts,
        conversationReadTimestamps: nextReadTimestamps,
      };
    }),
  reset: () => set(() => ({ ...createInitialState() })),
}));
