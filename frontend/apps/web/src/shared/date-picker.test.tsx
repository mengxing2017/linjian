import dayjs from 'dayjs'
import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { DatePicker, LocalDateTimePicker } from './date-picker'

afterEach(cleanup)

it('navigates across years without changing the selected date and respects date bounds', () => {
  const change = vi.fn()
  render(<DatePicker open value={dayjs('2026-12-15')} minDate={dayjs('2026-12-01')}
    maxDate={dayjs('2027-01-31')} onChange={change} />)
  const previous = screen.getByRole('button', { name: '上个月' }) as HTMLButtonElement
  const next = screen.getByRole('button', { name: '下个月' }) as HTMLButtonElement
  expect(previous.textContent).toBe('<')
  expect(previous.disabled).toBe(true)
  expect(next.textContent).toBe('>')
  fireEvent.click(next)
  expect(screen.getByRole('button', { name: '选择年份' }).textContent).toBe('2027年')
  expect(next.disabled).toBe(true)
  expect(change).not.toHaveBeenCalled()
  fireEvent.click(previous)
  expect(screen.getByRole('button', { name: '选择年份' }).textContent).toBe('2026年')
})

it('preserves local minute precision on selection and emits an empty value on clear', () => {
  const change = vi.fn()
  render(<LocalDateTimePicker aria-label="生效时间" value="2026-12-15T08:30" onChange={change} />)
  const input = screen.getByLabelText('生效时间')
  expect((input as HTMLInputElement).value).toBe('2026-12-15 08:30')
  fireEvent.mouseDown(input)
  fireEvent.focus(input)
  fireEvent.change(input, { target: { value: '2026-12-16 09:45' } })
  fireEvent.keyDown(input, { key: 'Enter', keyCode: 13 })
  expect(change).toHaveBeenCalledWith('2026-12-16T09:45')
  fireEvent.click(screen.getByRole('button', { name: '清除' }))
  expect(change).toHaveBeenLastCalledWith('')
})
