import js from '@eslint/js'
import pluginVue from 'eslint-plugin-vue'
import skipFormatting from '@vue/eslint-config-prettier/skip-formatting'
import tsParser from '@typescript-eslint/parser'

export default [
  {
    name: 'app/files-to-lint',
    files: ['**/*.{js,mjs,jsx,vue}'],
  },
  {
    name: 'app/files-to-ignore',
    ignores: ['**/dist/**', '**/dist-ssr/**', '**/coverage/**', '**/node_modules/**'],
  },
  js.configs.recommended,
  ...pluginVue.configs['flat/essential'],
  skipFormatting,
  {
    name: 'app/language-options',
    languageOptions: {
      ecmaVersion: 'latest',
      sourceType: 'module',
      globals: {
        window: 'readonly',
        document: 'readonly',
        console: 'readonly',
        navigator: 'readonly',
        fetch: 'readonly',
        setTimeout: 'readonly',
        clearTimeout: 'readonly',
        setInterval: 'readonly',
        clearInterval: 'readonly',
        URL: 'readonly',
        Blob: 'readonly',
        File: 'readonly',
        FormData: 'readonly',
        Image: 'readonly',
        localStorage: 'readonly',
        sessionStorage: 'readonly',
        alert: 'readonly',
        confirm: 'readonly',
        // 以下几个是运行下来才发现的漏项：历史代码里确实在用，
        // 简报给的清单没覆盖到，这里按需要补全，而不是关掉 no-undef。
        requestAnimationFrame: 'readonly',
        cancelAnimationFrame: 'readonly',
        ResizeObserver: 'readonly',
        AbortController: 'readonly',
        performance: 'readonly',
        Headers: 'readonly',
        URLSearchParams: 'readonly',
        TextEncoder: 'readonly',
      },
    },
  },
  {
    // 仓库里实际存在 3 个 <script setup lang="ts"> 的 .vue 文件（About.vue、
    // Help.vue、UpdateModal.vue），跟简报「当前零 TS 代码」的前提不符。
    // 这里只让 vue-eslint-parser 把它们的 <script> 块交给
    // @typescript-eslint/parser 解析（该包已经是 devDependency，随
    // @vue/eslint-config-typescript 一起装的），只解决“能不能解析”，
    // 不引入任何类型感知规则，也不需要 tsconfig.json —— 那部分仍然留给
    // ③b。没有它，这 3 个文件在 lint 时会直接报 parsing error，等于
    // 完全没被 lint 覆盖。
    name: 'app/vue-ts-script-blocks',
    files: ['**/*.vue'],
    languageOptions: {
      parserOptions: {
        parser: tsParser,
      },
    },
  },
  {
    // vue/multi-word-component-names 是给「可复用组件」防止和原生/第三方
    // 元素撞名的规则。About.vue / Help.vue 是路由级页面组件，只通过
    // vue-router 加载，从不作为 <xxx> 标签被引用，撞名风险为零；改名要联
    // 动改 router/index.js 的 import，属于本轮范围外的重构，所以只对这
    // 两个文件关规则，其余组件仍然受它保护。
    name: 'app/single-word-view-names',
    files: ['src/views/About.vue', 'src/views/Help.vue'],
    rules: {
      'vue/multi-word-component-names': 'off',
    },
  },
]
