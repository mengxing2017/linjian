import type { ReactNode } from 'react'
import { readFileSync } from 'node:fs'
import { URL as NodeURL } from 'node:url'
import { SchoolPermissionsContext } from '../features/admin/workspace/permissions-context'

const catalog = JSON.parse(readFileSync(new NodeURL('../../../../../api-contract/fixtures/school-permission-catalog.json', import.meta.url), 'utf8')) as { data: { attributes: { catalog_version: string; permissions: Array<{key: string; applicable_roles: string[]; default_by_role: Record<string, boolean>}> } } }
export const permissionCatalog = catalog.data.attributes
export function rolePermissions(role: string): string[] {
  return permissionCatalog.permissions.filter(item => item.applicable_roles.includes(role) && item.default_by_role[role as 'admin' | 'operator']).map(item => item.key)
}
export function PermissionFixture({ children, role = 'admin' }: { children: ReactNode; role?: string }) {
  return <SchoolPermissionsContext.Provider value={{ platform: false, effective: rolePermissions(role) }}>{children}</SchoolPermissionsContext.Provider>
}
