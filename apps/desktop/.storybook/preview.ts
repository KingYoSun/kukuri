import { createElement } from 'react';
import { MemoryRouter } from 'react-router-dom';

import type { Preview } from '@storybook/react-vite';

import i18n, { type SupportedLocale } from '@/i18n';
import { installWindowDesktopMock } from '@/mocks/installWindowDesktopMock';

import '@/styles/index.css';

installWindowDesktopMock();

const preview: Preview = {
  globalTypes: {
    theme: {
      name: 'Theme',
      description: 'Desktop shell theme',
      defaultValue: 'dark',
      toolbar: {
        icon: 'mirror',
        items: [
          { value: 'dark', title: 'Dark' },
          { value: 'light', title: 'Light' },
        ],
      },
    },
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
    locale: {
      name: 'Locale',
      description: 'Desktop shell locale',
      defaultValue: 'en',
      toolbar: {
        icon: 'globe',
        items: [
          { value: 'en', title: 'English' },
          { value: 'ja', title: '日本語' },
          { value: 'zh-CN', title: '简体中文' }
        ]
      }
    }
  },
  parameters: {
    layout: 'fullscreen',
    controls: {
      expanded: true,
    },
  },
  decorators: [
    (Story, context) => {
      const theme = context.globals.theme === 'light' ? 'light' : 'dark';
      const shellWidth = context.globals.shellWidth === 'narrow' ? 420 : 960;
      const locale = (context.globals.locale ?? 'en') as SupportedLocale;
      document.documentElement.dataset.theme = theme;
      document.documentElement.lang = locale;
      void i18n.changeLanguage(locale);

      return createElement(
        'div',
        {
          style: {
            minHeight: '100vh',
            padding: '24px',
            background: 'var(--shell-background)',
          },
        },
        createElement(
          MemoryRouter,
          null,
          createElement(
            'div',
            {
              className: 'shell-phase1',
              style: {
                width: '100%',
                maxWidth: `${shellWidth}px`,
                margin: '0 auto',
                padding: 0,
                gap: 0,
              },
            },
            createElement(Story),
          ),
        ),
      );
    },
  ],
};

export default preview;
