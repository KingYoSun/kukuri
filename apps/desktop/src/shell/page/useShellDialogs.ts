import { startTransition, useCallback, useEffect, useRef, useState } from 'react';

import type { PrimarySection, TimelineWorkspaceView } from '@/components/shell/types';
import type { DesktopApi } from '@/lib/api';
import { mergeKnownAuthors } from '@/shell/selectors';
import { useDesktopShellFieldSetter } from '@/shell/store';

type PendingLeaveChannel = {
  channelId: string;
  topicId: string;
};

type UseShellDialogsArgs = {
  activePrimarySection: PrimarySection;
  api: DesktopApi;
  timelineView: TimelineWorkspaceView;
};

export function useShellDialogs({
  activePrimarySection,
  api,
  timelineView,
}: UseShellDialogsArgs) {
  const [composeDialogOpen, setComposeDialogOpen] = useState(false);
  const [channelDialogOpen, setChannelDialogOpen] = useState(false);
  const [channelSettingsDialogOpen, setChannelSettingsDialogOpen] = useState(false);
  const [leaveChannelDialogOpen, setLeaveChannelDialogOpenState] = useState(false);
  const [pendingLeaveChannel, setPendingLeaveChannel] = useState<PendingLeaveChannel | null>(null);
  const [liveCreateDialogOpen, setLiveCreateDialogOpen] = useState(false);
  const [gameCreateDialogOpen, setGameCreateDialogOpen] = useState(false);
  const [profileAvatarCropFile, setProfileAvatarCropFile] = useState<File | null>(null);
  const [profileAvatarCropOpen, setProfileAvatarCropOpen] = useState(false);
  const [profileAvatarInputKey, setProfileAvatarInputKey] = useState(0);
  const previousPrimarySectionRef = useRef(activePrimarySection);
  const previousTimelineViewRef = useRef(timelineView);
  const setSocialConnections = useDesktopShellFieldSetter('socialConnections');
  const setKnownAuthorsByPubkey = useDesktopShellFieldSetter('knownAuthorsByPubkey');

  useEffect(() => {
    const previousPrimarySection = previousPrimarySectionRef.current;
    const previousTimelineView = previousTimelineViewRef.current;
    const enteredBookmarkTimeline =
      previousPrimarySection === 'timeline' &&
      activePrimarySection === 'timeline' &&
      previousTimelineView !== 'bookmarks' &&
      timelineView === 'bookmarks';

    if ((activePrimarySection !== 'timeline' || enteredBookmarkTimeline) && composeDialogOpen) {
      setComposeDialogOpen(false);
    }
    if (activePrimarySection !== 'live' && liveCreateDialogOpen) {
      setLiveCreateDialogOpen(false);
    }
    if (activePrimarySection !== 'game' && gameCreateDialogOpen) {
      setGameCreateDialogOpen(false);
    }
    previousPrimarySectionRef.current = activePrimarySection;
    previousTimelineViewRef.current = timelineView;
  }, [
    activePrimarySection,
    composeDialogOpen,
    gameCreateDialogOpen,
    liveCreateDialogOpen,
    timelineView,
  ]);

  useEffect(() => {
    if (!composeDialogOpen) {
      return;
    }
    let disposed = false;
    void (async () => {
      try {
        const following = await api.listSocialConnections('following');
        if (disposed) {
          return;
        }
        startTransition(() => {
          setSocialConnections((current) => ({ ...current, following }));
          setKnownAuthorsByPubkey((current) => mergeKnownAuthors(current, following));
        });
      } catch {
        // best effort: fall back to already-known authors for suggestions
      }
    })();
    return () => {
      disposed = true;
    };
  }, [api, composeDialogOpen, setKnownAuthorsByPubkey, setSocialConnections]);

  const openLeaveChannelDialog = useCallback((topicId: string, channelId: string) => {
    setPendingLeaveChannel({ topicId, channelId });
    setLeaveChannelDialogOpenState(true);
  }, []);

  const setLeaveChannelDialogOpen = useCallback((open: boolean) => {
    setLeaveChannelDialogOpenState(open);
    if (!open) {
      setPendingLeaveChannel(null);
    }
  }, []);

  const confirmLeaveChannel = useCallback(
    async (leaveChannel: (topicId: string, channelId: string) => Promise<void>) => {
      if (!pendingLeaveChannel) {
        return;
      }
      await leaveChannel(pendingLeaveChannel.topicId, pendingLeaveChannel.channelId);
      setLeaveChannelDialogOpenState(false);
      setPendingLeaveChannel(null);
    },
    [pendingLeaveChannel]
  );

  return {
    channelDialogOpen,
    channelSettingsDialogOpen,
    composeDialogOpen,
    confirmLeaveChannel,
    gameCreateDialogOpen,
    leaveChannelDialogOpen,
    liveCreateDialogOpen,
    openLeaveChannelDialog,
    profileAvatarCropFile,
    profileAvatarCropOpen,
    profileAvatarInputKey,
    setChannelDialogOpen,
    setChannelSettingsDialogOpen,
    setComposeDialogOpen,
    setGameCreateDialogOpen,
    setLeaveChannelDialogOpen,
    setLiveCreateDialogOpen,
    setProfileAvatarCropFile,
    setProfileAvatarCropOpen,
    setProfileAvatarInputKey,
  };
}
