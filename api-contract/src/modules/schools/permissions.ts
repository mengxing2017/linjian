import type { Client, Cancellation } from '../../http/client'

export type PermissionEffect = 'allow' | 'deny'
export interface PermissionOverride { permission_key: string; effect: PermissionEffect }
export interface PermissionDefinition {
  id: string
  key: string
  name: string
  description: string
  group: string
  applicable_roles: string[]
  default_by_role: Record<string, boolean>
  requires: string[]
  high_risk: boolean
}
export interface PermissionCatalog {
  id: string
  catalog_version: string
  permissions: PermissionDefinition[]
  editable: boolean
  uneditable_reason: string
}
export interface MemberPermissions {
  id: string
  school_id: string
  username: string
  display_name: string
  role: string
  active: boolean
  mode: 'platform' | 'school'
  revision: string
  catalog_version: string
  overrides: PermissionOverride[]
  effective_permissions: string[]
}
export interface ManagementMember {
  id: string
  username: string
  display_name: string
  role: string
  active: boolean
  platform_admin: boolean
  revision: string
  override_count: number
  updated_at: string | null
}
export interface AuthorizationInput {
  catalog_version: string
  expected_revision: string
  role: 'admin' | 'operator'
  overrides: PermissionOverride[]
  reason: string
}
export interface AuthorizationAction {
  catalog_version: string
  expected_revision: string
  action: 'remove_management' | 'revoke'
  reason: string
}
export interface RecommendationPermission {
  catalog_version: string
  expected_revision: string
  effect: PermissionEffect | 'inherit'
  reason: string
}
export interface MemberFilters { page?: number; search?: string; role?: string; has_overrides?: boolean }
export function createSchoolPermissionsClient(client: Pick<Client, 'requestResource' | 'requestCollection'>) {
  const path = (school: string, suffix: string) => `/api/v1/schools/${encodeURIComponent(school)}/${suffix}`
  const member = (school: string, account: string, suffix: string) => path(school, `members/${encodeURIComponent(account)}/${suffix}`)
  const query = (filters: MemberFilters) => {
    const pairs: Array<[string, string]> = [['page[number]', String(filters.page ?? 1)], ['page[size]', '30']]
    if (filters.search) pairs.push(['search', filters.search])
    if (filters.role) pairs.push(['role', filters.role])
    if (filters.has_overrides !== undefined) pairs.push(['has_overrides', String(filters.has_overrides)])
    return '?' + pairs.map(([key, value]) => `${encodeURIComponent(key)}=${encodeURIComponent(value)}`).join('&')
  }
  return {
    catalog: (school: string, signal?: Cancellation) => client.requestResource<PermissionCatalog>(path(school, 'permission-catalog'), 'GET', undefined, signal),
    mine: (school: string, signal?: Cancellation) => client.requestResource<MemberPermissions>(path(school, 'permissions/me'), 'GET', undefined, signal),
    detail: (school: string, account: string, signal?: Cancellation) => client.requestResource<MemberPermissions>(member(school, account, 'permissions'), 'GET', undefined, signal),
    members: (school: string, filters: MemberFilters = {}, signal?: Cancellation) => client.requestCollection<ManagementMember>(path(school, 'members') + query(filters), signal),
    candidates: (school: string, filters: MemberFilters = {}, signal?: Cancellation) => client.requestCollection<ManagementMember>(path(school, 'member-candidates') + query(filters), signal),
    save: (school: string, account: string, input: AuthorizationInput) => client.requestResource<MemberPermissions>(member(school, account, 'authorization'), 'PUT', input),
    action: (school: string, account: string, input: AuthorizationAction) => client.requestResource<MemberPermissions>(member(school, account, 'authorization-actions'), 'POST', input),
    recommendation: (school: string, account: string, input: RecommendationPermission) => client.requestResource<MemberPermissions>(member(school, account, 'recommendation-capability'), 'PUT', input),
  }
}
