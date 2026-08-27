import type { Meta, StoryObj } from '@storybook/react-vite';

import { createDefaultDomeCustomization } from './DomeSceneModel';
import { DomeCustomizationControls } from './DomeCustomizationControls';

const meta = {
  title: 'Extended/Metaverse/DomeCustomizationControls',
  component: DomeCustomizationControls,
  parameters: { layout: 'centered' },
  args: {
    customization: createDefaultDomeCustomization(),
    pending: false,
    locale: 'en',
    onSave: async () => undefined,
    onImportTexture: async (file: File) => ({
      kind: 'texture',
      blob_hash: `story-${file.name}`,
      mime_type: file.type || 'image/png',
      size_bytes: file.size,
      name: file.name,
    }),
  },
} satisfies Meta<typeof DomeCustomizationControls>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Owner: Story = { args: { isOwner: true } };
export const ReadOnlyGuest: Story = { args: { isOwner: false } };
export const Pending: Story = { args: { isOwner: true, pending: true } };
