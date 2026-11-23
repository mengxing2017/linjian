import { Alert, Button, Form, Input } from 'antd'
import { useQueryClient } from '@tanstack/react-query'
import { Field } from '../../shared/field'
import { label } from '../../shared/label'
import { identity, workspace, type Row, type User } from './api'
import type { CommunityAction } from './actions'
import { notifySuccess } from '../../shared/notify'
import { useOperationScope } from '../../shared/use-operation-scope'

export function ProfilePanel({
  school: s,
  user,
  follows,
  busy,
  run,
  error,
}: {
  school: string
  user: User
  follows: Row[]
  busy: boolean
  run: CommunityAction
  error?: string
}) {
  const cache = useQueryClient()
  const capture = useOperationScope(s)
  return (
    <Form
      layout="vertical"
      className="business-form"
      initialValues={{ name: user.display_name }}
      disabled={busy}
      onFinish={({ name }: { name: string }) =>
        run(async () => {
          const active = capture()
          await identity.profile(name)
          if (!active()) return
          notifySuccess('昵称已更新')
          try {
            await cache.invalidateQueries({ queryKey: ['session'] }, { throwOnError: true })
          } catch {
            throw new Error('昵称已更新，显示信息刷新失败，请刷新查看')
          }
        }, { refreshFailure: '昵称已更新，列表刷新失败，请刷新查看' })
      }
    >
      <Field name="name" required label="昵称">
        <Input />
      </Field>
      <div className="form-actions form-actions-sticky">
        {error && <Alert type="error" title={error} />}
        <Button type="primary" htmlType="submit" loading={busy} disabled={busy}>
          更新昵称
        </Button>
      </div>
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
