import { useCallback, useEffect, useRef, useState, type RefObject } from 'react';

export type SceneInputMode = 'idle' | 'requesting' | 'locked' | 'unavailable';

// Pointer Lock is document-wide; each mounted room may only release its own canvas.
export function useMetaverseSceneInput(stageRef: RefObject<HTMLDivElement | null>, eligible: boolean, sessionKey: string) {
  const [mode, setMode] = useState<SceneInputMode>('idle');
  const owner = useRef<HTMLCanvasElement | null>(null);
  const wanted = useRef(false);
  const generation = useRef(0);
  const permitted = useRef(eligible);

  const release = useCallback(() => {
    wanted.current = false;
    generation.current += 1;
    if (owner.current && document.pointerLockElement === owner.current) document.exitPointerLock?.();
    setMode('idle');
  }, []);

  useEffect(() => {
    permitted.current = eligible;
    if (!eligible) release();
  }, [eligible, release]);

  useEffect(() => {
    const canvas = stageRef.current?.querySelector('canvas') ?? null;
    owner.current = canvas;
    const changed = () => {
      if (canvas && document.pointerLockElement === canvas) {
        if (!wanted.current || !permitted.current || document.hidden || !document.hasFocus()) {
          document.exitPointerLock?.();
          return;
        }
        setMode('locked');
      } else {
        wanted.current = false;
        setMode((current) => current === 'unavailable' ? current : 'idle');
      }
    };
    const failed = () => {
      if (!wanted.current) return;
      wanted.current = false;
      setMode('unavailable');
    };
    const hidden = () => { if (document.hidden) release(); };
    const focusChanged = (event: FocusEvent) => {
      if (event.target !== stageRef.current && event.target !== canvas) release();
    };
    document.addEventListener('pointerlockchange', changed);
    document.addEventListener('pointerlockerror', failed);
    document.addEventListener('visibilitychange', hidden);
    document.addEventListener('focusin', focusChanged);
    window.addEventListener('blur', release);
    return () => {
      release();
      document.removeEventListener('pointerlockchange', changed);
      document.removeEventListener('pointerlockerror', failed);
      document.removeEventListener('visibilitychange', hidden);
      document.removeEventListener('focusin', focusChanged);
      window.removeEventListener('blur', release);
      owner.current = null;
    };
  }, [release, sessionKey, stageRef]);

  const start = useCallback(() => {
    if (!eligible || document.hidden || !document.hasFocus() || document.pointerLockElement) return;
    const canvas = stageRef.current?.querySelector('canvas') ?? null;
    owner.current = canvas;
    stageRef.current?.focus({ preventScroll: true });
    const attempt = ++generation.current;
    if (!canvas?.requestPointerLock) { setMode('unavailable'); return; }
    wanted.current = true;
    setMode('requesting');
    const failed = () => {
      if (attempt === generation.current && wanted.current) {
        wanted.current = false;
        setMode('unavailable');
      }
    };
    try {
      // Both Promise-returning and older void-returning WebViews are supported.
      void Promise.resolve(canvas.requestPointerLock()).then(() => {
        if (attempt !== generation.current && document.pointerLockElement === canvas) document.exitPointerLock?.();
      }, failed);
    } catch { failed(); }
  }, [eligible, stageRef]);

  return { mode, start, release };
}
