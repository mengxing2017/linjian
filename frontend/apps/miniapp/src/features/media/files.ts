import { mediaApi, type UploadPurpose, uploadKey } from './api'
import { onScopeChange, scopeKey } from '../school-context/context'
import { createFileCache } from '../../shared/file-cache'
let sequence = 0
function unlink(path: string) {
  uni.getFileSystemManager().unlink({ filePath: path, fail: () => {} })
}
const files = createFileCache(unlink)
onScopeChange(files.clear)
export const releaseImages = files.release
export async function uploadImage(school: string, filePath: string, purpose: UploadPurpose = "post_image", key = uploadKey()) {
  const bytes = await new Promise<ArrayBuffer>((resolve, reject) =>
    uni.getFileSystemManager().readFile({
      filePath,
      success: (r) =>
        typeof r.data === 'string' ? reject(new Error('图片读取格式错误')) : resolve(r.data),
      fail: reject,
    }),
  )
  if (bytes.byteLength > 5 * 1024 * 1024) throw new Error('图片不能超过 5 MiB')
  return mediaApi.upload(school, bytes, purpose, key)
}
export function imagePath(school: string, source: string, owner?: object): Promise<string> {
  // Public external images carry no Authorization header.
  if (/^https:\/\//i.test(source)) return Promise.resolve(source)
  const key = `${scopeKey()}:${school}:${source}`
  const scope = scopeKey()
  return files.read(key, async () => {
    const bytes = await mediaApi.download(school, source)
    if (scope !== scopeKey()) throw new Error('学校或登录状态已改变')
    const path = `${(uni as typeof uni & { env: { USER_DATA_PATH: string } }).env.USER_DATA_PATH}/linjian-${Date.now()}-${sequence++}.img`
    await new Promise<void>((resolve, reject) =>
      uni
        .getFileSystemManager()
        .writeFile({ filePath: path, data: bytes, success: () => resolve(), fail: reject }),
    )
    if (scope !== scopeKey()) {
      unlink(path)
      throw new Error('学校或登录状态已改变')
    }
    return { path, bytes: bytes.byteLength }
  }, owner)
}

export async function keepDraftImages(school: string, draft: string, sources: string[]) {
  return mediaApi.lease(school, draft, sources.map(v => v.slice(v.lastIndexOf('/') + 1)))
}
