import type { ChannelAccessTokenKind } from '@/lib/api';
import { parseHashRouteLocation } from '@/shell/routes';

export type TopicLinkReference = {
  kind: 'topic';
  topic: string;
  route: string;
};

export type PostLinkReference = {
  kind: 'post';
  topic: string;
  threadId: string;
  focusObjectId: string | null;
  route: string;
};

export type LiveLinkReference = {
  kind: 'live';
  topic: string;
  channelId: string | null;
  sessionId: string;
  route: string;
};

export type GameLinkReference = {
  kind: 'game';
  topic: string;
  channelId: string | null;
  roomId: string;
  route: string;
};

export type ShareTokenReference = {
  kind: 'share_token';
  token: string;
  tokenKind: ChannelAccessTokenKind;
};

export type InternalSmartReference =
  | TopicLinkReference
  | PostLinkReference
  | LiveLinkReference
  | GameLinkReference
  | ShareTokenReference;

export type SmartTextSegment =
  | {
      kind: 'text';
      text: string;
    }
  | {
      kind: 'reference';
      reference: InternalSmartReference;
    };

const TOPIC_PATTERN = /kukuri:topic:[A-Za-z0-9:_-]+/;
const ROUTE_PATTERN = /#\/(?:timeline|live|game)\?[^\s]+/;

function buildRoute(pathname: string, params: URLSearchParams): string {
  const search = params.toString();
  return search ? `#${pathname}?${search}` : `#${pathname}`;
}

export function buildTopicLink(topic: string): string {
  const params = new URLSearchParams();
  params.set('topic', topic);
  return buildRoute('/timeline', params);
}

export function buildPostLink(
  topic: string,
  threadId: string,
  focusObjectId: string | null = null
): string {
  const params = new URLSearchParams();
  params.set('topic', topic);
  params.set('context', 'thread');
  params.set('threadId', threadId);
  if (focusObjectId) {
    params.set('focusObjectId', focusObjectId);
  }
  return buildRoute('/timeline', params);
}

export function buildLiveLink(
  topic: string,
  sessionId: string,
  channelId: string | null = null
): string {
  const params = new URLSearchParams();
  params.set('topic', topic);
  if (channelId) {
    params.set('channel', channelId);
  }
  params.set('sessionId', sessionId);
  return buildRoute('/live', params);
}

export function buildGameLink(
  topic: string,
  roomId: string,
  channelId: string | null = null
): string {
  const params = new URLSearchParams();
  params.set('topic', topic);
  if (channelId) {
    params.set('channel', channelId);
  }
  params.set('roomId', roomId);
  return buildRoute('/game', params);
}

export function shortenReferenceId(value: string): string {
  const trimmed = value.trim();
  if (trimmed.length <= 18) {
    return trimmed;
  }
  return `${trimmed.slice(0, 10)}…`;
}

export function parseShareTokenKind(rawValue: string): ChannelAccessTokenKind | null {
  const trimmed = rawValue.trim();
  if (!trimmed) {
    return null;
  }
  if (trimmed.startsWith('invite:')) {
    return 'invite';
  }
  if (trimmed.startsWith('grant:')) {
    return 'grant';
  }
  if (trimmed.startsWith('share:')) {
    return 'share';
  }
  if (!trimmed.startsWith('{')) {
    return null;
  }
  try {
    const parsed = JSON.parse(trimmed) as {
      envelope?: {
        kind?: string;
      };
    };
    if (parsed.envelope?.kind === 'channel-invite') {
      return 'invite';
    }
    if (parsed.envelope?.kind === 'channel-friend-grant') {
      return 'grant';
    }
    if (parsed.envelope?.kind === 'channel-share') {
      return 'share';
    }
  } catch {
    return null;
  }
  return null;
}

export function parseInternalRouteLink(rawValue: string): InternalSmartReference | null {
  const { pathname, search } = parseHashRouteLocation(rawValue);
  const params = new URLSearchParams(search);
  const topic = params.get('topic')?.trim() ?? '';
  if (!topic) {
    return null;
  }
  if (pathname === '/timeline') {
    const context = params.get('context');
    const threadId = params.get('threadId')?.trim() ?? '';
    if (context === 'thread' && threadId) {
      const focusObjectId = params.get('focusObjectId')?.trim() || null;
      return {
        kind: 'post',
        topic,
        threadId,
        focusObjectId,
        route: rawValue,
      };
    }
    return {
      kind: 'topic',
      topic,
      route: rawValue,
    };
  }
  if (pathname === '/live') {
    const sessionId = params.get('sessionId')?.trim() ?? '';
    if (!sessionId) {
      return null;
    }
    return {
      kind: 'live',
      topic,
      channelId: params.get('channel')?.trim() || null,
      sessionId,
      route: rawValue,
    };
  }
  if (pathname === '/game') {
    const roomId = params.get('roomId')?.trim() ?? '';
    if (!roomId) {
      return null;
    }
    return {
      kind: 'game',
      topic,
      channelId: params.get('channel')?.trim() || null,
      roomId,
      route: rawValue,
    };
  }
  return null;
}

function parseTopicReference(rawTopic: string): TopicLinkReference {
  return {
    kind: 'topic',
    topic: rawTopic,
    route: buildTopicLink(rawTopic),
  };
}

function findNextReference(
  value: string,
  offset: number
): { index: number; length: number; reference: InternalSmartReference } | null {
  const remaining = value.slice(offset);
  const routeMatch = ROUTE_PATTERN.exec(remaining);
  const topicMatch = TOPIC_PATTERN.exec(remaining);
  const candidates = [
    routeMatch
      ? {
          index: offset + routeMatch.index,
          text: routeMatch[0],
          reference: parseInternalRouteLink(routeMatch[0]),
        }
      : null,
    topicMatch
      ? {
          index: offset + topicMatch.index,
          text: topicMatch[0],
          reference: parseTopicReference(topicMatch[0]),
        }
      : null,
  ].filter(
    (
      candidate
    ): candidate is {
      index: number;
      text: string;
      reference: InternalSmartReference | null;
    } => candidate !== null
  );

  if (candidates.length === 0) {
    return null;
  }

  candidates.sort((left, right) => left.index - right.index);
  const next = candidates[0];
  if (!next.reference) {
    return null;
  }
  return {
    index: next.index,
    length: next.text.length,
    reference: next.reference,
  };
}

export function parseSmartText(value: string): SmartTextSegment[][] {
  return value.split('\n').map((line) => {
    const trimmed = line.trim();
    const shareTokenKind = parseShareTokenKind(trimmed);
    if (shareTokenKind) {
      return [
        {
          kind: 'reference',
          reference: {
            kind: 'share_token',
            token: trimmed,
            tokenKind: shareTokenKind,
          },
        },
      ] satisfies SmartTextSegment[];
    }

    const segments: SmartTextSegment[] = [];
    let offset = 0;

    while (offset < line.length) {
      const next = findNextReference(line, offset);
      if (!next) {
        segments.push({
          kind: 'text',
          text: line.slice(offset),
        });
        break;
      }
      if (next.index > offset) {
        segments.push({
          kind: 'text',
          text: line.slice(offset, next.index),
        });
      }
      segments.push({
        kind: 'reference',
        reference: next.reference,
      });
      offset = next.index + next.length;
    }

    if (segments.length === 0) {
      segments.push({
        kind: 'text',
        text: '',
      });
    }
    return segments;
  });
}
