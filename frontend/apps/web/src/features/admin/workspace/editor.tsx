import { DatePicker } from '../../../shared/date-picker'
import dayjs from 'dayjs'
import { parseDate } from '../../../shared/format-date'
import { useId, useState } from 'react'
import { Button, ColorPicker, Form, Input, InputNumber, Select } from 'antd'
import { FormFeedback } from '../../../shared/form-feedback'

interface Field {
  name: string
  title: string
  type?: string
  options?: { value: string; label: string }[]
  optional?: boolean
  readOnly?: boolean
  value?: string | number
  wide?: boolean
}

export function Editor({
  fields,
  onSave,
  submit = '保存',
  columns = false,
}: {
  fields: Field[]
  onSave: (v: Record<string, string>) => Promise<unknown>
  submit?: string
  columns?: boolean
}) {
  const formName = useId()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [done, setDone] = useState(false)
  async function save(values: Record<string, string>) {
    setBusy(true)
    setError('')
    setDone(false)
    try {
      await onSave(
        Object.fromEntries(fields.map((field) => [field.name, String(values[field.name] ?? '')])),
      )
      setDone(true)
    } catch (error) {
      setError((error as Error).message)
    } finally {
      setBusy(false)
    }
  }
  return (
    <Form
      name={formName}
      layout="vertical"
      onFinish={save}
      className={columns ? 'business-form form-grid' : 'business-form'}
      disabled={busy}
      initialValues={Object.fromEntries(fields.map((field) => [field.name, field.value]))}
    >
      {fields.map((field) => (
        <Form.Item
          key={field.name}
          name={field.name}
          label={field.title}
          getValueProps={
            field.type === 'datetime'
              ? (value) => ({ value: parseDate(value) ? dayjs(parseDate(value)) : null })
              : undefined
          }
          getValueFromEvent={
            field.type === 'datetime'
              ? (date) => date?.toISOString() ?? ''
              : field.type === 'color'
                ? (color) => color.toHexString()
                : undefined
          }
          className={field.wide || field.type === 'textarea' ? 'form-wide' : undefined}
          rules={[{ required: !field.optional, message: `请填写${field.title}` }]}
        >
          {field.options ? (
            <Select placeholder="请选择" options={field.options} aria-label={field.title} style={{ width: '100%' }} popupMatchSelectWidth={false} />
          ) : field.type === 'datetime' ? (
            <DatePicker showTime format="YYYY/M/D HH:mm:ss" className="full-width" />
          ) : field.type === 'color' ? (
            <ColorPicker showText disabledAlpha />
          ) : field.type === 'number' ? (
            <InputNumber className="full-width" />
          ) : field.type === 'textarea' ? (
            <Input.TextArea readOnly={field.readOnly} rows={4} />
          ) : field.type === 'password' ? (
            <Input.Password readOnly={field.readOnly} />
          ) : (
            <Input readOnly={field.readOnly} type={field.type ?? 'text'} />
          )}
        </Form.Item>
      ))}
      <div className="form-actions form-actions-sticky">
        <FormFeedback
          feedback={
            error
              ? { type: 'error', message: error }
              : done
                ? { type: 'success', message: '已保存' }
                : null
          }
        />
        <Button type="primary" htmlType="submit" loading={busy}>
          {busy ? '正在保存…' : submit}
        </Button>
      </div>
    </Form>
  )
}
