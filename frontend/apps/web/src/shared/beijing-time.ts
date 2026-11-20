import dayjs from 'dayjs'
import utc from 'dayjs/plugin/utc'

dayjs.extend(utc)

const UTC_OFFSET_MINUTES = 480

export function toBeijingInput(value?: string): string {
  const instant = dayjs(value)
  return instant.isValid() ? instant.utcOffset(UTC_OFFSET_MINUTES).format('YYYY-MM-DDTHH:mm') : ''
}

export function fromBeijingInput(value: string): string {
  return value ? dayjs(`${value}+08:00`).toISOString() : ''
}
