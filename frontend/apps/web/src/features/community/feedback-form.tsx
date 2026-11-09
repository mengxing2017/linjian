import { Button, Form, Input } from 'antd'
import { Field } from '../../shared/field'
import { workspace } from './api'
import type { CommunityAction } from './actions'

export function FeedbackForm({
  school: s,
  busy,
  run,
}: {
  school: string
  busy: boolean
  run: CommunityAction
}) {
  return (
    <Form
      layout="vertical"
      className="business-form"
      disabled={busy}
      onFinish={({ content, contact }: { content: string; contact?: string }) =>
        run(() =>
          workspace.mutate(s, 'cases', { kind: 'feedback', content, contact: contact ?? '' }),
        )
      }
    >
      <Field name="content" required label="意见反馈">
        <Input.TextArea maxLength={500} />
      </Field>
      <Field name="contact" label="联系方式">
        <Input />
      </Field>
      <Button type="primary" htmlType="submit" loading={busy} disabled={busy}>
        提交反馈
      </Button>
    </Form>
  )
}
