// docs/.vitepress/theme/index.ts
import DefaultTheme from 'vitepress/theme'
import type { Theme } from 'vitepress'
import { defineClientComponent } from 'vitepress'
import Landing from './components/landing.vue'
import Timeline from './components/Timeline.vue'
export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component('Landing', Landing)
    app.component('Timeline', Timeline)
  }
} satisfies Theme
