import { schoolsApi } from './api'
import { onScopeChange, scopeKey } from './context'
const files = new Map<string, Promise<string>>(),
  paths = new Set<string>()
let serial = 0
function unlink(path: string) {
  uni.getFileSystemManager().unlink({ filePath: path, fail: () => {} })
}
onScopeChange(() => {
  files.clear()
  for (const path of paths) unlink(path)
  paths.clear()
})
export function brandingFile(school: string, slot: 'logo' | 'header', revision: number) {
  const scope = scopeKey(),
    key = `${scope}:${school}:${revision}:${slot}`
  if (files.has(key)) return files.get(key)!
  const pending = (async () => {
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
    paths.add(path)
    return path
  })().catch((e) => {
    files.delete(key)
    throw e
  })
  files.set(key, pending)
  return pending
}
export const defaultTheme = {
  theme_primary: '#6A5AF9',
  theme_secondary: '#8F6BFF',
  theme_text_color: '#FFFFFF',
}
export function themeColor(value: unknown, fallback: string) {
  return typeof value === 'string' && /^#[\da-fA-F]{6}$/.test(value) ? value : fallback
}
