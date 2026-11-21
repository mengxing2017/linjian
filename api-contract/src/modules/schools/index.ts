export type { School } from '../business/types'
export { createSchoolsClient } from './client'
export { schoolPath } from './path'
export { createSchoolPermissionsClient } from './permissions'
export type {
  AuthorizationAction,
  AuthorizationInput,
  ManagementMember,
  MemberFilters,
  MemberPermissions,
  PermissionCatalog,
  PermissionDefinition,
  PermissionEffect,
  PermissionOverride,
  RecommendationPermission,
} from './permissions'
export type { SchoolDisplay, SchoolFilters, SchoolInput } from './types'
