import axios from 'axios'
import { fetch } from '@tauri-apps/plugin-http'

import router from '@/router'
import {
  IMAGE_UPLOAD_LIMIT_MB,
  SYNC_IMPORT_LIMIT_MB,
  normalizeUploadError,
  showUploadDialog,
} from '@/utils/upload'

const isTauri = window.__TAURI_INTERNALS__ !== undefined
const API_PORT = 5140

const baseURL = isTauri ? `http://127.0.0.1:${API_PORT}/api` : '/api'

console.log('%c[Config] Environment Init', 'background: #333; color: #bada55')
console.log(`[Config] isTauri: ${isTauri}`)
console.log(`[Config] BaseURL: ${baseURL}`)

const tauriAdapter = async (config) => {
  const reqId = Math.floor(Math.random() * 10000)
  const startTime = performance.now()

  const basePath = config.baseURL || ''
  const requestPath = config.url || ''
  const fullUrl = requestPath.startsWith('http')
    ? requestPath
    : `${basePath.replace(/\/$/, '')}/${requestPath.replace(/^\//, '')}`

  try {
    const headers = new Headers()
    const axiosHeaders = config.headers

    if (axiosHeaders) {
      const headersObj =
        typeof axiosHeaders.toJSON === 'function' ? axiosHeaders.toJSON() : axiosHeaders

      for (const [key, val] of Object.entries(headersObj)) {
        if (val !== undefined && val !== null) {
          if (key.toLowerCase() === 'content-length') continue
          if (key.toLowerCase() === 'host') continue
          headers.set(key, String(val))
        }
      }
    }

    let body
    if (config.data) {
      if (typeof config.data === 'string') {
        body = config.data
        if (!headers.has('Content-Type')) headers.set('Content-Type', 'text/plain')
      } else if (config.data instanceof FormData) {
        headers.delete('Content-Type')
        body = config.data
      } else {
        body = JSON.stringify(config.data)
        headers.set('Content-Type', 'application/json')
      }
    }

    const response = await fetch(fullUrl, {
      method: config.method?.toUpperCase(),
      headers,
      body,
    })

    const responseType = config.responseType || 'json'
    let responseData

    if (responseType === 'arraybuffer') {
      responseData = await response.arrayBuffer()
    } else if (responseType === 'blob') {
      responseData = await response.blob()
    } else {
      const rawText = await response.text()
      try {
        responseData = responseType === 'json' || !responseType ? JSON.parse(rawText) : rawText
      } catch {
        responseData = rawText
      }
    }

    const duration = (performance.now() - startTime).toFixed(2)
    console.log(
      `[Req #${reqId}] ${config.method?.toUpperCase()} ${fullUrl} -> ${response.status} (${duration}ms)`
    )

    const axiosResponse = {
      data: responseData,
      status: response.status,
      statusText: response.statusText,
      headers: Object.fromEntries(response.headers.entries()),
      config,
      request: null,
    }

    // 模拟 axios 默认行为：非 2xx 状态码 reject
    if (response.status < 200 || response.status >= 300) {
      const error = new Error(`Request failed with status code ${response.status}`)
      error.response = axiosResponse
      error.config = config
      error.code = response.status >= 500 ? 'ERR_BAD_RESPONSE' : 'ERR_BAD_REQUEST'
      throw error
    }

    return axiosResponse
  } catch (error) {
    console.error(`[Req #${reqId}] fatal error`, error)
    throw error
  }
}

const apiClient = axios.create({
  baseURL,
  timeout: 30000,
  adapter: isTauri ? tauriAdapter : undefined,
  headers: {
    Accept: 'application/json',
  },
})

apiClient.interceptors.request.use(
  (config) => {
    const token = sessionStorage.getItem('access_token')
    if (token) {
      config.headers.Authorization = `Bearer ${token}`
    }
    return config
  },
  (error) => Promise.reject(error)
)

apiClient.interceptors.response.use(
  (response) => response,
  (error) => {
    const status = error.response?.status || 0
    console.error(
      `[Axios Error] Status: ${status} | Code: ${error.code} | Message: ${error.message}`
    )

    const requestUrl = String(error.config?.url || '')
    const isFormUpload = error.config?.data instanceof FormData
    if (isFormUpload) {
      if (requestUrl.includes('/sync/import-products')) {
        showUploadDialog('商品包导入失败', normalizeUploadError(error, SYNC_IMPORT_LIMIT_MB))
      } else if (requestUrl.includes('/events')) {
        showUploadDialog('付款二维码上传失败', normalizeUploadError(error, IMAGE_UPLOAD_LIMIT_MB))
      } else if (requestUrl.includes('/master-products')) {
        showUploadDialog('商品预览图上传失败', normalizeUploadError(error, IMAGE_UPLOAD_LIMIT_MB))
      }
    }

    if (status === 401 || status === 403) {
      const path = router.currentRoute.value.path
      if (path !== '/login') {
        router.push('/login').catch(() => {})
      }
    }

    return Promise.reject(error)
  }
)

export default apiClient
