import { create } from 'zustand';

import { withPersist } from './utils/persistHelpers';
import { createCommunityNodePersistConfig } from './config/persist';

interface CommunityNodeState {
  enableAccessControl: boolean;
  setEnableAccessControl: (value: boolean) => void;
  reset: () => void;
}

const initialState = {
  enableAccessControl: true,
};

export const useCommunityNodeStore = create<CommunityNodeState>()(
  withPersist<CommunityNodeState>(
    (set) => ({
      ...initialState,
      setEnableAccessControl: (value) => set({ enableAccessControl: value }),
      reset: () => set(initialState),
    }),
    createCommunityNodePersistConfig<CommunityNodeState>(),
  ),
);
