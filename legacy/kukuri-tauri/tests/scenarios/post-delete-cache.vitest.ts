import baseConfig from '../../vitest.config';
import { defineConfig, mergeConfig } from 'vitest/config';

const reportPath =
  process.env.POST_DELETE_CACHE_REPORT ?? 'test-results/post-delete-cache/report.json';

export default mergeConfig(
  baseConfig,
  defineConfig({
    test: {
      include: [
        'src/tests/unit/hooks/useDeletePost.test.tsx',
        'src/tests/unit/components/posts/PostCard.test.tsx',
        'src/tests/unit/components/posts/PostCard.deleteOffline.test.tsx',
      ],
      reporters: [
        'default',
        ['json', { outputFile: reportPath }],
      ],
    },
  }),
);
