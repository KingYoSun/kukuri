import { createJSONStorage, StateStorage } from 'zustand/middleware';

/**
 * 共通のpersist設定を生成するヘルパー関数
 */
export const createPersistConfig = <T>(
  name: string,
  partialize?: (state: T) => Partial<T>,
  storage?: StateStorage,
) => ({
  name,
  storage: storage || createJSONStorage(() => localStorage),
  partialize,
});

/**
 * localStorageを使用する標準的なpersist設定を生成
 */
export const createLocalStoragePersist = <T>(
  name: string,
  partialize?: (state: T) => Partial<T>,
) => createPersistConfig(name, partialize);

/**
 * 特定のフィールドのみを永続化するpartialize関数を生成
 */
export const createPartializer = <T, K extends keyof T>(
  fields: K[],
): ((state: T) => Pick<T, K>) => {
  return (state: T) => {
    const partial: Partial<T> = {};
    fields.forEach((field) => {
      partial[field] = state[field];
    });
    return partial as Pick<T, K>;
  };
};

/**
 * Mapオブジェクトをシリアライズ可能な形式に変換
 */
export const serializeMap = <K, V>(map: Map<K, V>): Array<[K, V]> => {
  return Array.from(map.entries());
};

/**
 * シリアライズされた配列をMapオブジェクトに復元
 */
export const deserializeMap = <K, V>(entries: Array<[K, V]>): Map<K, V> => {
  return new Map(entries);
};

/**
 * Map型を含むstateをシリアライズ/デシリアライズするstorage
 */
export const createMapAwareStorage = (): StateStorage => {
  return {
    getItem: (name) => {
      const str = localStorage.getItem(name);
      if (!str) return null;
      
      try {
        const { state, version } = JSON.parse(str);
        // Mapフィールドを復元
        if (state) {
          Object.keys(state).forEach((key) => {
            if (state[key] && Array.isArray(state[key]) && state[key][0] && Array.isArray(state[key][0])) {
              // [[key, value], ...] の形式ならMapとして復元
              state[key] = deserializeMap(state[key]);
            }
          });
        }
        return JSON.stringify({ state, version });
      } catch {
        return str;
      }
    },
    setItem: (name, value) => {
      try {
        const { state, version } = JSON.parse(value);
        // Mapフィールドをシリアライズ
        if (state) {
          Object.keys(state).forEach((key) => {
            if (state[key] instanceof Map) {
              state[key] = serializeMap(state[key]);
            }
          });
        }
        localStorage.setItem(name, JSON.stringify({ state, version }));
      } catch {
        localStorage.setItem(name, value);
      }
    },
    removeItem: (name) => {
      localStorage.removeItem(name);
    },
  };
};