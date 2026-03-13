import type { StateCreator } from 'zustand';
import { createJSONStorage, persist, type StateStorage } from 'zustand/middleware';

export interface PersistOptions<T, PersistedState extends Partial<T> = Partial<T>> {
  name: string;
  partialize?: (state: T) => PersistedState;
  storage?: StateStorage;
  version?: number;
}

export const createPersistConfig = <T, PersistedState extends Partial<T> = Partial<T>>({
  name,
  partialize,
  storage,
  version,
}: PersistOptions<T, PersistedState>) => {
  const resolvedStorage =
    storage != null
      ? createJSONStorage<PersistedState>(() => storage)
      : typeof window !== 'undefined'
        ? createJSONStorage<PersistedState>(() => localStorage)
        : undefined;

  return {
    name,
    storage: resolvedStorage,
    partialize,
    version,
  };
};

export const withPersist = <T, PersistedState extends Partial<T> = Partial<T>>(
  initializer: StateCreator<T, [], []>,
  options: PersistOptions<T, PersistedState>,
) => persist(initializer, createPersistConfig<T, PersistedState>(options));

export const createLocalStoragePersist = <T, PersistedState extends Partial<T> = Partial<T>>(
  name: string,
  partialize?: (state: T) => PersistedState,
) => createPersistConfig<T, PersistedState>({ name, partialize });

export const createPartializer = <T, K extends keyof T>(
  fields: K[],
): ((state: T) => Partial<T>) => {
  return (state: T) => {
    const partial: Partial<T> = {};
    fields.forEach((field) => {
      partial[field] = state[field];
    });
    return partial;
  };
};

export const serializeMap = <K, V>(map: Map<K, V>): Array<[K, V]> => {
  return Array.from(map.entries());
};

export const deserializeMap = <K, V>(entries: Array<[K, V]>): Map<K, V> => {
  return new Map(entries);
};

export const createMapAwareStorage = (): StateStorage => {
  return {
    getItem: (name) => {
      const str = localStorage.getItem(name);
      if (!str) return null;

      try {
        const { state, version } = JSON.parse(str);
        if (state) {
          Object.keys(state).forEach((key) => {
            if (
              state[key] &&
              Array.isArray(state[key]) &&
              state[key][0] &&
              Array.isArray(state[key][0])
            ) {
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
