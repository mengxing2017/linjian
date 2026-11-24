import { expect, it } from 'vitest'
import { documentIcon, iconNames, iconPath } from '../src/shared/icons'

it('maps normal and selected icons without changing the navigation meaning', () => {
  expect(iconPath('home')).toBe('/static/icons/home-neutral.png')
  expect(iconPath('home', true)).toBe('/static/icons/home-primary.png')
  expect(iconPath('messages', true)).toBe('/static/icons/notice-primary.png')
  expect(iconPath('like', true)).toBe('/static/icons/appreciate-danger.png')
  expect(iconPath('publish')).toBe(iconPath('publish', true))
  expect(iconPath('collection')).toBe(iconPath('collection', true))
})

it('unknown names and prototype keys resolve to a local document icon', () => {
  for (const name of ['unknown', '../secret', 'https://example.com/icon.png', '__proto__', 'toString', 'constructor']) {
    expect(iconPath(name)).toBe('/static/icons/form-neutral.png')
  }
  for (const name of iconNames) expect(iconPath(name)).toMatch(/^\/static\/icons\/[a-z-]+\.png$/)
})

it('legal types get static icons without creating extra legal entries', () => {
  expect(documentIcon('privacy_policy')).toBe('privacy')
  expect(documentIcon('user_agreement')).toBe('application')
  expect(documentIcon('unknown')).toBe('application')
})
