import type { Meta, StoryObj } from '@storybook/react-vite';

import { ProductionColumnWorkspaceStory } from './ProductionColumnWorkspaceStory';

const meta = {
  title: 'Review/ProductionColumnWorkspace',
  component: ProductionColumnWorkspaceStory,
  parameters: {
    layout: 'fullscreen',
    reviewCanvas: true,
  },
} satisfies Meta<typeof ProductionColumnWorkspaceStory>;

export default meta;

type Story = StoryObj<typeof meta>;

export const ScopedDraftsAndComposer: Story = {};
export const ControlCenterOpen: Story = {
  args: {
    initialControlCenterOpen: true,
  },
};
export const InteractiveProductionShell: Story = {};
export const VariableSpanWideSurfaces: Story = {
  globals: { shellWidth: 'ultrawide1920' },
  args: {
    scenario: 'wide-surfaces',
    metaverseSpan: 3,
  },
};
export const MetaverseFourSpan: Story = {
  globals: { shellWidth: 'ultrawide1920' },
  args: {
    scenario: 'wide-surfaces',
    metaverseSpan: 4,
  },
};
