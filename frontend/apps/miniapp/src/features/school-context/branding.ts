import { schoolsApi } from './api'
import { onScopeChange, scopeKey } from './context'
import { createFileCache } from '../../shared/file-cache'
let serial = 0
function unlink(path: string) {
  uni.getFileSystemManager().unlink({ filePath: path, fail: () => {} })
}
const files = createFileCache(unlink, 24, 16 * 1024 * 1024)
onScopeChange(files.clear)
export const releaseBranding = files.release
export function brandingFile(school: string, slot: 'logo' | 'header', revision: number, owner?: object) {
  const scope = scopeKey(),
    key = `${scope}:${school}:${revision}:${slot}`
  return files.read(key, async () => {
    const bytes = await schoolsApi.branding(school, slot)
    if (scope !== scopeKey()) throw new Error('学校或账号已变化')
    const path = `${(uni as typeof uni & { env: { USER_DATA_PATH: string } }).env.USER_DATA_PATH}/linjian-brand-${Date.now()}-${serial++}.img`
    await new Promise<void>((resolve, reject) =>
      uni
        .getFileSystemManager()
        .writeFile({ filePath: path, data: bytes, success: () => resolve(), fail: reject }),
    )
    if (scope !== scopeKey()) {
      unlink(path)
      throw new Error('学校或账号已变化')
    }
    return { path, bytes: bytes.byteLength }
  }, owner)
}
export const defaultTheme = {
  theme_primary: '#6A5AF9',
  theme_secondary: '#8F6BFF',
  theme_text_color: '#FFFFFF',
}
export function themeColor(value: unknown, fallback: string) {
  return typeof value === 'string' && /^#[\da-fA-F]{6}$/.test(value) ? value : fallback
}
