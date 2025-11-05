import { create } from 'zustand';

import { errorHandler } from '@/lib/errorHandler';

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

interface SetMessagesOptions {
  replace?: boolean;
}

interface DirectMessageStoreState {
  isDialogOpen: boolean;
  activeConversationNpub: string | null;
  messageDraft: string;
  isSending: boolean;
  conversations: Record<string, DirectMessageModel[]>;
  optimisticMessages: Record<string, DirectMessageModel[]>;
  unreadCounts: Record<string, number>;
  openDialog: (conversationNpub: string) => void;
  closeDialog: () => void;
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
  markConversationAsRead: (conversationNpub: string) => void;
  incrementUnreadCount: (conversationNpub: string, amount?: number) => void;
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
  | 'setDraft'
  | 'setIsSending'
  | 'appendOptimisticMessage'
  | 'resolveOptimisticMessage'
  | 'failOptimisticMessage'
  | 'setMessages'
  | 'markConversationAsRead'
  | 'incrementUnreadCount'
  | 'reset'
> => ({
  ...createDialogState(),
  conversations: {},
  optimisticMessages: {},
  unreadCounts: {},
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
      activeConversationNpub: conversationNpub,
      unreadCounts: {
        ...state.unreadCounts,
        [conversationNpub]: 0,
      },
    })),
  closeDialog: () => set((state) => ({ ...state, ...createDialogState() })),
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
  markConversationAsRead: (conversationNpub) =>
    set((state) => ({
      ...state,
      unreadCounts: { ...state.unreadCounts, [conversationNpub]: 0 },
    })),
  incrementUnreadCount: (conversationNpub, amount = 1) =>
    set((state) => ({
      ...state,
      unreadCounts: {
        ...state.unreadCounts,
        [conversationNpub]: (state.unreadCounts[conversationNpub] ?? 0) + amount,
      },
    })),
  reset: () => set(() => ({ ...createInitialState() })),
}));
