import { fixupConfigRules } from '@eslint/compat'
import reactRefresh from 'eslint-plugin-react-refresh'
import globals from 'globals'
import tsParser from '@typescript-eslint/parser'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import js from '@eslint/js'
import { FlatCompat } from '@eslint/eslintrc'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const compat = new FlatCompat({
  baseDirectory: __dirname,
  recommendedConfig: js.configs.recommended,
  allConfig: js.configs.all
})

export default [
  {
    ignores: [
      '**/dist',
      '**/node_modules',
      '**/dist',
      '**/out',
      '**/.gitignore',
      '**/eslint.config.mjs',
      '**/postcss.config.cjs',
      '**/tailwind.config.js'
    ]
  },
  ...fixupConfigRules(
    compat.extends(
      'eslint:recommended',
      'plugin:@typescript-eslint/recommended',
      'plugin:react-hooks/recommended',
      '@electron-toolkit/eslint-config-ts/recommended'
    )
  ),
  {
    plugins: {
      'react-refresh': reactRefresh
    },

    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node
      },

      parser: tsParser
    },

    rules: {
      'react-refresh/only-export-components': ['off'],
      '@typescript-eslint/explicit-function-return-type': ['off']
    }
  }
]
