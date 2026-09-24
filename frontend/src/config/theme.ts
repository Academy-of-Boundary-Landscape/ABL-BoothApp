/**
 * 主题配置文件
 * 集中管理所有颜色定义，方便主题切换
 */
import type { GlobalThemeOverrides } from 'naive-ui'

/** 一套完整主题的颜色定义（深/浅主题同构）。 */
export interface ThemeColors {
  background: {
    primary: string
    secondary: string
    card: string
    elevated: string
    input: string
  }
  text: {
    primary: string
    secondary: string
    tertiary: string
    disabled: string
    muted: string
    placeholder: string
    white: string
  }
  primary: {
    base: string
    hover: string
    pressed: string
    dark: string
    light: string
  }
  status: {
    success: string
    warning: string
    error: string
    info: string
    cancelled: string
  }
  border: {
    base: string
    focus: string
    light: string
    divider: string
  }
  special: {
    overlay: string
    tooltipBg: string
    highlight: string
    delete: string
  }
}

/** 不随主题切换的几何常量（间距 / 圆角 / 字号 / 阴影）。 */
export interface DesignTokens {
  spacing: {
    xs: string
    sm: string
    md: string
    lg: string
    xl: string
    '2xl': string
  }
  radius: {
    sm: string
    md: string
    lg: string
    xl: string
    pill: string
  }
  font: {
    xs: string
    sm: string
    base: string
    md: string
    lg: string
    xl: string
    '2xl': string
  }
  shadow: {
    sm: string
    md: string
    lg: string
    xl: string
  }
  /** 断点（px）。CSS 侧见 styles/media.css，两处数值必须一致（theme.spec.ts 校验）。 */
  breakpoints: {
    phone: number
    tablet: number
  }
  /** 页宽档位 */
  pageWidth: {
    narrow: string
    content: string
    wide: string
  }
  /** 字重 */
  fontWeight: {
    regular: number
    medium: number
    bold: number
  }
  /** 行高（无单位） */
  lineHeight: {
    tight: number
    base: number
  }
}

/**
 * 全局字体栈（不打包字体文件，离线 + 体积考虑，见 spec §4.4）。
 * generateCSSVariables 输出为 --font-family，Naive 主题也用它。
 */
export const fontFamily =
  "system-ui, -apple-system, 'Segoe UI', 'PingFang SC', 'Microsoft YaHei', 'Noto Sans CJK SC', sans-serif"

/**
 * Naive UI common 覆盖项。已知无效的 `borderColorHover` / `borderColorPressed`
 * 在 ThemeCommonVars 中不存在，已删除（spec §4.3）。
 */
type NaiveCommonOverrides = NonNullable<GlobalThemeOverrides['common']>

