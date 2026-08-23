import { createContext, useContext, type ReactNode } from 'react';

import type { ColumnRuntimeState } from '@/shell/columnRuntime';

type ColumnRuntimeContextValue = ColumnRuntimeState & {
  requestAudioFocus: () => void;
  releaseAudioFocus: () => void;
};

const DEFAULT_RUNTIME: ColumnRuntimeContextValue = {
  visible: true,
  active: true,
  audioFocused: false,
  suspended: false,
  requestAudioFocus: () => undefined,
  releaseAudioFocus: () => undefined,
};

const ColumnRuntimeContext = createContext<ColumnRuntimeContextValue>(DEFAULT_RUNTIME);

export function ColumnRuntimeProvider({
  children,
  value,
}: {
  children: ReactNode;
  value: ColumnRuntimeContextValue;
}) {
  return (
    <ColumnRuntimeContext.Provider value={value}>
      {children}
    </ColumnRuntimeContext.Provider>
  );
}

// eslint-disable-next-line react-refresh/only-export-components
export function useColumnRuntime() {
  return useContext(ColumnRuntimeContext);
}
