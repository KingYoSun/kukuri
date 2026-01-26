import { create } from 'zustand';

import { withPersist } from './utils/persistHelpers';
import { createCommunityNodePersistConfig } from './config/persist';

interface CommunityNodeState {
  enableAccessControl: boolean;
  enableLabels: boolean;
  enableTrust: boolean;
  enableSearch: boolean;
  setEnableAccessControl: (value: boolean) => void;
  setEnableLabels: (value: boolean) => void;
  setEnableTrust: (value: boolean) => void;
  setEnableSearch: (value: boolean) => void;
  reset: () => void;
}

const initialState = {
  enableAccessControl: true,
  enableLabels: true,
  enableTrust: true,
  enableSearch: false,
};

export const useCommunityNodeStore = create<CommunityNodeState>()(
  withPersist<CommunityNodeState>(
    (set) => ({
      ...initialState,
      setEnableAccessControl: (value) => set({ enableAccessControl: value }),
      setEnableLabels: (value) => set({ enableLabels: value }),
      setEnableTrust: (value) => set({ enableTrust: value }),
      setEnableSearch: (value) => set({ enableSearch: value }),
      reset: () => set(initialState),
    }),
    createCommunityNodePersistConfig<CommunityNodeState>(),
  ),
);
