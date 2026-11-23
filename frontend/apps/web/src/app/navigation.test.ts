import { rolePermissions } from '../test/permissions'
import { expect, it } from 'vitest'
import { isValidElement } from 'react'
import { navigation, navigationDomains, visibleNavigationKeys } from './navigation'

it('gives every navigation destination a distinct icon', () => {
  const icons = navigation.map((item) => {
    expect(isValidElement(item.icon)).toBe(true)
    return isValidElement(item.icon) ? item.icon.type : undefined
  })
  expect(new Set(icons).size).toBe(navigation.length)
})
it('exposes complete forum management without sending capabilities', () => {
  expect(navigation.map((v) => v.key)).toEqual(
    expect.arrayContaining([
      'posts',
      'cases',
      'resources',
      'platform-resources',
      'members',
      'settings',
      'schools',
      'accounts',
      'site-settings',
    ]),
  )
  expect(
    Object.fromEntries(
      navigationDomains.map(({ key }) => [
        key,
        navigation.filter((v) => v.domain === key).map((v) => v.key),
      ]),
    ),
  ).toEqual({
    statistics: ['statistics', 'statistics-content', 'statistics-registration', 'statistics-activity', 'statistics-schools'],
    forum: ['posts', 'home-menu', 'cases', 'resources', 'platform-resources', 'blocked-words', 'platform-blocked-words'],
    operations: ['group-qrcodes','home-banners','home-shortcuts','service-targets','operation-reports','marketing-slots','school-marketing','customer-config','feedback'],
    school: ['message-advertisements', 'message-advertisement-reviews', 'schools', 'miniapp-users', 'school-changes', 'members', 'settings'],
    source: ['source-connections', 'source-bindings', 'source-outbox'],
    administration: ['administrators', 'accounts', 'site-settings', 'miniapp-registration-settings', 'audit'],
  })
  expect(navigation.some((item) => item.key === 'comments')).toBe(false)
  expect(navigation.map((v) => v.key)).not.toEqual(
    expect.arrayContaining(['mobile-execution-tasks', 'broadcast-tasks']),
  )
})
it('keeps platform account and school management separate from school operators', () => {
  expect(visibleNavigationKeys(false, 'operator', rolePermissions('operator'))).toContain('posts')
  expect(visibleNavigationKeys(false, 'operator', rolePermissions('operator'))).toContain('group-qrcodes')
  expect(visibleNavigationKeys(false, 'operator', rolePermissions('operator'))).not.toContain('customer-config')
  expect(visibleNavigationKeys(true)).toContain('customer-config')
  expect(visibleNavigationKeys(false, 'operator', rolePermissions('operator'))).not.toContain('accounts')
  expect(visibleNavigationKeys(false, 'admin', rolePermissions('admin'))).toContain('members')
  expect(visibleNavigationKeys(false, 'admin', rolePermissions('admin'))).not.toContain('schools')
  expect(visibleNavigationKeys(false, 'member')).toEqual([])
  expect(visibleNavigationKeys(true)).toContain('accounts')
  expect(visibleNavigationKeys(true)).toContain('miniapp-registration-settings')
  expect(visibleNavigationKeys(false, 'admin', rolePermissions('admin'))).not.toContain('miniapp-registration-settings')
  expect(navigation.find((item) => item.key === 'accounts')?.label).toBe('后台账号')
})

it('places separate statistics navigation before forum and limits school comparison to platform admins', () => {
  expect(navigationDomains[0].key).toBe('statistics')
  expect(navigation.find((n) => n.key === 'statistics')?.label).toBe('统计概览')
  expect(visibleNavigationKeys(false, 'operator', rolePermissions('operator'))).toContain('statistics-activity')
  expect(visibleNavigationKeys(false, 'admin', rolePermissions('admin'))).not.toContain('statistics-schools')
})

it('keeps school routing and audits platform-only even with arbitrary school permission keys', () => {
  const permissions = [...rolePermissions('admin'), 'source.binding.read', 'audit.school.read']
  for (const role of ['admin', 'operator']) {
    const visible = visibleNavigationKeys(false, role, permissions)
    expect(visible).not.toContain('source-bindings')
    expect(visible).not.toContain('audit')
  }
  expect(visibleNavigationKeys(true)).toEqual(expect.arrayContaining(['source-bindings', 'audit']))
})

it('separates school message advertisement management from platform review and banner displays', () => {
  expect(visibleNavigationKeys(false, 'admin', rolePermissions('admin'))).toContain('message-advertisements')
  expect(visibleNavigationKeys(false, 'operator', rolePermissions('admin'))).not.toContain('message-advertisements')
  expect(visibleNavigationKeys(false, 'admin', rolePermissions('admin'))).not.toContain('message-advertisement-reviews')
  expect(visibleNavigationKeys(true)).toContain('message-advertisement-reviews')
})
