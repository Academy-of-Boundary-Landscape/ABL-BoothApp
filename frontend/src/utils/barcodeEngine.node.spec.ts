// @vitest-environment node
//
// 证明「打包进来的那份 wasm」在 Node 里可用、且全程不联网：
// 1) 无原生 BarcodeDetector 时走 ponyfill，并把 locateFile 指向本地 wasm（不以 http 开头）。
// 2) 用 writer 生成 EAN-13 / UPC-A / Code128 / QR 的 PNG，再用 reader + 本地 wasm 解回原值。
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { afterEach, describe, expect, it, vi } from 'vitest'

const engineMocks = vi.hoisted(() => ({
  prepareZXingModule: vi.fn(),
  detect: vi.fn(),
}))

vi.mock('barcode-detector/ponyfill', () => ({
  BarcodeDetector: class {
    detect = engineMocks.detect
  },
  prepareZXingModule: engineMocks.prepareZXingModule,
}))

// 测试中禁止网络：ponyfill 默认会去 jsDelivr 拉 wasm，一拉就报错。
vi.stubGlobal('fetch', () => {
  throw new Error('no network')
})

const readerWasmPath = fileURLToPath(
  new URL('../../node_modules/zxing-wasm/dist/reader/zxing_reader.wasm', import.meta.url)
)
const writerWasmPath = fileURLToPath(
  new URL('../../node_modules/zxing-wasm/dist/writer/zxing_writer.wasm', import.meta.url)
)

function readWasmBinary(path: string): ArrayBuffer {
  const buffer = readFileSync(path)
  return buffer.buffer.slice(
    buffer.byteOffset,
    buffer.byteOffset + buffer.byteLength
  ) as ArrayBuffer
}

afterEach(() => {
  engineMocks.prepareZXingModule.mockClear()
  engineMocks.detect.mockClear()
})

describe('barcodeEngine（无原生 BarcodeDetector）', () => {
  it('调用 prepareZXingModule，且 locateFile 指向本地 wasm（不以 http 开头）', async () => {
    vi.stubGlobal('BarcodeDetector', undefined)
    vi.resetModules()

    const { loadDetector } = await import('@/utils/barcodeEngine')
    await loadDetector()

    expect(engineMocks.prepareZXingModule).toHaveBeenCalledTimes(1)
    const options = engineMocks.prepareZXingModule.mock.calls[0][0] as {
      overrides: { locateFile: (path: string, prefix: string) => string }
    }
    const located = options.overrides.locateFile('zxing_reader.wasm', 'x/')
    expect(located.startsWith('http')).toBe(false)
    // 非 wasm 文件必须原样回落到 prefix，保证库内部相对路径不被破坏。
    expect(options.overrides.locateFile('zxing_reader.js', 'x/')).toBe('x/zxing_reader.js')
  })
})

describe('自带 wasm 解码（node，无网络）', () => {
  it('EAN-13 / UPC-A / Code128 / QR 都能解出原值', async () => {
    const { prepareZXingModule: prepareReader, readBarcodes } = await import('zxing-wasm/reader')
    const { prepareZXingModule: prepareWriter, writeBarcode } = await import('zxing-wasm/writer')

    prepareReader({ overrides: { wasmBinary: readWasmBinary(readerWasmPath) } })
    prepareWriter({ overrides: { wasmBinary: readWasmBinary(writerWasmPath) } })

    const cases = [
      { format: 'EAN13', text: '4901234567894' },
      { format: 'UPCA', text: '036000291452' },
      { format: 'Code128', text: 'P001' },
      { format: 'QRCode', text: 'P001' },
    ] as const

    for (const testCase of cases) {
      const written = await writeBarcode(testCase.text, { format: testCase.format })
      expect(written.error).toBe('')
      expect(written.image).not.toBeNull()

      const bytes = new Uint8Array(await written.image!.arrayBuffer())
      const results = await readBarcodes(bytes, { formats: [testCase.format] })
      expect(results.length).toBeGreaterThan(0)
      // ZXing 读 UPC-A 时常按「前补 0 的 EAN-13」输出（12 位 → 13 位），
      // 这正是 matchBarcode 要做 UPC-A/EAN-13 等价的原因，两种形态都算解出原值。
      const decoded = results[0].text
      expect(decoded === testCase.text || decoded === `0${testCase.text}`).toBe(true)
    }
  })
})
