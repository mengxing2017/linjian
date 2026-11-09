import { Button, Form, Input } from 'antd'
import { useQueryClient } from '@tanstack/react-query'
import { Field } from '../../shared/field'
import { label } from '../../shared/label'
import { identity, workspace, type Row, type User } from './api'
import type { CommunityAction } from './actions'

export function ProfilePanel({
  school: s,
  user,
  follows,
  busy,
  run,
}: {
  school: string
  user: User
  follows: Row[]
  busy: boolean
  run: CommunityAction
}) {
  const cache = useQueryClient()
  return (
    <Form
      layout="vertical"
      className="business-form"
      initialValues={{ name: user.display_name }}
      disabled={busy}
      onFinish={({ name }: { name: string }) =>
        run(async () => {
          await identity.profile(name)
          await cache.invalidateQueries({ queryKey: ['session'] })
        })
      }
    >
      <Field name="name" required label="昵称">
        <Input />
      </Field>
      <Button type="primary" htmlType="submit" loading={busy} disabled={busy}>
        更新昵称
      </Button>
      <h2>我的关注</h2>
      {follows.map((f) => (
        <p key={f.id}>
          {label(f.display_name)}{' '}
          <Button
            htmlType="button"
            onClick={() => run(() => workspace.mutate(s, `follows/${f.id}`, undefined, 'DELETE'))}
          >
            取消关注
          </Button>
        </p>
      ))}
    </Form>
  )
}
