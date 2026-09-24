// 门禁：设计刻度只能来自 token（config/theme.ts → CSS 变量）。豁免只能行内写，且必须写理由：
//   /* stylelint-disable-next-line <rule> -- 理由 */
// 规则写错会静默放过——src/gates.spec.ts 用 scripts/fixtures/stylelint/violations.vue 断言每条都能报错。
const SPACE = String.raw`(?:var\(--space-[a-z0-9]+\)|0|auto|calc\((?:\s|var\(--space-[a-z0-9]+\)|[-+*/]|\d+(?:\.\d+)?)+\))`
const spaceList = new RegExp(`^${SPACE}(?:\\s+${SPACE}){0,3}$`)
const RADIUS = String.raw`(?:var\(--radius-[a-z0-9]+\)|0|50%)`
const radiusList = new RegExp(`^${RADIUS}(?:\\s+${RADIUS}){0,3}$`)

export default {
  extends: ['stylelint-config-recommended', 'stylelint-config-recommended-vue'],
  reportDescriptionlessDisables: true,
  reportNeedlessDisables: true,
  reportInvalidScopeDisables: true,
  rules: {
    'no-descending-specificity': null, // 存量代码大量触发，与本次目标无关
    'color-no-hex': true,
    'color-named': 'never',
    'function-disallowed-list': ['rgb', 'rgba', 'hsl', 'hsla'],
    'declaration-property-value-allowed-list': {
      'font-size': [/^var\(--font-[a-z0-9]+\)$/, 'inherit'],
      'font-weight': [/^var\(--weight-[a-z]+\)$/, 'inherit'],
      // 禁止 font 简写：它能把 font-size / font-weight 藏在里面绕过上面的限制。
      font: ['inherit'],
      '/^border(-(top|bottom)-(left|right))?-radius$/': [radiusList],
      'box-shadow': [/^var\(--shadow-[a-z0-9]+\)$/, 'none'],
      '/^(row-|column-)?gap$/': [spaceList],
      '/^(padding|margin)(-(top|right|bottom|left|inline|block)(-start|-end)?)?$/': [spaceList],
      'max-width': [
        /^var\(--page-[a-z]+\)$/,
        /%$/,
        'none',
        /^\d+(\.\d+)?(ch|em|rem|vw)$/,
        /^[1-3]?\d{1,2}px$/,
        /^min\(/,
      ],
    },
    'media-feature-name-disallowed-list': ['/width$/', '/height$/'], // 只允许 custom media 与 prefers-* / orientation / hover / pointer
    'comment-no-empty': true,
  },
  overrides: [{ files: ['**/*.vue'], customSyntax: 'postcss-html' }],
}
