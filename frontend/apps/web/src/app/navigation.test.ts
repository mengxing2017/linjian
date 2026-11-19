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
    operations: ['group-qrcodes','home-banners','customer-config','feedback'],
    school: ['schools', 'miniapp-users', 'members', 'settings'],
    source: ['source-connections', 'source-bindings', 'source-outbox'],
    administration: ['administrators', 'accounts', 'site-settings', 'audit'],
  })
  expect(navigation.some((item) => item.key === 'comments')).toBe(false)
  expect(navigation.map((v) => v.key)).not.toEqual(
    expect.arrayContaining(['mobile-execution-tasks', 'broadcast-tasks']),
  )
})
it('keeps platform account and school management separate from school operators', () => {
  expect(visibleNavigationKeys(false, 'operator')).toContain('posts')
  expect(visibleNavigationKeys(false, 'operator')).toContain('group-qrcodes')
  expect(visibleNavigationKeys(false, 'operator')).not.toContain('customer-config')
  expect(visibleNavigationKeys(true)).toContain('customer-config')
  expect(visibleNavigationKeys(false, 'operator')).not.toContain('accounts')
  expect(visibleNavigationKeys(false, 'admin')).toContain('members')
  expect(visibleNavigationKeys(false, 'admin')).not.toContain('schools')
  expect(visibleNavigationKeys(false, 'member')).toEqual([])
  expect(visibleNavigationKeys(true)).toContain('accounts')
})

it('places separate statistics navigation before forum and limits school comparison to platform admins', () => {
  expect(navigationDomains[0].key).toBe('statistics')
  expect(navigation.find((n) => n.key === 'statistics')?.label).toBe('统计概览')
  expect(visibleNavigationKeys(false, 'operator')).toContain('statistics-activity')
  expect(visibleNavigationKeys(false, 'admin')).not.toContain('statistics-schools')
})
