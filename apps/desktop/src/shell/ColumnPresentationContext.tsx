import { createContext, useContext } from 'react';

// Ephemeral DOM presentation, never part of persisted workspace/session state.
export const ColumnFullscreenContext = createContext(false);
export function useColumnFullscreen() {
  return useContext(ColumnFullscreenContext);
}
