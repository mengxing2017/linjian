import { miniappUserApi } from './api'
import { onScopeChange, scopeKey } from '../school-context/context'
import { createFileCache } from '../../shared/file-cache'
let serial = 0
function unlink(path: string) {
  uni.getFileSystemManager().unlink({ filePath: path, fail: () => {} })
}
const files = createFileCache(unlink, 24, 16 * 1024 * 1024)
onScopeChange(files.clear)
export const releaseAvatars = files.release
export async function uploadAvatar(path: string) {
  const scope = scopeKey()
  const bytes = await new Promise<ArrayBuffer>((resolve, reject) =>
    uni.getFileSystemManager().readFile({
      filePath: path,
      success: (r) =>
        typeof r.data === 'string' ? reject(new Error('图片读取失败')) : resolve(r.data),
      fail: reject,
    }),
  )
  if (bytes.byteLength > 5 * 1024 * 1024) throw new Error('头像不能超过 5 MiB')
  if (scope !== scopeKey()) throw new Error('登录状态已改变')
  return miniappUserApi.uploadAvatar(bytes)
}
export function avatarFile(url: string, owner?: object) {
  const scope = scopeKey()
  return files.read(`${scope}:${url}`, async () => {
    const bytes = await miniappUserApi.avatar(url)
    if (scope !== scopeKey()) throw new Error('登录状态已改变')
    const path = `${(uni as typeof uni & { env: { USER_DATA_PATH: string } }).env.USER_DATA_PATH}/linjian-avatar-${Date.now()}-${serial++}.img`
    await new Promise<void>((resolve, reject) =>
      uni
        .getFileSystemManager()
        .writeFile({ filePath: path, data: bytes, success: () => resolve(), fail: reject }),
    )
    if (scope !== scopeKey()) {
      unlink(path)
      throw new Error('登录状态已改变')
    }
    return { path, bytes: bytes.byteLength }
  }, owner)
}
