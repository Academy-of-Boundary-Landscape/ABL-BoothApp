import { describe, it, expect } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { spawnSync } from 'node:child_process'
import stylelint from 'stylelint'

// vitest 跑在 jsdom 下，import.meta.url 是 http://，所以用 vitest root（frontend/）定位。
const frontendRoot = process.cwd()
const configFile = resolve(frontendRoot, 'stylelint.config.mjs')

function lineOf(text: string, index: number): number {
  return text.slice(0, index).split('\n').length
}

describe('stylelint 门禁', () => {
  it('fixture：每条期望规则都命中，合法用例零告警', async () => {
    const fixture = resolve(frontendRoot, 'scripts/fixtures/stylelint/violations.vue')
    const code = readFileSync(fixture, 'utf8')
    const { results } = await stylelint.lint({ code, codeFilename: fixture, configFile })
    const warnings = results.flatMap((r) => r.warnings.map((w) => ({ line: w.line, rule: w.rule })))

    const markers = [...code.matchAll(/expect: ([a-z-]+|\(none\))/g)].map((m) => ({
      rule: m[1],
      line: lineOf(code, m.index ?? 0),
    }))
    expect(markers.length).toBeGreaterThan(0)

    markers.forEach((marker, i) => {
      const start = marker.line + 1
      const end = i + 1 < markers.length ? markers[i + 1].line - 1 : code.split('\n').length
      const inRange = warnings.filter((w) => w.line >= start && w.line <= end)
      if (marker.rule === '(none)') {
        expect(inRange, `(none) 块 ${start}-${end} 不应有告警`).toEqual([])
      } else {
        expect(
          inRange.some((w) => w.rule === marker.rule),
          `第 ${start}-${end} 行附近应有 ${marker.rule} 告警，实际：${JSON.stringify(inRange)}`
        ).toBe(true)
      }
    })
  })

  it('无理由的 stylelint-disable-next-line 会报 descriptionless', async () => {
    const fixture = resolve(frontendRoot, 'scripts/fixtures/stylelint/disables.vue')
    const code = readFileSync(fixture, 'utf8')
    const { results } = await stylelint.lint({ code, codeFilename: fixture, configFile })
    const warnings = results.flatMap((r) => r.warnings)
    expect(warnings.map((w) => w.rule)).toContain('--report-descriptionless-disables')
  })
})

describe('原语边界门禁', () => {
  it('边界脚本：fixture 每条规则都命中', () => {
    const r = spawnSync('node', ['scripts/check-ui-boundary.mjs', '--self-test'], {
      encoding: 'utf8',
    })
    expect(r.status, r.stdout + r.stderr).toBe(0)
  })

  it('边界脚本：src 下对干净文件零命中', () => {
    const r = spawnSync(
      'node',
      ['scripts/check-ui-boundary.mjs', '--files', 'src/views/NotFound.vue'],
      {
        encoding: 'utf8',
      }
    )
    expect(r.status, r.stdout + r.stderr).toBe(0)
  })
})
