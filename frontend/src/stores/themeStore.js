// src/stores/themeStore.js
import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { useStorage } from '@vueuse/core'
import { colord } from 'colord'
import { darkTheme, lightTheme, generateCSSVariables, generateNaiveUITheme } from '@/config/theme'

export const useThemeStore = defineStore('theme', () => {
  // 1. 状态：是否暗黑模式 (持久化)
  const isDark = useStorage('isDark', false)

  // 2. 状态：自定义主色 (持久化)
  // 如果用户没选过，就用默认配置里的颜色
  const customPrimaryColor = useStorage('customPrimary', null)

  // 2.5 商品图比例偏好 (持久化)
  // '3:4' = 竖版（默认，适合立绘/明信片/海报）
  // '1:1' = 正方形（适合亚克力/徽章/周边小物）
  // 影响：商品网格展示比例 + 上传裁剪器的默认比例
  const productImageAspect = useStorage('productImageAspect', '3:4')

  // 3. 计算当前的基础配置 (根据模式选择 lightTheme 或 darkTheme)
  const currentBaseTheme = computed(() => isDark.value ? darkTheme : lightTheme)

  // 4. 计算最终的颜色配置
  // 如果用户选了自定义颜色，覆盖默认配置里的 primary.base
  const currentThemeConfig = computed(() => {
    const theme = { ...currentBaseTheme.value }

    // 如果有自定义颜色，动态计算衍生色
    if (customPrimaryColor.value) {
      const c = customPrimaryColor.value
      theme.primary = {
        base: c,
        hover: colord(c).lighten(0.05).toHex(),
        pressed: colord(c).darken(0.05).toHex(),
        dark: colord(c).darken(0.2).toHex(),
        light: colord(c).alpha(isDark.value ? 0.05 : 0.1).toRgbString(),
      }
    }
    return theme
  })

  // 5. 生成 Naive UI 需要的对象
  const naiveThemeOverrides = computed(() => generateNaiveUITheme(currentThemeConfig.value))

  // 6. 生成 CSS 变量字符串
  const cssVariables = computed(() => generateCSSVariables(currentThemeConfig.value))

  // 7. 监听变化，自动更新 CSS 变量到 Head
  watch(cssVariables, (vars) => {
    let styleTag = document.getElementById('theme-vars')
    if (!styleTag) {
      styleTag = document.createElement('style')
      styleTag.id = 'theme-vars'
      document.head.appendChild(styleTag)
    }
    styleTag.textContent = `:root { ${vars} }`
  }, { immediate: true })

  // 动作：重置颜色
  function resetColor() {
    customPrimaryColor.value = null
  }

  return {
    isDark,
    customPrimaryColor,
    productImageAspect,
    currentBaseTheme, // 供 App.vue 判断使用 naive 的 darkTheme 还是 null
    naiveThemeOverrides,
    resetColor
  }
})
