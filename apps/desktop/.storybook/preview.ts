import { createElement } from 'react';

import type { Preview } from '@storybook/react-vite';

import { installWindowDesktopMock } from '@/mocks/installWindowDesktopMock';

import '@/styles/index.css';

installWindowDesktopMock();

const preview: Preview = {
  globalTypes: {
    shellWidth: {
      name: 'Shell Width',
      description: 'Desktop shell review width',
      defaultValue: 'desktop',
      toolbar: {
        icon: 'browser',
        items: [
          { value: 'narrow', title: 'Narrow desktop' },
          { value: 'desktop', title: 'Desktop' },
        ],
      },
    },
  },
  parameters: {
    layout: 'fullscreen',
    controls: {
      expanded: true,
    },
  },
  decorators: [
    (Story, context) => {
      const shellWidth = context.globals.shellWidth === 'narrow' ? 420 : 960;

      return createElement(
        'div',
        {
          style: {
            minHeight: '100vh',
            padding: '24px',
            background: 'var(--color-surface-muted)',
          },
        },
        createElement(
          'div',
          {
            style: {
              width: '100%',
              maxWidth: `${shellWidth}px`,
              margin: '0 auto',
            },
          },
          createElement(Story),
        ),
      );
    },
  ],
};

export default preview;
