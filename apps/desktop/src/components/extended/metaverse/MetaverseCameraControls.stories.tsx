import type { Meta, StoryObj } from '@storybook/react-vite';
import { MetaverseCameraControls } from './MetaverseCameraControls';

const meta = {
  title: 'Extended/MetaverseCameraControls',
  component: MetaverseCameraControls,
  args: { locale: 'en', mode: 'idle', enabled: true, onStart: () => undefined, onCommand: () => undefined },
  decorators: [(Story) => <div style={{ position: 'relative', height: 360, width: '100%', minWidth: 280 }}><Story /></div>],
} satisfies Meta<typeof MetaverseCameraControls>;
export default meta;
type Story = StoryObj<typeof meta>;
export const PointerFree: Story = {};
export const Requesting: Story = { args: { mode: 'requesting' } };
export const Locked: Story = { args: { mode: 'locked' } };
export const Unavailable: Story = { args: { mode: 'unavailable' } };
export const Inactive: Story = { args: { enabled: false } };
export const Japanese: Story = { args: { locale: 'ja' } };
export const Chinese: Story = { args: { locale: 'zh-CN' } };
