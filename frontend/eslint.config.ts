import { globalIgnores } from 'eslint/config'
import {
  defineConfigWithVueTs,
  vueTsConfigs,
  configureVueProject,
} from '@vue/eslint-config-typescript'
import pluginVue from 'eslint-plugin-vue'
import skipFormatting from '@vue/eslint-config-prettier/skip-formatting'

// 全部 .vue 都是 <script setup lang="ts">；混进 JS 的 script 块会被 lint 拒绝。
configureVueProject({ scriptLangs: ['ts'] })

export default defineConfigWithVueTs(
  { name: 'app/files-to-lint', files: ['**/*.{js,mjs,ts,mts,vue}'] },
  globalIgnores([
    '**/dist/**',
    '**/dist-ssr/**',
    '**/coverage/**',
    // 生成物：由 scripts/gen-api.mjs 从 src-tauri/openapi.json 生成
    'src/api/schema.d.ts',
    // 第三方类型定义的修补副本，保持与上游近乎逐字一致
    'types/openapi-typescript-helpers.d.ts',
  ]),
  pluginVue.configs['flat/essential'],
  vueTsConfigs.recommended,
  {
    name: 'app/ts-rules',
    rules: {
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/ban-ts-comment': [
        'error',
        { 'ts-expect-error': 'allow-with-description', 'ts-ignore': true, 'ts-nocheck': true },
      ],
    },
  },
  {
    // vue/multi-word-component-names 是给「可复用组件」防止和原生/第三方元素撞名的规则。
    // About.vue / Help.vue 是路由级页面组件，只通过 vue-router 加载，从不作为 <xxx> 标签
    // 被引用，撞名风险为零；改名要联动改 router，不值得。
    name: 'app/single-word-view-names',
    files: ['src/views/About.vue', 'src/views/Help.vue'],
    rules: {
      'vue/multi-word-component-names': 'off',
    },
  },
  skipFormatting,
)
