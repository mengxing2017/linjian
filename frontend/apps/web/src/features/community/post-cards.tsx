import { Button, Card } from 'antd'
import { Link } from 'react-router-dom'
import { label } from '../../shared/label'
import { useDialogs } from '../../shared/use-dialogs'
import { workspace, type Post, type Row, type User } from './api'
import type { CommunityAction } from './actions'

export function PostCards({
  school: s,
  user,
  posts,
  reactions,
  follows,
  busy,
  run,
  onEdit,
}: {
  school: string
  user: User
  posts: Post[]
  reactions: Row[]
  follows: Row[]
  busy: boolean
  run: CommunityAction
  onEdit: (post: Post) => void
}) {
  const dialogs = useDialogs()
  return (
    <>
      {dialogs.contextHolder}
      {posts.map((p) => (
        <Card className="community-card" key={p.id}>
          <h2>
            <Link to={`/community/${s}/posts/${p.id}`}>{p.title}</Link>
          </h2>
          <p className="text-content">{p.content}</p>
          {p.images.map((src) => (
            <img key={src} src={src} alt="帖子图片" loading="lazy" />
          ))}
          <p>
            {label(p.author_name)} · {p.audit_status} · {p.visibility}
          </p>
          <div className="community-nav">
            {(['like', 'collect'] as const).map((kind) => {
              const active = reactions.some((r) => r.post_id === p.id && r.kind === kind)
              return (
                <Button
                  key={kind}
                  disabled={busy || p.audit_status !== 'approved' || p.visibility !== 'visible'}
                  onClick={() =>
                    run(() =>
                      workspace.mutate(
                        s,
                        `posts/${p.id}/reactions/${kind}`,
                        undefined,
                        active ? 'DELETE' : 'PUT',
                      ),
                    )
                  }
                >
                  {active ? '取消' : ''}
                  {kind === 'like' ? '点赞' : '收藏'}
                </Button>
              )
            })}
            {p.author_id !== user.id && (
              <Button
                disabled={busy}
                onClick={() =>
                  run(() =>
                    workspace.mutate(
                      s,
                      `follows/${p.author_id}`,
                      undefined,
                      follows.some((f) => f.id === p.author_id) ? 'DELETE' : 'PUT',
                    ),
                  )
                }
              >
                关注 / 取消关注作者
              </Button>
            )}
            <Button
              disabled={busy}
              onClick={async () => {
                const content = await dialogs.prompt('举报原因')
                if (content)
                  void run(() =>
                    workspace.mutate(s, 'cases', { kind: 'report', post_id: p.id, content }),
                  )
              }}
            >
              举报
            </Button>
            {p.author_id === user.id && (
              <>
                <Button
                  onClick={() => {
                    onEdit(p)
                  }}
                >
                  编辑
                </Button>
                <Button
                  disabled={busy}
                  onClick={async () => {
                    if (await dialogs.confirm('确认删除这条内容？'))
                      void run(() =>
                        workspace.mutate(
                          s,
                          `posts/${p.id}/visibility`,
                          { revision: p.revision, visibility: 'deleted', top_sort: 0 },
                          'PUT',
                        ),
                      )
                  }}
                >
                  删除
                </Button>
              </>
            )}
          </div>
        </Card>
      ))}
    </>
  )
}
