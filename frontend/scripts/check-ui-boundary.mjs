#!/usr/bin/env node
// 原语边界门禁（spec §6.2）：禁止绕过 components/ui 原语层的写法。
//
// 扫描 src/**/*.{vue,ts}（排除 components/ui/**、composables/useFeedback.ts、*.spec.ts），
// 命中行上一行写成 `// ui-boundary-ignore: 理由` 或 `<!-- ui-boundary-ignore: 理由 -->` 可豁免（理由必填）。
//
// 用法：
//   node scripts/check-ui-boundary.mjs               默认：有命中 exit 1，逐条打印 `file:line rule`
//   node scripts/check-ui-boundary.mjs --files a b  只查给定文件
//   node scripts/check-ui-boundary.mjs --report     只打印按文件汇总的计数，exit 0
//   node scripts/check-ui-boundary.mjs --self-test  对 scripts/fixtures/boundary/ 自测，规则不齐 exit 1
import { readFileSync, readdirSync } from 'node:fs'
import { basename, dirname, join, relative, resolve, extname } from 'node:path'
import { fileURLToPath } from 'node:url'

const SCRIPT_DIR = dirname(fileURLToPath(import.meta.url))
const ROOT = dirname(SCRIPT_DIR) // frontend/
const SRC_DIR = join(ROOT, 'src')
const FIXTURE_DIR = join(SCRIPT_DIR, 'fixtures', 'boundary')

const LEGACY_CLASSES = [
  'page-header',
  'section-header',
  'error-message',
  'loading-message',
  'empty-hint',
  'empty-line',
  'modal-header',
  'table-wrapper',
]

const LEGACY_COMPONENTS = ['AppModal', 'EmptyGuide', 'CollapsibleSection']

/** 每条 规则 id → 在文本里找出所有命中位置 */
const RULES = [
  {
    id: 'native-dialog',
    find(text) {
      const hits = []
      const re = /\b(?:window\.)?(?:alert|confirm)\(/g
      let m
      while ((m = re.exec(text))) {
        // 排除 `fb.confirm(` / `.alert(` 这类成员调用
        if (text[m.index - 1] === '.') continue
        hits.push({ index: m.index, match: m[0] })
      }
      return hits
    },
  },
  {
    id: 'naive-feedback',
    find(text) {
      const hits = []
      const importRe = /import\s+([\s\S]*?)\s+from\s+['"]naive-ui['"]/g
      let m
      while ((m = importRe.exec(text))) {
        if (/\b(?:useMessage|useDialog|NModal)\b/.test(m[1])) {
          hits.push({ index: m.index, match: 'naive-ui feedback import' })
        }
      }
      const tagRe = /<n-modal[\s>]/g
      while ((m = tagRe.exec(text))) {
        hits.push({ index: m.index, match: '<n-modal' })
      }
      return hits
    },
  },
  {
    id: 'legacy-class',
    find(text) {
      const hits = []
      const attrRe = /(?<![\w:-])(?:v-bind:)?(?::class|class)\s*=\s*("([^"]*)"|'([^']*)')/gs
      let m
      while ((m = attrRe.exec(text))) {
        const value = m[2] ?? m[3] ?? ''
        const valueStart = m.index + m[0].length - value.length - 1
        for (const name of LEGACY_CLASSES) {
          const re = new RegExp(`(?<![\\w-])${name}(?![\\w-])`, 'g')
          let n
          while ((n = re.exec(value))) {
            hits.push({ index: valueStart + n.index, match: name })
          }
        }
      }
      return hits
    },
  },
  {
    id: 'legacy-component',
    find(text) {
      const hits = []
      const re = new RegExp(
        `from\\s+['"][^'"]*shared/(?:${LEGACY_COMPONENTS.join('|')})(?:\\.vue)?['"]`,
        'g'
      )
      let m
      while ((m = re.exec(text))) {
        hits.push({ index: m.index, match: m[0] })
      }
      return hits
    },
  },
  {
    id: 'inner-width',
    find(text) {
      const hits = []
      const re = /window\.innerWidth/g
      let m
      while ((m = re.exec(text))) {
        hits.push({ index: m.index, match: m[0] })
      }
      return hits
    },
  },
]

const RULE_IDS = RULES.map((r) => r.id)

function lineAt(text, index) {
  let line = 1
  for (let i = 0; i < index; i++) {
    if (text[i] === '\n') line++
  }
  return line
}

function hasIgnoreReason(prevLine) {
  const m = prevLine.match(/(?:\/\/|<!--)\s*ui-boundary-ignore:(.*)$/)
  if (!m) return false
  return m[1].replace(/-->\s*$/, '').trim().length > 0
}

function scanText(text) {
  const lines = text.split('\n')
  const hits = []
  let suppressed = 0
  for (const rule of RULES) {
    for (const raw of rule.find(text)) {
      const line = lineAt(text, raw.index)
      const prev = lines[line - 2] ?? ''
      if (hasIgnoreReason(prev)) {
        suppressed++
        continue
      }
      hits.push({ line, rule: rule.id, match: raw.match })
    }
  }
  hits.sort((a, b) => a.line - b.line || a.rule.localeCompare(b.rule))
  return { hits, suppressed }
}

function isExcluded(absPath) {
  const rel = relative(ROOT, absPath).split('\\').join('/')
  if (rel.startsWith('src/components/ui/')) return true
  if (rel === 'src/composables/useFeedback.ts') return true
  if (basename(rel).endsWith('.spec.ts')) return true
  return false
}

function walk(dir, out) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, entry.name)
    if (entry.isDirectory()) walk(p, out)
    else if (entry.isFile() && ['.vue', '.ts'].includes(extname(entry.name)) && !isExcluded(p)) {
      out.push(p)
    }
  }
  return out
}

