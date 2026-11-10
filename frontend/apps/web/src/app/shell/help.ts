// Static help URLs contain no business identifiers or credentials.
export const pageHelp: Record<string, string> = {
  'source-connections': 'platform-admin/source-access',
  'source-bindings': 'platform-admin/source-access',

  'source-outbox': 'operator/source-outbox',
  posts: 'operator/moderation',
  'home-menu': 'operator/moderation#display-resources',
  cases: 'operator/moderation',
  resources: 'operator/moderation#display-resources',
  'blocked-words': 'operator/moderation#display-resources',
  'platform-blocked-words': 'platform-admin/audit-and-resources',
  'miniapp-users': 'school-admin/settings-and-members#miniapp-users',
  members: 'school-admin/settings-and-members',
  settings: 'school-admin/settings-and-members',
  audit: 'platform-admin/audit-and-resources#audit',
  schools: 'platform-admin/accounts-and-schools',
  accounts: 'platform-admin/accounts-and-schools',
  administrators: 'platform-admin/accounts-and-schools',
  'platform-resources': 'platform-admin/audit-and-resources',
}
export const helpHref = (active: string) =>
  pageHelp[active] ? `/docs/guide/${pageHelp[active]}` : '/docs/guide/'
