import { expect, it, vi } from 'vitest'
vi.mock('../src/features/school-context/context', () => ({ onScopeChange: vi.fn(), clearScope: vi.fn(), invalidateSchool: vi.fn() }))
import { clearScope, invalidateSchool } from '../src/features/school-context/context'
import { handlePageError } from '../src/shared/use-page'

it('does not discard the school, login or draft on a phone-verification denial', () => {
  const error = Object.assign(new Error('请先验证手机号'), { status: 403, code: 'phone_verification_required' })
  expect(handlePageError(error)).toBe('请先验证手机号')
  expect(clearScope).not.toHaveBeenCalled()
  expect(invalidateSchool).not.toHaveBeenCalled()
})