// 深色主题配置（当前主题）
export const darkTheme: ThemeColors = {
  // === 基础颜色 ===
  background: {
    primary: '#121212', // 主背景色
    secondary: '#1a1a1a', // 次要背景色
    card: '#101010', // 卡片背景色
    elevated: '#242424', // 提升层背景色
    input: '#121212', // 输入框背景色
  },

  // === 文本颜色 ===
  text: {
    primary: '#E0E0E0', // 主文本
    secondary: '#C8C8C8', // 次要文本
    tertiary: '#A0A0A0', // 三级文本
    disabled: '#888888', // 禁用文本
    muted: '#AAAAAA', // 弱化文本
    placeholder: '#CCCCCC', // 占位符
    white: '#FFFFFF', // 纯白
  },

  // === 主题色 ===
  primary: {
    base: '#8B0012', // 主色（北大红）
    hover: '#9D0015', // 悬停
    pressed: '#77000F', // 按下
    dark: '#5F000C', // 深色变体
    light: 'rgba(139, 0, 18, 0.05)', // 浅色变体
  },

  // === 状态颜色 ===
  status: {
    success: '#4CAF50', // 成功/完成
    warning: '#F5A623', // 警告/待处理
    error: '#F44336', // 错误/取消
    info: '#4A90E2', // 信息
    cancelled: '#9E9E9E', // 已取消
  },

  // === 边框颜色 ===
  border: {
    base: '#2C2C2C', // 基础边框
    focus: '#3A3A3A', // 聚焦边框
    light: '#444444', // 浅色边框
    divider: '#2C2C2C', // 分割线
  },

  // === 特殊颜色 ===
  special: {
    overlay: 'rgba(0, 0, 0, 0.4)', // 遮罩层
    tooltipBg: 'rgba(0, 0, 0, 0.75)', // 提示框背景
    highlight: '#FFDF57', // 高亮/强调
    delete: '#DC3545', // 删除按钮
  },
}
export const lightTheme: ThemeColors = {
  // === 基础颜色 ===
  background: {
    // 关键修改：主背景不再是纯白，而是带有极淡蓝紫调的冷灰，护眼且显高级
    primary: '#F2F5F8',
    // 次要背景（侧边栏/导航栏）：纯白，与主背景形成区分
    secondary: '#FFFFFF',
    // 卡片背景：纯白，在冷灰主背景上会很突出
    card: '#FFFFFF',
    // 悬浮层：纯白
    elevated: '#FFFFFF',
    // 输入框：极淡的灰，让输入框在白卡片上有这一层淡淡的底色，不纯靠边框
    input: '#F9FAFB',
  },

  // === 文本颜色 ===
  text: {
    primary: '#111827', // 加深：接近纯黑的深灰（Tailwind Gray 900），提高阅读清晰度
    secondary: '#4B5563', // 次要文本（Gray 600）
    tertiary: '#9CA3AF', // 三级文本
    disabled: '#D1D5DB', // 禁用
    muted: '#6B7280', // 弱化
    placeholder: '#9CA3AF', // 占位符
    white: '#FFFFFF', // 反白文字
  },

  // === 主题色 ===
  primary: {
    base: '#8B0012',
    hover: '#9D0015',
    pressed: '#77000F',
    dark: '#5F000C',
    light: 'rgba(139, 0, 18, 0.12)',
  },

  // === 状态颜色 ===
  // 保持你原有的现代色盘，这些颜色在白底上表现良好
  status: {
    success: '#10B981',
    warning: '#F59E0B',
    error: '#EF4444',
    info: '#3B82F6',
    cancelled: '#9CA3AF',
  },

  // === 边框颜色 ===
  border: {
    // 关键修改：加深基础边框颜色。
    // 原来的 E5E7EB 在某些显示器上几乎看不见，改为 E2E4E8
    base: '#E2E4E8',
    focus: '#00A99D', // 聚焦颜色
    light: '#F3F4F6', // 极浅分割线
    divider: '#EEF0F2', // 内容分割线，比背景稍深
  },

  // === 特殊颜色 ===
  special: {
    overlay: 'rgba(0, 0, 0, 0.4)',
    tooltipBg: '#1F2937', // Tooltip 保持深色背景
    highlight: '#FEF3C7',
    delete: '#EF4444',
  },
}
// ============================================================
// Design Tokens — 间距 / 圆角 / 字号 / 阴影
// 这些不随主题切换而改变，是全局几何常量
// ============================================================
export const tokens: DesignTokens = {
  // 4px 基准的间距刻度
  spacing: {
    xs: '4px', // 紧凑内间距、icon 与文字间距
    sm: '8px', // 列表项间距、badge padding
    md: '12px', // 卡片内间距、工具栏 padding
    lg: '16px', // 区块间距、section gap
    xl: '24px', // 页面边距、大区块分隔
    '2xl': '32px', // 页面顶部/底部留白
  },

  // 圆角：4 档 + pill
  radius: {
    sm: '4px', // badge、tag、小按钮
    md: '8px', // 卡片、输入框、面板
    lg: '12px', // 商品卡片、modal
    xl: '16px', // 大面板、移动端抽屉顶部
    pill: '999px', // 胶囊按钮、分类 chip
  },

  // 字号层级（基于 rem，根 16px）
  font: {
    xs: '0.75rem', // 12px — 时间戳、辅助标注
    sm: '0.85rem', // 13.6px — 次要文本、描述、meta
    base: '0.95rem', // 15.2px — 正文、列表项
    md: '1.05rem', // 16.8px — 强调正文、子标题
    lg: '1.25rem', // 20px — 区块标题、面板 header
    xl: '1.5rem', // 24px — 页面标题
    '2xl': '2rem', // 32px — Hero 标题
  },

  // 阴影层级
  shadow: {
    sm: '0 1px 3px rgba(0,0,0,0.08)', // 卡片静态
    md: '0 4px 12px rgba(0,0,0,0.1)', // 悬浮、弹出
    lg: '0 8px 24px rgba(0,0,0,0.15)', // modal、抽屉
    xl: '0 12px 32px rgba(0,0,0,0.2)', // 拖拽中
  },

  // 断点（≤ phone 手机；≤ tablet 平板，含手机）。
  // styles/media.css 的 custom media 与这里必须一致（theme.spec.ts 校验）。
  breakpoints: {
    phone: 640,
    tablet: 1024,
  },

  // 页宽档位
  pageWidth: {
    narrow: '640px',
    content: '960px',
    wide: '1280px',
  },

  // 字重
  fontWeight: {
    regular: 400,
    medium: 500,
    bold: 600,
  },

  // 行高（无单位）
  lineHeight: {
    tight: 1.3,
    base: 1.6,
  },
}