function scanFiles(files) {
  const results = []
  for (const file of files) {
    const text = readFileSync(file, 'utf8')
    const { hits, suppressed } = scanText(text)
    if (hits.length || suppressed) {
      results.push({ file, rel: relative(ROOT, file).split('\\').join('/'), hits, suppressed })
    }
  }
  return results
}

function parseArgs(argv) {
  const opts = { files: [], selfTest: false, report: false }
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i]
    if (a === '--self-test') opts.selfTest = true
    else if (a === '--report') opts.report = true
    else if (a === '--files') {
      while (i + 1 < argv.length && !argv[i + 1].startsWith('--')) opts.files.push(argv[++i])
    } else if (a.startsWith('--files=')) {
      opts.files.push(...a.slice('--files='.length).split(/\s+/).filter(Boolean))
    } else {
      console.error(`未知参数: ${a}`)
      process.exit(2)
    }
  }
  return opts
}

function selfTest() {
  const files = walk(FIXTURE_DIR, []).sort()
  const results = scanFiles(files)
  const hits = results.flatMap((r) => r.hits)
  const missing = RULE_IDS.filter((id) => !hits.some((h) => h.rule === id))
  const suppressed = results.reduce((n, r) => n + r.suppressed, 0)
  if (missing.length) {
    console.error(`[self-test] 以下规则在 fixture 里没有命中: ${missing.join(', ')}`)
    process.exit(1)
  }
  if (suppressed === 0) {
    console.error('[self-test] fixture 里带理由的 ui-boundary-ignore 没有生效')
    process.exit(1)
  }
  // 负例：成员调用不得被 native-dialog 误伤
  for (const file of files) {
    const lines = readFileSync(file, 'utf8').split('\n')
    for (let i = 0; i < lines.length; i++) {
      if (/fb\.confirm|foo\.alert/.test(lines[i]) && hits.some((h) => h.line === i + 1)) {
        console.error(`[self-test] 成员调用被误伤: ${relative(ROOT, file)}:${i + 1}`)
        process.exit(1)
      }
    }
  }
  console.log(
    `[self-test] ok：${files.length} 个 fixture，命中 ${hits.length} 条，豁免 ${suppressed} 条，覆盖 ${RULE_IDS.length} 条规则`
  )
  return true
}

function main() {
  const opts = parseArgs(process.argv.slice(2))
  if (opts.selfTest) {
    selfTest()
    return
  }
  const files = opts.files.length
    ? opts.files.map((f) => resolve(process.cwd(), f)).filter((f) => !isExcluded(f))
    : walk(SRC_DIR, [])
  const results = scanFiles(files)
  if (opts.report) {
    for (const r of results) console.log(`${r.hits.length} ${r.rel}`)
    return
  }
  let total = 0
  for (const r of results) {
    for (const h of r.hits) {
      total++
      console.log(`${r.rel}:${h.line} ${h.rule}`)
    }
  }
  if (total > 0) process.exit(1)
}

main()
