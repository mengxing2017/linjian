import { Alert, Button, Form, Input } from 'antd'
import { Field } from '../../shared/field'
import { workspace } from './api'
import type { CommunityAction } from './actions'
import { useState } from 'react'
import { useOperationScope } from '../../shared/use-operation-scope'

export function FeedbackForm({
  school: s,
  busy,
  run,
  error,
  onReturn,
  onContinue,
}: {
  school: string
  busy: boolean
  run: CommunityAction
  error?: string
  onReturn: () => void
  onContinue: () => void
}) {
  const [form] = Form.useForm()
  const [submitted, setSubmitted] = useState(false)
  const capture = useOperationScope(s)
  if (submitted) return <section>
    <Alert type="success" title="反馈已提交" />
    {error && <Alert type="warning" title={error} />}
    <Button disabled={busy} onClick={onReturn}>返回社区</Button>
    <Button disabled={busy} onClick={() => { onContinue(); setSubmitted(false) }}>继续反馈</Button>
  </section>
  return (
    <Form
      form={form}
      layout="vertical"
      className="business-form"
      disabled={busy}
      onFinish={({ content, contact }: { content: string; contact?: string }) =>
        run(async () => {
          const active = capture()
          await workspace.mutate(s, 'cases', { kind: 'feedback', content, contact: contact ?? '' })
          if (!active()) return
          form.resetFields()
          setSubmitted(true)
        }, { refreshFailure: '反馈已提交，列表刷新失败，请刷新查看' })
      }
    >
      <Field name="content" required label="意见反馈">
        <Input.TextArea maxLength={500} />
      </Field>
      <Field name="contact" label="联系方式">
        <Input />
      </Field>
      <div className="form-actions form-actions-sticky">
        {error && <Alert type="error" title={error} />}
        <Button type="primary" htmlType="submit" loading={busy} disabled={busy}>
          提交反馈
        </Button>
      </div>
    </Form>
  )
}
