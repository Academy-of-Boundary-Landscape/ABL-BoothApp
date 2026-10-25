import { beforeEach, describe, expect, it, vi } from 'vitest'
import { getPendingQuickSale, recordQuickSale, retryPendingQuickSale } from './quickSale'
import { cents } from '@/utils/money'

const mocks = vi.hoisted(() => ({ post: vi.fn() }))
vi.mock('@/api/client', () => ({
  api: { POST: mocks.post },
  unwrap: (promise: Promise<unknown>) => promise,
  errorMessage: (error: Error) => error.message,
  ApiRequestError: class extends Error {
    constructor(public status: number) {
      super('请求失败')
    }
  },
}))

beforeEach(() => {
  sessionStorage.clear()
  localStorage.clear()
  mocks.post.mockReset()
  vi.stubGlobal('crypto', { randomUUID: () => '550e8400-e29b-41d4-a716-446655440000' })
})

describe('quick sale retry', () => {
  it('keeps the same request id after an unknown network outcome and blocks another sale', async () => {
    mocks.post.mockRejectedValueOnce(new Error('网络中断'))
    await expect(
      recordQuickSale(3, [{ product_id: 7, quantity: 2 }], '微信', cents(5000))
    ).rejects.toThrow('网络中断')
    expect(getPendingQuickSale()?.items).toEqual([{ product_id: 7, quantity: 2 }])
    expect(localStorage.getItem('vendor_quick_sale_pending_v1')).toContain('550e8400')
    await expect(
      recordQuickSale(3, [{ product_id: 8, quantity: 1 }], '微信', cents(2000))
    ).rejects.toThrow('上一笔')

    mocks.post.mockResolvedValueOnce({ id: 42, status: 'completed' })
    await expect(retryPendingQuickSale()).resolves.toMatchObject({ id: 42 })
    expect(getPendingQuickSale()).toBeNull()
    expect(mocks.post.mock.calls[0][1].body.request_id).toBe(
      mocks.post.mock.calls[1][1].body.request_id
    )
    expect(mocks.post.mock.calls[0][1].body.expected_amount).toBe(5000)
    expect(mocks.post.mock.calls[0][1].body.items).toEqual([{ product_id: 7, quantity: 2 }])
  })

  it('does not report a cancelled idempotent order as a new sale', async () => {
    mocks.post.mockResolvedValueOnce({ id: 42, status: 'cancelled' })
    await expect(
      recordQuickSale(3, [{ product_id: 7, quantity: 1 }], '微信', cents(3000))
    ).rejects.toThrow('已撤销')
    expect(getPendingQuickSale()).toBeNull()
  })

  it('sends the quoted amount and the actual received amount separately', async () => {
    mocks.post.mockResolvedValueOnce({ id: 43, status: 'completed' })
    await recordQuickSale(3, [{ product_id: 7, quantity: 1 }], '微信', cents(3000), cents(2800))
    expect(mocks.post.mock.calls[0][1].body).toMatchObject({
      expected_amount: 3000,
      final_amount: 2800,
    })
  })
})
