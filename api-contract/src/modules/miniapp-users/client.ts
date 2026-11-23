import type { Cancellation, Client } from '../../http/client'
import { userQuery } from './query'
import type { MiniappUser, UserFilters, PhoneBinding, BindPhoneInput } from './types'
const part = encodeURIComponent
export function createMiniappUsersClient(
  client: Pick<Client, 'requestResource' | 'requestCollection' | 'requestBinary'>,
) {
  const base = (school?: string) =>
    school ? `/api/v1/schools/${part(school)}/miniapp-users` : '/api/v1/platform/miniapp-users'
  return {
    list(school: string | undefined, filters: UserFilters = {}, signal?: Cancellation) {
      if (filters.mobile) {
        const { page, ...attributes } = filters
        return client.requestCollection<MiniappUser>(base(school) + '/searches', signal, 'POST', {
          'page[number]': page ?? 1,
          'page[size]': 30,
          ...attributes,
        })
      }
      return client.requestCollection<MiniappUser>(base(school) + '?' + userQuery(filters), signal)
    },
    detail: (school: string | undefined, id: string, signal?: Cancellation) =>
      client.requestResource<MiniappUser>(`${base(school)}/${part(id)}`, 'GET', undefined, signal),
    status: (
      school: string | undefined,
      user: MiniappUser,
      status: 'normal' | 'banned',
      reason: string,
    ) =>
      client.requestResource<null>(`${base(school)}/${part(user.id)}/status`, 'PUT', {
        status,
        reason,
        revision: user.revision,
      }),
    correctMobile: (user: MiniappUser, mobile: string, reason: string) =>
      client.requestResource<null>(`${base()}/${part(user.id)}/mobile`, 'PUT', {
        mobile,
        reason,
        revision: user.revision,
      }),
    correctSchool: (user: MiniappUser, school_id: string, reason: string) =>
      client.requestResource<null>(`${base()}/${part(user.id)}/school`, 'PUT', {
        school_id,
        reason,
        revision: user.revision,
      }),
    profile: () => client.requestResource<MiniappUser>('/api/v1/miniapp/profile'),
    phoneBinding: () => client.requestResource<PhoneBinding>('/api/v1/miniapp/profile/phone-binding'),
    bindPhone: (input: BindPhoneInput) => client.requestResource<PhoneBinding>('/api/v1/miniapp/profile/phone-bindings', 'POST', input),
    revokePhone: (revision: number) => client.requestResource<PhoneBinding>(`/api/v1/miniapp/profile/phone-binding?revision=${revision}`, 'DELETE'),
    saveProfile: (nickname: string, avatar_asset_id: string | null, revision: number) =>
      client.requestResource<MiniappUser>('/api/v1/miniapp/profile', 'PUT', {
        nickname,
        avatar_asset_id,
        revision,
      }),
    registerMobile: (mobile: string, revision: number) =>
      client.requestResource<MiniappUser>('/api/v1/miniapp/profile/mobile', 'PUT', {
        mobile,
        revision,
      }),
    uploadAvatar: (bytes: ArrayBuffer) =>
      client.requestResource<{ id: string; url: string }>(
        '/api/v1/miniapp/avatar-assets',
        'POST',
        bytes,
      ),
    avatar: (path: string, signal?: Cancellation) => {
      if (
        !/^\/api\/v1\/miniapp\/avatar-assets\/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(
          path,
        )
      )
        throw new Error('头像地址无效')
      return client.requestBinary(path, signal)
    },
  }
}
