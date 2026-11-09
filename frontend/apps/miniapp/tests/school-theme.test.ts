import { expect, it, vi } from 'vitest'
vi.mock('../src/features/school-context/context', () => ({
  schoolContext: { schools: [], id: '' },
  onScopeChange: () => () => {},
  scopeKey: () => '',
}))
vi.mock('../src/features/school-context/api', () => ({ schoolsApi: { branding: vi.fn() } }))
import { themeColor, defaultTheme } from '../src/features/school-context/branding'
it('uses a single theme default and rejects arbitrary CSS values', () => {
  expect(defaultTheme).toEqual({
    theme_primary: '#6A5AF9',
    theme_secondary: '#8F6BFF',
    theme_text_color: '#FFFFFF',
  })
  expect(themeColor('#AABBCC', defaultTheme.theme_primary)).toBe('#AABBCC')
  for (const value of ['url(https://evil.test)', 'var(--x)', 'red', '#aabbcc00', undefined])
    expect(themeColor(value, defaultTheme.theme_primary)).toBe('#6A5AF9')
})
