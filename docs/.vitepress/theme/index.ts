// docs/.vitepress/theme/index.ts
import DefaultTheme from 'vitepress/theme'
import type { Theme } from 'vitepress'
import { defineClientComponent } from 'vitepress'
import Landing from './components/Landing.vue'
import LandingEn from './components/LandingEn.vue'
import LandingJp from './components/LandingJp.vue'
import Timeline from './components/Timeline.vue'
import TimelineEn from './components/TimelineEn.vue'
import TimelineJp from './components/TimelineJp.vue'
export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component('Landing', Landing)
    app.component('LandingEn', LandingEn)
    app.component('LandingJp', LandingJp)
    app.component('Timeline', Timeline)
    app.component('TimelineEn', TimelineEn)
    app.component('TimelineJp', TimelineJp)
  }
} satisfies Theme
