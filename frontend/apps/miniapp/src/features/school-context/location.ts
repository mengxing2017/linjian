export type Coordinates = { latitude: number; longitude: number; coordinate_system: 'gcj02' }
export class LocationFailure extends Error {
  constructor(public readonly reason: 'denied' | 'failed', message: string) { super(message) }
}
export function normalizeCoordinates(latitude: number, longitude: number): Coordinates {
  if (!Number.isFinite(latitude) || !Number.isFinite(longitude) || Math.abs(latitude) > 90 || Math.abs(longitude) > 180) throw new LocationFailure('failed', '定位结果无效，请重试或手动选择')
  return { latitude: Number(latitude.toFixed(6)), longitude: Number(longitude.toFixed(6)), coordinate_system: 'gcj02' }
}
export const location = {
  async privacyRequired() {
    if (typeof uni.getPrivacySetting !== 'function') throw new LocationFailure('failed', '当前微信版本不支持位置隐私授权，请升级微信或手动选择')
    return new Promise<boolean>((resolve, reject) => uni.getPrivacySetting({ success: setting => resolve(setting.needAuthorization), fail: reject }))
  },
  async authorize() {
    const setting = await uni.getSetting({})
    if (setting.authSetting['scope.userLocation'] === false) throw new LocationFailure('denied', '未授权位置，无法推荐学校；仍可手动搜索选择')
    if (!setting.authSetting['scope.userLocation']) {
      try { await uni.authorize({ scope: 'scope.userLocation' }) }
      catch { throw new LocationFailure('denied', '未授权位置，无法推荐学校；仍可手动搜索选择') }
    }
  },
  async locate(): Promise<Coordinates> {
    try {
      const value = await uni.getLocation({ type: 'gcj02', isHighAccuracy: false })
      return normalizeCoordinates(value.latitude, value.longitude)
    } catch { throw new LocationFailure('failed', '暂时无法获取位置，请检查系统和微信定位权限，或手动选择') }
  },
}
