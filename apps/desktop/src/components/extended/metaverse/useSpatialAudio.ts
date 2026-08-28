import { useCallback, useEffect, useRef, useState, type RefObject } from 'react';

import type { GameRoomView, MetaverseRoomEventView } from '@/lib/api';
import type { AvatarTransform } from '../MetaverseSceneModel';
import type { DomeNeighborTransitionView } from './DomeTransitionModel';
import type { MetaverseRoomActions } from './MetaverseRoomActions';
import {
  connectionOpeningAudioDistanceCm,
  distanceCm,
  METAVERSE_AUDIO_SAMPLE_RATE_HZ,
  resamplePcm16,
  spatialAudioGain,
} from './SpatialAudioModel';

type UseSpatialAudioArgs = {
  actions: MetaverseRoomActions;
  selectedRoom: GameRoomView | null;
  localPeerId: string;
  listenerTransformRef: RefObject<AvatarTransform | null>;
  mutedAuthorPubkeys: ReadonlySet<string>;
  transitionNeighbors: DomeNeighborTransitionView[];
};

export function useSpatialAudio({
  actions,
  selectedRoom,
  localPeerId,
  listenerTransformRef,
  mutedAuthorPubkeys,
  transitionNeighbors,
}: UseSpatialAudioArgs) {
  const [microphoneEnabled, setMicrophoneEnabled] = useState(false);
  const audioContextRef = useRef<AudioContext | null>(null);
  const microphoneStreamRef = useRef<MediaStream | null>(null);
  const microphoneSourceRef = useRef<MediaStreamAudioSourceNode | null>(null);
  const microphoneProcessorRef = useRef<ScriptProcessorNode | null>(null);
  const playingSourcesRef = useRef(new Set<AudioBufferSourceNode>());

  const stopSpatialAudio = useCallback(() => {
    for (const source of playingSourcesRef.current) source.stop();
    playingSourcesRef.current.clear();
  }, []);

  const disableMicrophone = useCallback(() => {
    microphoneProcessorRef.current?.disconnect();
    microphoneSourceRef.current?.disconnect();
    microphoneStreamRef.current?.getTracks().forEach((track) => track.stop());
    microphoneProcessorRef.current = null;
    microphoneSourceRef.current = null;
    microphoneStreamRef.current = null;
    setMicrophoneEnabled(false);
  }, []);

  const playSpatialAudioFrame = useCallback((view: MetaverseRoomEventView) => {
    if (view.content.event.type !== 'spatial_audio_frame'
      || mutedAuthorPubkeys.has(String(view.envelope.pubkey))
      || view.content.peer_id === localPeerId) return;
    const context = audioContextRef.current;
    const listenerPosition = listenerTransformRef.current?.position;
    if (!context || !listenerPosition || context.state !== 'running') return;
    const frame = view.content.event.frame;
    let distance = distanceCm(frame.position, listenerPosition);
    if (selectedRoom && frame.room_id !== selectedRoom.room_id) {
      const neighbor = transitionNeighbors.find((candidate) =>
        candidate.room.room_id === frame.room_id && candidate.boundaryState === 'ready'
      );
      if (!neighbor) return;
      distance = connectionOpeningAudioDistanceCm(
        frame.position,
        neighbor.targetDirection,
        neighbor.direction,
        listenerPosition
      );
    }
    const buffer = context.createBuffer(1, frame.samples.length, METAVERSE_AUDIO_SAMPLE_RATE_HZ);
    const channel = buffer.getChannelData(0);
    frame.samples.forEach((sample, index) => { channel[index] = sample / 32_768; });
    const source = context.createBufferSource();
    const gain = context.createGain();
    gain.gain.value = spatialAudioGain(distance);
    source.buffer = buffer;
    source.connect(gain).connect(context.destination);
    playingSourcesRef.current.add(source);
    source.onended = () => playingSourcesRef.current.delete(source);
    source.start();
  }, [listenerTransformRef, localPeerId, mutedAuthorPubkeys, selectedRoom, transitionNeighbors]);

  const enableMicrophone = useCallback(async () => {
    if (!selectedRoom || microphoneStreamRef.current) return;
    const stream = await navigator.mediaDevices.getUserMedia({
      audio: { channelCount: 1 },
      video: false,
    });
    const context = audioContextRef.current ?? new AudioContext();
    audioContextRef.current = context;
    await context.resume();
    const source = context.createMediaStreamSource(stream);
    const processor = context.createScriptProcessor(1024, 1, 1);
    source.connect(processor);
    processor.connect(context.destination);
    processor.onaudioprocess = (event) => {
      const samples = resamplePcm16(event.inputBuffer.getChannelData(0), context.sampleRate);
      if (!samples.length || !selectedRoom || !microphoneStreamRef.current) return;
      const now = Date.now();
      void actions.publishRoomEvent(selectedRoom.room_id, localPeerId, now, {
        type: 'spatial_audio_frame',
        frame: {
          room_id: selectedRoom.room_id,
          peer_id: localPeerId,
          position: listenerTransformRef.current?.position ?? [0, 100, 0],
          sample_rate_hz: METAVERSE_AUDIO_SAMPLE_RATE_HZ,
          samples,
          captured_at: now,
        },
      }).catch(() => disableMicrophone());
    };
    microphoneStreamRef.current = stream;
    microphoneSourceRef.current = source;
    microphoneProcessorRef.current = processor;
    setMicrophoneEnabled(true);
  }, [actions, disableMicrophone, listenerTransformRef, localPeerId, selectedRoom]);

  const toggleMicrophone = useCallback(() => {
    if (microphoneEnabled) disableMicrophone();
    else void enableMicrophone().catch(() => disableMicrophone());
  }, [disableMicrophone, enableMicrophone, microphoneEnabled]);

  useEffect(() => () => {
    disableMicrophone();
    stopSpatialAudio();
    void audioContextRef.current?.close();
    audioContextRef.current = null;
  }, [disableMicrophone, stopSpatialAudio]);

  return {
    microphoneEnabled,
    toggleMicrophone,
    disableMicrophone,
    playSpatialAudioFrame,
    stopSpatialAudio,
  };
}