// 生成 CSS 变量
export function generateCSSVariables(theme: ThemeColors): string {
  const t = tokens
  return `
    /* ===== 间距 ===== */
    --space-xs: ${t.spacing.xs};
    --space-sm: ${t.spacing.sm};
    --space-md: ${t.spacing.md};
    --space-lg: ${t.spacing.lg};
    --space-xl: ${t.spacing.xl};
    --space-2xl: ${t.spacing['2xl']};

    /* ===== 圆角 ===== */
    --radius-sm: ${t.radius.sm};
    --radius-md: ${t.radius.md};
    --radius-lg: ${t.radius.lg};
    --radius-xl: ${t.radius.xl};
    --radius-pill: ${t.radius.pill};

    /* ===== 字号 ===== */
    --font-xs: ${t.font.xs};
    --font-sm: ${t.font.sm};
    --font-base: ${t.font.base};
    --font-md: ${t.font.md};
    --font-lg: ${t.font.lg};
    --font-xl: ${t.font.xl};
    --font-2xl: ${t.font['2xl']};

    /* ===== 阴影 ===== */
    --shadow-sm: ${t.shadow.sm};
    --shadow-md: ${t.shadow.md};
    --shadow-lg: ${t.shadow.lg};
    --shadow-xl: ${t.shadow.xl};

    /* ===== 页宽 ===== */
    --page-narrow: ${t.pageWidth.narrow};
    --page-content: ${t.pageWidth.content};
    --page-wide: ${t.pageWidth.wide};

    /* ===== 字重 ===== */
    --weight-regular: ${t.fontWeight.regular};
    --weight-medium: ${t.fontWeight.medium};
    --weight-bold: ${t.fontWeight.bold};

    /* ===== 行高 ===== */
    --leading-tight: ${t.lineHeight.tight};
    --leading-base: ${t.lineHeight.base};

    /* ===== 字体栈 ===== */
    --font-family: ${fontFamily};

    /* 背景色 */
    --bg-color: ${theme.background.primary};
    --bg-secondary: ${theme.background.secondary};
    --card-bg-color: ${theme.background.card};
    --bg-elevated: ${theme.background.elevated};

    /* 文本颜色 */
    --primary-text-color: ${theme.text.primary};
    --secondary-text-color: ${theme.text.secondary};
    --tertiary-text-color: ${theme.text.tertiary};
    --text-disabled: ${theme.text.disabled};
    --text-muted: ${theme.text.muted};
    --text-placeholder: ${theme.text.placeholder};
    --text-white: ${theme.text.white};

    /* 主题色 */
    --accent-color: ${theme.primary.base};
    --accent-color-dark: ${theme.primary.dark};
    --accent-color-light: ${theme.primary.light};

    /* 状态颜色 */
    --success-color: ${theme.status.success};
    --warning-color: ${theme.status.warning};
    --error-color: ${theme.status.error};
    --info-color: ${theme.status.info};
    --cancelled-color: ${theme.status.cancelled};

    /* 边框颜色 */
    --border-color: ${theme.border.base};
    --border-color-light: ${theme.border.light};
    --divider-color: ${theme.border.divider};

    /* 特殊颜色 */
    --overlay-color: ${theme.special.overlay};
    --tooltip-bg: ${theme.special.tooltipBg};
    --highlight-color: ${theme.special.highlight};
    --delete-color: ${theme.special.delete};

    /* 交互颜色 */
    --hover-bg-color: ${theme.primary.light};

    /* 组件颜色 */
  `
}

