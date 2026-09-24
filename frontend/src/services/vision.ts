import { api, unwrap, type Schemas } from '@/api/client'

/**
 * 查询 Vision 运行时状态
 */
export function getVisionStatus(): Promise<Schemas['VisionStatusResponse']> {
  return unwrap<Schemas['VisionStatusResponse']>(api.GET('/vision/status'))
}

/**
 * 以图搜图
 * @param imageFile - 查询图片
 * @param opts.topK - 返回前 K 个结果
 * @param opts.mode - "order" | "admin_event" | "admin_master"
 * @param opts.eventId - mode 为 order/admin_event 时必填
 * @param opts.masterProductIds - 限定搜索范围的商品 ID 列表
 * @param opts.roi - 感兴趣区域
 */
export function searchByImage(
  imageFile: File | Blob,
  opts: {
    topK?: number
    mode?: string
    eventId?: number
    masterProductIds?: number[]
    roi?: { x: number; y: number; w: number; h: number }
  } = {}
): Promise<Schemas['VisionSearchResponse']> {
  const fd = new FormData()
  fd.append('image', imageFile)
  fd.append('top_k', String(opts.topK ?? 5))

  if (opts.mode) fd.append('mode', opts.mode)
  if (opts.eventId != null) fd.append('event_id', String(opts.eventId))
  if (opts.masterProductIds?.length) {
    fd.append('master_product_ids', JSON.stringify(opts.masterProductIds))
  }
  if (opts.roi) fd.append('roi', JSON.stringify(opts.roi))

  return unwrap<Schemas['VisionSearchResponse']>(
    api.POST('/vision/search', {
      // 旧 axios 的 per-request timeout: 15000——用调用方 signal 保留，与全局 30s 取先到者。
      signal: AbortSignal.timeout(15_000),
      body: fd as never, // multipart
    })
  )
}

/**
 * 触发索引重建
 */
export function rebuildIndex(forceFull = false): Promise<Schemas['VisionRebuildResponse']> {
  return unwrap<Schemas['VisionRebuildResponse']>(
    api.POST('/vision/rebuild', { body: { force_full: forceFull } })
  )
}

/**
 * 获取可用模型列表
 */
export function listModels(): Promise<Schemas['VisionModelsResponse']> {
  return unwrap<Schemas['VisionModelsResponse']>(api.GET('/vision/models'))
}

/**
 * 安装模型
 */
export function installModel(
  modelId: string,
  source?: string
): Promise<Schemas['VisionInstallModelResponse']> {
  return unwrap<Schemas['VisionInstallModelResponse']>(
    api.POST('/vision/models/install', { body: { model_id: modelId, source } })
  )
}

/**
 * 轮询模型安装进度
 */
export function getInstallTask(taskId: string): Promise<Schemas['VisionInstallTaskResponse']> {
  return unwrap<Schemas['VisionInstallTaskResponse']>(
    api.GET('/vision/models/tasks/{task_id}', { params: { path: { task_id: taskId } } })
  )
}

/**
 * 激活模型
 */
export function activateModel(modelId: string): Promise<Schemas['VisionActivateModelResponse']> {
  return unwrap<Schemas['VisionActivateModelResponse']>(
    api.POST('/vision/models/activate', { body: { model_id: modelId } })
  )
}

// ==================== 商品视觉图管理 ====================

/**
 * 列出商品的所有视觉图
 */
export function listProductImages(
  masterProductId: number
): Promise<Schemas['MasterProductImageDto'][]> {
  return unwrap<Schemas['MasterProductImageDto'][]>(
    api.GET('/master-products/{id}/images', { params: { path: { id: masterProductId } } })
  )
}

/**
 * 为商品添加视觉图
 */
export function addProductImage(
  masterProductId: number,
  imageFile: File,
  kind = 'gallery'
): Promise<Schemas['MasterProductImageResponse']> {
  const fd = new FormData()
  fd.append('image', imageFile)
  fd.append('kind', kind)
  return unwrap<Schemas['MasterProductImageResponse']>(
    api.POST('/master-products/{id}/images', {
      params: { path: { id: masterProductId } },
      body: fd as never, // multipart
    })
  )
}

/**
 * 删除商品视觉图
 */
export function deleteProductImage(
  masterProductId: number,
  imageId: number
): Promise<Schemas['DeleteMasterProductImageResponse']> {
  return unwrap<Schemas['DeleteMasterProductImageResponse']>(
    api.DELETE('/master-products/{id}/images/{image_id}', {
      params: { path: { id: masterProductId, image_id: imageId } },
    })
  )
}
