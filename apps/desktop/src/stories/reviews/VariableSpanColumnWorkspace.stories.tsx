import type { Meta, StoryObj } from '@storybook/react-vite';

import { VariableSpanColumnWorkspacePrototype } from './VariableSpanColumnWorkspacePrototype';

const meta = {
  title: 'Review/VariableSpanColumnWorkspace',
  component: VariableSpanColumnWorkspacePrototype,
  parameters: {
    layout: 'fullscreen',
    reviewCanvas: true,
  },
  args: {
    scenario: 'single',
    initialControlCenterOpen: false,
    reducedMotion: false,
  },
  argTypes: {
    scenario: {
      control: 'select',
      options: [
        'single',
        'multi',
        'thread-chain',
        'stream',
        'metaverse-3',
        'metaverse-4',
        'mobile',
        'states',
      ],
    },
  },
} satisfies Meta<typeof VariableSpanColumnWorkspacePrototype>;

export default meta;

type Story = StoryObj<typeof meta>;

export const SingleTimeline: Story = {};

export const TimelineThreadProfile: Story = {
  args: { scenario: 'multi' },
};

export const ThreadChain: Story = {
  args: { scenario: 'thread-chain' },
};

export const StreamTwoSpan: Story = {
  args: { scenario: 'stream' },
};

export const MetaverseThreeSpan: Story = {
  args: { scenario: 'metaverse-3' },
};

export const MetaverseFourSpan: Story = {
  args: { scenario: 'metaverse-4' },
};

export const MobileOneViewport: Story = {
  args: { scenario: 'mobile' },
};

export const ControlCenterOpen: Story = {
  args: { scenario: 'multi', initialControlCenterOpen: true },
};

export const ColumnStates: Story = {
  args: { scenario: 'states' },
};

export const ReducedMotion: Story = {
  args: { scenario: 'multi', initialControlCenterOpen: true, reducedMotion: true },
  globals: { motion: 'reduce' },
};
