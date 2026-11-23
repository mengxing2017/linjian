import { expect, it, vi } from 'vitest'
import { uploadBatch } from '../src/features/media/upload-batch'

it('summarizes a successful batch once with upload-stage wording', async () => {
  const send = vi.fn().mockResolvedValue(true)
  expect(await uploadBatch(['a', 'b'], send, () => true)).toBe('本次已上传 2 张，提交表单后生效')
  expect(send).toHaveBeenCalledTimes(2)
})

it('counts rejected files and scopes retry counts to the retry batch', async () => {
  const send = vi.fn().mockResolvedValueOnce(true).mockResolvedValueOnce(false)
  expect(await uploadBatch(['a', 'b'], send, () => true)).toBe(
    '本次已上传 1 张，失败 1 张，提交表单后生效',
  )
  expect(await uploadBatch(['b'], vi.fn().mockResolvedValue(true), () => true)).toBe(
    '本次已上传 1 张，提交表单后生效',
  )
})

it('never claims a successful upload for an all-failed or cancelled batch', async () => {
  expect(await uploadBatch(['a'], vi.fn().mockResolvedValue(false), () => true)).toBe(
    '本次已上传 0 张，失败 1 张，请重试失败图片',
  )
  expect(await uploadBatch([], vi.fn(), () => true)).toBeUndefined()
})

it('stops stale batches without publishing a summary or uploading remaining files', async () => {
  let active = true
  const send = vi.fn(async () => {
    active = false
    return true
  })
  expect(await uploadBatch(['a', 'b'], send, () => active)).toBeUndefined()
  expect(send).toHaveBeenCalledTimes(1)
})
