/**
 * 扫码识别引擎加载（spec §6.1）。
 *
 * 原生 `BarcodeDetector` 的 `getSupportedFormats()` 覆盖 `ean_13` 与 `qr_code` 时优先用原生
 * （Chrome Android / macOS）；否则退回 `barcode-detector/ponyfill`（底层 zxing-wasm）。
 *
 * wasm 必须自带：ponyfill 默认从 jsDelivr CDN 拉 `zxing_reader.wasm`（约 1.1MB），场馆常常断网。
 * 这里用 Vite 的 `?url` 把 wasm 打进 `dist/assets/`，再用 `prepareZXingModule` 的 `locateFile`
 * 指过去，Tauri 资源协议与 LAN 浏览器都从本机加载。
 *
 * 本模块只在扫码面板首次打开时被动态 `import()`，点单页首屏不受影响。
 */
import wasmUrl from 'zxing-wasm/reader/zxing_reader.wasm?url'

export const SCAN_FORMATS = [
  'ean_13',
  'ean_8',
  'upc_a',
  'upc_e',
  'code_128',
  'code_39',
  'qr_code',
] as const

export interface Detector {
  detect(source: ImageBitmapSource): Promise<{ rawValue: string }[]>
}

interface NativeDetectorCtor {
  new (options?: { formats?: readonly string[] }): Detector
  getSupportedFormats(): Promise<readonly string[]>
}

/** 单例缓存：同一个 Detector 只创建一次。加载失败不缓存，下次打开面板可重试。 */
let detectorPromise: Promise<Detector> | null = null

export function loadDetector(): Promise<Detector> {
  if (!detectorPromise) {
    detectorPromise = createDetector().catch((error: unknown) => {
      detectorPromise = null
      throw error
    })
  }
  return detectorPromise
}

async function createDetector(): Promise<Detector> {
  const native = (globalThis as { BarcodeDetector?: NativeDetectorCtor }).BarcodeDetector
  if (native && typeof native.getSupportedFormats === 'function') {
    try {
      const supported = await native.getSupportedFormats()
      if (supported.includes('ean_13') && supported.includes('qr_code')) {
        // 只传原生确实支持的格式，避免个别实现因未知格式抛错。
        const formats = SCAN_FORMATS.filter((format) => supported.includes(format))
        return new native({ formats })
      }
    } catch {
      // 原生实现异常（权限、实现 bug 等）：静默退回 WASM，不打断扫码。
    }
  }

  const { BarcodeDetector, prepareZXingModule } = await import('barcode-detector/ponyfill')
  prepareZXingModule({
    overrides: {
      locateFile: (path: string, prefix: string) =>
        path.endsWith('.wasm') ? wasmUrl : prefix + path,
    },
  })
  return new BarcodeDetector({ formats: [...SCAN_FORMATS] })
}
