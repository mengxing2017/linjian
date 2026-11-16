import { DatePicker as AntDatePicker, type DatePickerProps } from 'antd'
import zhCN from 'antd/locale/zh_CN'
import dayjs, { type Dayjs } from 'dayjs'
import { useState } from 'react'
import 'dayjs/locale/zh-cn'
import { navigationSymbols } from './navigation'

type SingleDatePickerProps = DatePickerProps<Dayjs, false>

export function DatePicker(props: SingleDatePickerProps) {
  return <AntDatePicker
    locale={zhCN.DatePicker}
    {...props}
    prevIcon={navigationSymbols.previous}
    nextIcon={navigationSymbols.next}
    superPrevIcon={navigationSymbols.previous}
    superNextIcon={navigationSymbols.next}
  />
}

// Preserve datetime-local's local wall time and minute precision at call sites.
export function LocalDateTimePicker({ value, onChange, ...props }: Omit<SingleDatePickerProps, 'value' | 'onChange' | 'multiple'> & {
  value?: string
  onChange: (value: string) => void
}) {
  const [draft, setDraft] = useState('')
  const current = value ?? draft
  return <DatePicker {...props} showTime={{ format: 'HH:mm' }} format="YYYY-MM-DD HH:mm"
    value={current ? dayjs(current) : null}
    onChange={date => {
      const next = date ? date.format('YYYY-MM-DDTHH:mm') : ''
      setDraft(next)
      onChange(next)
    }}
  />
}
