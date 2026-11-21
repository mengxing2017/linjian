import { requestResource, setCsrf } from '../../http/browser-client'
import { part } from './paths'
import type { Row, Session } from './types'
export const identity = {
  async session() {
    const v = await requestResource<Session>('/api/v1/auth/session')
    setCsrf(v.csrf)
    return v
  },
  async login(username: string, password: string) {
    const v = await requestResource<Session>('/api/v1/auth/sessions', 'POST', {
      username,
      password,
    })
    setCsrf(v.csrf)
    return v
  },
  async logout() {
    await requestResource('/api/v1/auth/session', 'DELETE')
    setCsrf('')
  },
  profile: (display_name: string) => requestResource('/api/v1/profile', 'PUT', { display_name }),
  accounts: () => requestResource<Row[]>('/api/v1/accounts'),
  create: (body: { username: string; display_name: string; password: string }) =>
    requestResource<Row>('/api/v1/accounts', 'POST', body),
  update: (id: string, body: { display_name: string; active: boolean }) =>
    requestResource(`/api/v1/accounts/${part(id)}`, 'PUT', body),
  reset: (id: string, password: string) =>
    requestResource(`/api/v1/accounts/${part(id)}/password`, 'POST', { username: '', password }),
}
