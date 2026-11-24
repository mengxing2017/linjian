import js from '@eslint/js'
import ts from 'typescript-eslint'
import vue from 'eslint-plugin-vue'
export default ts.config(
  { ignores: ['dist', 'node_modules'] },
  js.configs.recommended,
  ...ts.configs.recommended,
  ...vue.configs['flat/essential'],
  {
    files: ['src/**/*.{ts,vue}'],
    languageOptions: { globals: { uni: 'readonly', UniApp: 'readonly', console: 'readonly', getCurrentPages: 'readonly', getApp: 'readonly', setTimeout: 'readonly' } },
    rules: {
      'vue/multi-word-component-names': 'off',
      'no-restricted-globals': ['error', { name: 'fetch', message: '使用 feature/api client' }],
    },
  },
  {
    files: ['src/custom-tab-bar/index.js'],
    languageOptions: { globals: { Component: 'readonly', getApp: 'readonly', setTimeout: 'readonly' } },
  },
  {
    files: ['src/**/*.vue'],
    languageOptions: { parserOptions: { parser: ts.parser, extraFileExtensions: ['.vue'] } },
  },
)
