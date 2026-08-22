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
export const InteractiveProductionShell: Story = {};
