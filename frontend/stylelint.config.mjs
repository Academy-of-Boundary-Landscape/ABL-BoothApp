// 门禁：设计刻度只能来自 token（config/theme.ts → CSS 变量）。豁免只能行内写，且必须写理由：
//   /* stylelint-disable-next-line <rule> -- 理由 */
// 规则写错会静默放过——src/gates.spec.ts 用 scripts/fixtures/stylelint/violations.vue 断言每条都能报错。
const SPACE = String.raw`(?:var\(--space-[a-z0-9]+\)|0|auto|calc\((?:\s|var\(--space-[a-z0-9]+\)|[-+*/]|\d+(?:\.\d+)?)+\))`
const spaceList = new RegExp(`^${SPACE}(?:\\s+${SPACE}){0,3}$`)
const RADIUS = String.raw`(?:var\(--radius-[a-z0-9]+\)|0|50%)`
const radiusList = new RegExp(`^${RADIUS}(?:\\s+${RADIUS}){0,3}$`)
// CSS 系统色关键字（color-named 不管它们），同样能绕过「颜色只来自 token」。
const SYSTEM_COLOR =
  /(?<![\w-])(?:canvas|canvastext|linktext|visitedtext|activetext|buttonface|buttontext|buttonborder|field|fieldtext|highlight|highlighttext|selecteditem|selecteditemtext|mark|marktext|graytext|accentcolor|accentcolortext)(?![\w-])/i
// token 命名空间：组件里自己定义同名前缀的变量（`--space-x: 13px`）就能绕过下面的值域白名单。
const TOKEN_PREFIX = 'space|font|weight|radius|shadow|page|leading'

export default {
  extends: ['stylelint-config-recommended', 'stylelint-config-recommended-vue'],
  reportDescriptionlessDisables: true,
  reportNeedlessDisables: true,
  reportInvalidScopeDisables: true,
  rules: {
    'no-descending-specificity': null, // 存量代码大量触发，与本次目标无关
    'color-no-hex': true,
    'color-named': 'never',
    'function-disallowed-list': [
      'rgb',
      'rgba',
      'hsl',
      'hsla',
      'hwb',
      'lab',
      'lch',
      'oklab',
      'oklch',
      'color',
    ],
    'declaration-property-value-disallowed-list': {
      '/^(color|background(-color)?|border(-(top|right|bottom|left|block|inline)(-(start|end))?)?(-color)?|outline(-color)?|fill|stroke|caret-color|accent-color|text-decoration(-color)?|column-rule(-color)?|text-emphasis-color|-webkit-text-fill-color|-webkit-text-stroke(-color)?)$/':
        [SYSTEM_COLOR],
    },
    // 组件里定义 token 命名空间的变量（`--space-x: 13px`）就能绕过下面的值域白名单；token 只来自 config/theme.ts
    'property-disallowed-list': [`/^--(?:${TOKEN_PREFIX})-/`],
    'at-rule-disallowed-list': ['custom-media', 'container'], // 断点只来自 styles/media.css
    'declaration-property-value-allowed-list': {
      'font-size': [/^var\(--font-[a-z0-9]+\)$/, 'inherit'],
      'font-weight': [/^var\(--weight-[a-z]+\)$/, 'inherit'],
      // 禁止 font 简写：它能把 font-size / font-weight 藏在里面绕过上面的限制。
      font: ['inherit'],
      '/^border(-(top|bottom)-(left|right)|-(start|end)-(start|end))?-radius$/': [radiusList],
      '/^(-webkit-|-moz-)?box-shadow$/': [/^var\(--shadow-[a-z0-9]+\)$/, 'none'],
      '/^(grid-)?(row-|column-)?gap$/': [spaceList],
      '/^(padding|margin)(-(top|right|bottom|left|inline|block)(-start|-end)?)?$/': [spaceList],
      'max-width': [
        /^var\(--page-[a-z]+\)$/,
        /%$/,
        'none',
        // 内容级小宽度（截断文字等）放行；页面级宽度（≥ 640px = --page-narrow）必须用 --page-*
        /^\d+(\.\d+)?(ch|vw)$/,
        /^[1-3]?\d(\.\d+)?r?em$/,
        /^[1-3]?\d{1,2}px$/,
        /^min\((?:(?!\d{4,}px|[4-9]\d\dpx)[^;])*\)$/,
      ],
    },
    'media-feature-name-disallowed-list': ['/width$/', '/height$/'], // 只允许 custom media 与 prefers-* / orientation / hover / pointer
    'comment-no-empty': true,
  },
  overrides: [
    { files: ['**/*.vue'], customSyntax: 'postcss-html' },
    { files: ['src/styles/media.css'], rules: { 'at-rule-disallowed-list': ['container'] } },
  ],
}
