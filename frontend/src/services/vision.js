import apiClient from './api'

/**
 * 查询 Vision 运行时状态
 * @returns {Promise<{model_id, model_version, index_version, index_size, is_ready, is_rebuilding, last_rebuild_at, reason}>}
 */
export function getVisionStatus() {
  return apiClient.get('/vision/status').then((r) => r.data)
}

/**
 * 以图搜图
 * @param {File|Blob} imageFile - 查询图片
 * @param {object} opts
 * @param {number}        [opts.topK=5]              - 返回前 K 个结果
 * @param {string}        [opts.mode]                - "order" | "admin_event" | "admin_master"
 * @param {number}        [opts.eventId]             - mode 为 order/admin_event 时必填
 * @param {number[]}      [opts.masterProductIds]    - ���定搜索范围的商品 ID 列表
 * @param {{x,y,w,h}}    [opts.roi]                 - 感兴趣区域
 * @returns {Promise<{model_id, model_version, index_version, is_uncertain, results: Array<{master_product_id, product_code, name, score, thumb_url}>}>}
 */
export function searchByImage(imageFile, opts = {}) {
  const fd = new FormData()
  fd.append('image', imageFile)
  fd.append('top_k', String(opts.topK ?? 5))

  if (opts.mode) fd.append('mode', opts.mode)
  if (opts.eventId != null) fd.append('event_id', String(opts.eventId))
  if (opts.masterProductIds?.length) {
    fd.append('master_product_ids', JSON.stringify(opts.masterProductIds))
  }
  if (opts.roi) fd.append('roi', JSON.stringify(opts.roi))

  return apiClient.post('/vision/search', fd, { timeout: 15000 }).then((r) => r.data)
}

/**
 * 触发索引重建
 * @param {boolean} [forceFull=false]
 */
export function rebuildIndex(forceFull = false) {
  return apiClient.post('/vision/rebuild', { force_full: forceFull }).then((r) => r.data)
}

/**
 * 获取可用模型列表
 * @returns {Promise<{active_model_id, models: Array<{model_id, model_version, dim, installed, is_active}>}>}
 */
export function listModels() {
  return apiClient.get('/vision/models').then((r) => r.data)
}

/**
 * 安装模型
 * @param {string} modelId
 * @param {string} [source]
 * @returns {Promise<{task_id}>}
 */
export function installModel(modelId, source) {
  return apiClient.post('/vision/models/install', { model_id: modelId, source }).then((r) => r.data)
}

/**
 * 轮询模型安装进度
 * @param {string} taskId
 */
export function getInstallTask(taskId) {
  return apiClient.get(`/vision/models/tasks/${taskId}`).then((r) => r.data)
}

/**
 * 激活模型
 * @param {string} modelId
 */
export function activateModel(modelId) {
  return apiClient.post('/vision/models/activate', { model_id: modelId }).then((r) => r.data)
}

// ==================== 商品视觉图管理 ====================

/**
 * 列出商品的所有视觉图
 * @param {number} masterProductId
 * @returns {Promise<Array<{id, master_product_id, image_url, kind, created_at}>>}
 */
export function listProductImages(masterProductId) {
  return apiClient.get(`/master-products/${masterProductId}/images`).then((r) => r.data)
}

/**
 * 为商品添加视觉图
 * @param {number} masterProductId
 * @param {File} imageFile
 * @param {string} [kind='gallery']
 * @returns {Promise<{id, master_product_id, image_url, kind}>}
 */
export function addProductImage(masterProductId, imageFile, kind = 'gallery') {
  const fd = new FormData()
  fd.append('image', imageFile)
  fd.append('kind', kind)
  return apiClient.post(`/master-products/${masterProductId}/images`, fd).then((r) => r.data)
}

/**
 * 删除商品视觉图
 * @param {number} masterProductId
 * @param {number} imageId
 */
export function deleteProductImage(masterProductId, imageId) {
  return apiClient.delete(`/master-products/${masterProductId}/images/${imageId}`).then((r) => r.data)
}
