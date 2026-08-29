import type { Dispatch, SetStateAction } from 'react';

import type { DesktopApi } from '@/lib/api';
import type { OpenAuthorOptions, OpenThreadOptions } from '@/shell/routes';
import type { DesktopShellState, DesktopShellStateValue } from '@/shell/store';

export type Setter<K extends keyof DesktopShellState> = (
  value: DesktopShellStateValue<K>
) => void;

export type DerivedSetter<T> = (value: T | ((current: T) => T)) => void;

export type SyncRoute = (
  mode?: 'push' | 'replace',
  overrides?: Record<string, unknown>
) => void;

export type OpenDirectMessagePane = (
  peerPubkey: string,
  options?: {
    historyMode?: 'push' | 'replace';
    normalizeOnError?: boolean;
    parentColumnId?: string;
    preserveAuthorPane?: boolean;
    preservedAuthorPubkey?: string | null;
  }
) => Promise<void>;

export type OpenAuthorDetail = (
  authorPubkey: string,
  options?: OpenAuthorOptions
) => Promise<void>;

export type OpenThread = (
  threadId: string,
  options?: OpenThreadOptions
) => Promise<void>;

export type Translate = (key: string, options?: Record<string, unknown>) => string;

export type LoadTopics = (
  topics: string[],
  activeTopic: string,
  currentThread: string | null
) => Promise<void>;

export type BoolStateDispatch = Dispatch<SetStateAction<boolean>>;
export type NullableStringDispatch = Dispatch<SetStateAction<string | null>>;
export type NumberStateDispatch = Dispatch<SetStateAction<number>>;

export type ActionsBaseParams = {
  api: DesktopApi;
  translate: Translate;
  loadTopics: LoadTopics;
  syncRoute: SyncRoute;
};

export type NavigationActions = {
  openDirectMessagePane: OpenDirectMessagePane;
  openAuthorDetail: OpenAuthorDetail;
  openThread: OpenThread;
};