// 生成 Naive UI 主题覆盖配置（包含常用组件的主色同步）
export function generateNaiveUITheme(theme: ThemeColors): GlobalThemeOverrides {
  const t = tokens
  const primary = theme.primary
  const text = theme.text
  const common: NaiveCommonOverrides = {
    primaryColor: primary.base,
    primaryColorHover: primary.hover,
    primaryColorPressed: primary.pressed,
    primaryColorSuppl: primary.base,
    textColorBase: text.primary,
    textColor1: text.primary,
    textColor2: text.secondary,
    bodyColor: theme.background.primary,
    cardColor: theme.background.card,
    modalColor: theme.background.card,
    popoverColor: theme.background.card,
    tableColor: theme.background.card,
    inputColor: theme.background.input,
    borderColor: theme.border.base,
    dividerColor: theme.border.divider,
    successColor: theme.status.success,
    errorColor: theme.status.error,
    warningColor: theme.status.warning,
    infoColor: theme.status.info,
    // 几何项与 token 同步（spec §4.3），不再用 Naive 自己的一套
    borderRadius: t.radius.md,
    borderRadiusSmall: t.radius.sm,
    fontSize: t.font.base,
    fontSizeSmall: t.font.sm,
    fontSizeMedium: t.font.base,
    fontSizeLarge: t.font.md,
    fontFamily,
    lineHeight: String(t.lineHeight.base),
  }
  return {
    common,
    Button: {
      colorPrimary: primary.base,
      colorHoverPrimary: primary.hover,
      colorPressedPrimary: primary.pressed,
      colorFocusPrimary: primary.hover,
      textColorPrimary: text.white,
      textColorHoverPrimary: text.white,
      textColorPressedPrimary: text.white,
      textColorFocusPrimary: text.white,
      colorDisabledPrimary: primary.base,
      textColorDisabledPrimary: text.white,
      rippleColor: primary.light,
    },
    Tag: {
      colorPrimary: primary.light,
      textColorPrimary: primary.base,
      borderColorPrimary: primary.base,
      closeColorHoverPrimary: primary.hover,
      closeColorPressedPrimary: primary.pressed,
    },
    Switch: {
      railColorActive: primary.light,
      railColorActiveHover: primary.light,
      buttonColorActive: primary.base,
      buttonColorHoverActive: primary.hover,
      boxShadowFocus: `0 0 0 2px ${primary.light}`,
    },
    Checkbox: {
      colorChecked: primary.base,
      colorCheckedHover: primary.hover,
      colorCheckedPressed: primary.pressed,
      checkMarkColor: text.white,
    },
    Radio: {
      dotColorActive: text.white,
      buttonColor: primary.base,
      buttonColorHover: primary.hover,
      buttonColorActive: primary.pressed,
      buttonTextColor: text.white,
    },
    Input: {
      borderFocus: theme.border.focus,
      boxShadowFocus: `0 0 0 2px ${primary.light}`,
      caretColor: primary.base,
    },
    Select: {
      borderFocus: theme.border.focus,
      boxShadowFocus: `0 0 0 2px ${primary.light}`,
      caretColor: primary.base,
      colorActive: primary.light,
    },
    Slider: {
      fillColor: primary.base,
      fillColorHover: primary.hover,
      handleColor: primary.base,
      handleColorHover: primary.hover,
      handleColorActive: primary.pressed,
      indicatorColor: primary.base,
    },
    Progress: {
      colorInfo: primary.base,
    },
    Alert: {
      colorInfo: primary.light,
      titleTextColorInfo: primary.base,
      contentTextColorInfo: text.secondary,
    },
    Card: {
      borderRadius: t.radius.lg,
    },
    Dialog: {
      borderRadius: t.radius.lg,
    },
  }
}

// 导出默认主题（当前为深色）
export default lightTheme
