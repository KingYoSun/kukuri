import type { AsyncPanelState } from '@/shell/store';

// Record 状態(`Record<string, V>`)の 1 キー更新・削除の定型(WP-H6 PR1)。
// shell の状態更新はこの形が大半で、各所で spread / delete を手書きしていた。
// setter にそのまま渡せる updater を返す。

/** 1 キーを値で差し替える updater。常に新しいオブジェクトを返す(従来の spread と同じ)。 */
export function setRecordEntry<V>(
  key: string,
  value: V
): (current: Record<string, V>) => Record<string, V> {
  return (current) => ({ ...current, [key]: value });
}

/** 1 キーを「現在値からの導出」で差し替える updater。 */
export function updateRecordEntry<V>(
  key: string,
  update: (prev: V | undefined) => V
): (current: Record<string, V>) => Record<string, V> {
  return (current) => ({ ...current, [key]: update(current[key]) });
}

/**
 * 1 キーを削除する updater。キーが無ければ **current をそのまま返す**
 * (参照同一性を保ち、不要な再レンダーを起こさない。従来の手書き delete と同じ)。
 */
export function removeRecordEntry<V>(
  key: string
): (current: Record<string, V>) => Record<string, V> {
  return (current) => {
    if (!(key in current)) {
      return current;
    }
    const next = { ...current };
    delete next[key];
    return next;
  };
}

// AsyncPanelState({ status, error })の定型コンストラクタ。
// 「loading にするとき error を消し忘れる」类のずれを防ぐ。

export function panelLoading(): AsyncPanelState {
  return { status: 'loading', error: null };
}

export function panelReady(): AsyncPanelState {
  return { status: 'ready', error: null };
}

export function panelError(error: string): AsyncPanelState {
  return { status: 'error', error };
}
