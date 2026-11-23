import { Alert, Button, Form, Input, type FormInstance } from 'antd'
import { Field } from '../../shared/field'
import { PaginationControls } from '../../shared/pagination-controls'
import { label } from '../../shared/label'
import { useDialogs } from '../../shared/use-dialogs'
import { workspace, type Row, type User } from './api'
import type { CommunityAction } from './actions'
import { notifySuccess } from '../../shared/notify'
import { useOperationScope } from '../../shared/use-operation-scope'

export function PostComments({
  form: commentForm,
  school: s,
  postId,
  user,
  comments,
  busy,
  fetching,
  page,
  more,
  onPage,
  run,
  error,
}: {
  school: string
  postId: string
  user: User
  comments: Row[]
  form: FormInstance
  busy: boolean
  fetching: boolean
  page: number
  more?: boolean
  onPage: (page: number) => void
  run: CommunityAction
  error?: string
}) {
  const dialogs = useDialogs()
  const capture = useOperationScope(`${s}:${postId}`)
  async function comment(content: string, parent?: string) {
    const active = capture()
    const result = await workspace.mutate(s, `posts/${postId}/comments`, { content, ...(parent ? { parent_id: parent } : {}) })
    if (!active()) return
    notifySuccess(result.audit_status === 'approved' ? '评论已发布' : result.audit_status === 'pending' ? '评论已提交，等待审核' : '评论已提交')
    if (!parent) commentForm.resetFields()
  }
  return (
    <>
      {dialogs.contextHolder}
      <section>
        <h2>评论</h2>
        <Form
          form={commentForm}
          layout="vertical"
          className="business-form"
          disabled={busy}
          onFinish={({ content }: { content: string }) =>
            run(() => comment(content), { refreshFailure: '评论已提交，列表刷新失败，请刷新查看' })
          }
        >
          <Field name="content" required label="评论内容">
            <Input.TextArea maxLength={2000} />
          </Field>
          <div className="form-actions form-actions-sticky">
            {error && <Alert type="error" title={error} />}
            <Button type="primary" htmlType="submit" loading={busy}>
              提交评论
            </Button>
          </div>
        </Form>
        {comments.map((c) => (
          <article key={c.id}>
            <p>
              {label(c.author_name)}：{label(c.content)}
            </p>
            <Button
              disabled={busy}
              onClick={async () => {
                const active = capture()
                const content = await dialogs.prompt('回复内容')
                if (active() && content?.trim())
                  void run(() => comment(content, c.id), { refreshFailure: '评论已提交，列表刷新失败，请刷新查看' })
              }}
            >
              回复
            </Button>
            {c.author_id === user.id && (
              <Button
                disabled={busy}
                onClick={() =>
                  run(() =>
                    workspace.mutate(s, `comments/${c.id}`, { revision: c.revision }, 'DELETE'),
                  )
                }
              >
                删除
              </Button>
            )}
          </article>
        ))}
        <PaginationControls
          page={page}
          more={more}
          busy={fetching}
          change={onPage}
          subject="评论"
        />
      </section>
    </>
  )
}
