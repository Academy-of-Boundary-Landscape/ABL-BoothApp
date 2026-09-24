// composables/useViewport.ts —— JS 侧断点的唯一入口，数值与 styles/media.css 同源（tokens.breakpoints）
import { useBreakpoints } from '@vueuse/core'
import { tokens } from '@/config/theme'

const bp = useBreakpoints({
  phone: tokens.breakpoints.phone + 1,
  tablet: tokens.breakpoints.tablet + 1,
})

export function useViewport() {
  return { isPhone: bp.smaller('phone'), isTablet: bp.smaller('tablet') }
}
