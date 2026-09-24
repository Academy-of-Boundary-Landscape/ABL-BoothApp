import { describe, it, expect, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { tokens, generateCSSVariables, generateNaiveUITheme, lightTheme } from './theme'

const { breakpointsCalls, useBreakpointsMock } = vi.hoisted(() => {
  const breakpointsCalls: unknown[] = []
  return {
    breakpointsCalls,
    useBreakpointsMock: (bp: unknown) => {
      breakpointsCalls.push(bp)
      return { smaller: () => ({ value: true }) }
    },
  }
})

vi.mock('@vueuse/core', () => ({ useBreakpoints: useBreakpointsMock }))

// jsdom 环境下 import.meta.url 是 http:// 而非 file://，用 vitest root（frontend/）定位
const mediaCss = readFileSync(resolve(process.cwd(), 'src/styles/media.css'), 'utf8')

describe('tokens', () => {
  it('media.css 与 tokens.breakpoints 同源', () => {
    const { phone, tablet } = tokens.breakpoints
    expect(mediaCss).toContain(`@custom-media --phone (max-width: ${phone}px);`)
    expect(mediaCss).toContain(`@custom-media --tablet (max-width: ${tablet}px);`)
    expect(mediaCss).toContain(`@custom-media --not-phone (min-width: ${phone + 1}px);`)
    expect(mediaCss).toContain(`@custom-media --desktop (min-width: ${tablet + 1}px);`)
  })
  it('输出新增的 CSS 变量', () => {
    const css = generateCSSVariables(lightTheme)
    for (const v of [
      '--page-narrow: 640px',
      '--page-content: 960px',
      '--page-wide: 1280px',
      '--weight-regular: 400',
      '--weight-medium: 500',
      '--weight-bold: 600',
      '--leading-tight: 1.3',
      '--leading-base: 1.6',
    ])
      expect(css).toContain(v)
  })
  it('Naive 几何项来自 token', () => {
    const c = generateNaiveUITheme(lightTheme).common!
    expect(c.borderRadius).toBe(tokens.radius.md)
    expect(c.borderRadiusSmall).toBe(tokens.radius.sm)
    expect(c.fontSizeMedium).toBe(tokens.font.base)
    expect(c.fontFamily).toContain('PingFang SC')
    expect(generateNaiveUITheme(lightTheme).Card?.borderRadius).toBe(tokens.radius.lg)
  })
  it('不再输出无效的 Naive 键', () => {
    const c = generateNaiveUITheme(lightTheme).common as Record<string, unknown>
    expect(c.borderColorHover).toBeUndefined()
    expect(c.borderColorPressed).toBeUndefined()
  })
})

describe('useViewport', () => {
  it('断点数值转成 useBreakpoints 的 min-width 边界', async () => {
    const { useViewport } = await import('@/composables/useViewport')
    expect(breakpointsCalls).toEqual([
      { phone: tokens.breakpoints.phone + 1, tablet: tokens.breakpoints.tablet + 1 },
    ])
    const { isPhone, isTablet } = useViewport()
    expect(isPhone.value).toBe(true)
    expect(isTablet.value).toBe(true)
  })
})
