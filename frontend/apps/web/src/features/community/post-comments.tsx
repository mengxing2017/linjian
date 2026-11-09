import { Button, Form, Input, type FormInstance } from 'antd'
import { Field } from '../../shared/field'
import { PaginationControls } from '../../shared/pagination-controls'
import { label } from '../../shared/label'
import { useDialogs } from '../../shared/use-dialogs'
import { workspace, type Row, type User } from './api'
import type { CommunityAction } from './actions'

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
}) {
  const dialogs = useDialogs()
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
            run(async () => {
              await workspace.mutate(s, `posts/${postId}/comments`, { content })
              commentForm.resetFields()
            })
          }
        >
          <Field name="content" required label="评论内容">
            <Input.TextArea maxLength={2000} />
          </Field>
          <Button type="primary" htmlType="submit" loading={busy}>
            提交评论
          </Button>
        </Form>
        {comments.map((c) => (
          <article key={c.id}>
            <p>
              {label(c.author_name)}：{label(c.content)}
            </p>
            <Button
              disabled={busy}
              onClick={async () => {
                const content = await dialogs.prompt('回复内容')
                if (content)
                  void run(() =>
                    workspace.mutate(s, `posts/${postId}/comments`, { content, parent_id: c.id }),
                  )
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
