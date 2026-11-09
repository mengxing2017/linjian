import { useEffect, useState } from 'react'
import { Alert, Button, ConfigProvider, Form, Input, Select } from 'antd'
import { useLocation, useParams } from 'react-router-dom'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useSchools, useSchoolDirectory } from '../school-context'
import { PaginationControls } from '../../shared/pagination-controls'
import { workspace, product, type User, type Post } from './api'
import { ResourceCard } from './resource-card'
import { PostCards } from './post-cards'
import { PostComments } from './post-comments'
import { PostEditor } from './post-editor'
import { ProfilePanel } from './profile-panel'
import { Notifications } from './notifications'
import { FeedbackForm } from './feedback-form'

export function CommunityContent({ user }: { user: User }) {
  const [commentForm] = Form.useForm()
  const params = useParams()
  const location = useLocation()
  const cache = useQueryClient()
  const schools = useSchools()
  const directory = useSchoolDirectory()
  const [selected, setSelected] = useState('')
  const school = schools.data?.find(
    (v) => v.id === (params.school || selected || user.bound_school_id || schools.data[0]?.id),
  )
  const s = school?.id ?? ''
  const [search, setSearch] = useState(''),
    [view, setView] = useState(
      new URLSearchParams(window.location.search).get('service') ?? 'feed',
    ),
    [page, setPage] = useState(1),
    [error, setError] = useState(''),
    [busy, setBusy] = useState(false),
    [images, setImages] = useState<string[]>([]),
    [editing, setEditing] = useState<Post>()
  useEffect(() => {
    setView(new URLSearchParams(location.search).get('service') ?? 'feed')
    setPage(1)
  }, [location.search, location.pathname])
  const posts = useQuery({
    queryKey: ['business', s, 'feed', search, view, page],
    queryFn: () =>
      workspace.list<Post>(
        s,
        'posts',
        `?${new URLSearchParams({ search, 'page[number]': String(page), 'page[size]': '20', mine: String(view === 'mine'), ...(['collect', 'following'].includes(view) ? { kind: view } : {}) })}`,
      ),
    enabled: !!s,
  })
  const detail = useQuery({
    queryKey: ['business', s, 'post', params.id],
    queryFn: () => workspace.getPost(s, params.id!),
    enabled: !!s && !!params.id,
  })
  const [commentPage, setCommentPage] = useState(1),
    [noticePage, setNoticePage] = useState(1)
  const comments = useQuery({
    queryKey: ['business', s, 'post-comments', params.id, commentPage],
    queryFn: () => workspace.comments(s, params.id!, commentPage),
    enabled: !!s && !!params.id,
  })
  const reactions = useQuery({
    queryKey: ['business', s, 'reactions'],
    queryFn: () => workspace.list(s, 'reactions'),
    enabled: !!s,
  })
  const resources = useQuery({
    queryKey: ['business', s, 'resources'],
    queryFn: () => product.resources.list(s),
    enabled: !!s,
  })
  const notices = useQuery({
    queryKey: ['business', s, 'notifications', noticePage],
    queryFn: () => workspace.list(s, 'notifications', `?page[number]=${noticePage}&page[size]=30`),
    enabled: !!s,
  })
  const follows = useQuery({
    queryKey: ['business', s, 'follows'],
    queryFn: () => workspace.list(s, 'follows'),
    enabled: !!s,
  })
  async function run(action: () => Promise<unknown>) {
    setBusy(true)
    setError('')
    try {
      await action()
      await cache.invalidateQueries({ queryKey: ['business'] })
    } catch (e) {
      setError((e as Error).message)
    } finally {
      setBusy(false)
    }
  }
  async function publish(values: { title: string; content: string; tags?: string }) {
    await run(async () => {
      await workspace.mutate(
        s,
        editing ? `posts/${editing.id}` : 'posts',
        {
          title: values.title,
          content: values.content,
          images,
          tags: String(values.tags ?? '')
            .split(',')
            .map((v) => v.trim())
            .filter(Boolean),
          revision: editing?.revision,
        },
        editing ? 'PUT' : 'POST',
      )
      setEditing(undefined)
      setImages([])
      setView('mine')
    })
  }
  if (!s)
    return (
      <section>
        <h1>绑定学校</h1>
        <p>绑定后不能自行换校，请确认学校。</p>
        {directory.data?.items.map((v) => (
          <Button
            key={v.id}
            disabled={busy}
            onClick={() =>
              run(async () => {
                await workspace.bind(v.id)
                await cache.invalidateQueries({ queryKey: ['schools'] })
                await cache.invalidateQueries({ queryKey: ['session'] })
              })
            }
          >
            {v.name}
          </Button>
        ))}
        {error && <Alert type="error" showIcon title={<>{error}</>} />}
      </section>
    )
  const list = params.id ? (detail.data ? [detail.data] : []) : (posts.data?.items ?? [])
  const filtered = list
    .filter(
      (p) =>
        view !== 'collect' ||
        reactions.data?.items.some((r) => r.post_id === p.id && r.kind === 'collect'),
    )
    .filter((p) => view !== 'following' || follows.data?.items.some((f) => f.id === p.author_id))
  return (
    <ConfigProvider theme={{ token: { colorPrimary: school?.theme_primary || '#6A5AF9' } }}>
      <div>
        <h1>{school?.name}</h1>
        <Select
          className="full-width"
          aria-label="学校"
          value={s}
          onChange={async (value) => {
            await cache.cancelQueries({ queryKey: ['business'] })
            cache.removeQueries({ queryKey: ['business'] })
            setSelected(value)
          }}
        >
          {schools.data?.map((v) => (
            <Select.Option key={v.id} value={v.id}>
              {v.name}
            </Select.Option>
          ))}
        </Select>
        <nav className="community-nav">
          {[
            ['feed', '发现'],
            ['mine', '我的发布'],
            ['collect', '收藏'],
            ['following', '关注动态'],
            ['publish', '发布'],
            ['notifications', '消息'],
            ['resources', '校园服务'],
            ['profile', '个人信息'],
            ['feedback', '反馈'],
          ].map(([key, title]) => (
            <Button
              type={view === key ? 'primary' : 'default'}
              key={key}
              aria-current={view === key ? 'page' : undefined}
              onClick={() => {
                setView(key)
                setPage(1)
              }}
            >
              {title}
            </Button>
          ))}
        </nav>
        {(error || posts.error || detail.error) && (
          <Alert
            type="error"
            showIcon
            title={<>{error || posts.error?.message || detail.error?.message}</>}
          />
        )}
        {['feed', 'mine', 'collect', 'following'].includes(view) && (
          <>
            <Input.Search
              aria-label="搜索校园内容"
              enterButton="搜索"
              onSearch={(value) => {
                setSearch(value)
                setPage(1)
              }}
            />
            <PostCards
              school={s}
              user={user}
              posts={filtered}
              reactions={reactions.data?.items ?? []}
              follows={follows.data?.items ?? []}
              busy={busy}
              run={run}
              onEdit={(post) => {
                setEditing(post)
                setImages(post.images)
                setView('publish')
              }}
            />
            {!filtered.length && <p>暂无内容</p>}
            {!params.id && (
              <PaginationControls
                page={page}
                more={posts.data?.page?.has_more}
                busy={posts.isFetching}
                change={setPage}
              />
            )}
            {params.id && (
              <PostComments
                form={commentForm}
                school={s}
                postId={params.id}
                user={user}
                comments={comments.data?.items ?? []}
                busy={busy}
                fetching={comments.isFetching}
                page={commentPage}
                more={comments.data?.page?.has_more}
                onPage={setCommentPage}
                run={run}
              />
            )}
          </>
        )}
        {view === 'publish' && (
          <PostEditor
            school={s}
            editing={editing}
            images={images}
            setImages={setImages}
            busy={busy}
            run={run}
            publish={publish}
          />
        )}
        {view === 'notifications' && (
          <Notifications
            school={s}
            items={notices.data?.items ?? []}
            page={noticePage}
            more={notices.data?.page?.has_more}
            fetching={notices.isFetching}
            busy={busy}
            onPage={setNoticePage}
            run={run}
          />
        )}
        {['resources', 'legal'].includes(view) &&
          resources.data
            ?.filter((r) => r.kind !== 'blocked_word' && (view !== 'legal' || r.kind === 'legal'))
            .map((r) => (
              <ResourceCard
                key={r.id}
                school={s}
                item={r}
                onAction={
                  r.kind === 'banner'
                    ? () => workspace.mutate(s, `resources/${r.id}/click`)
                    : undefined
                }
              />
            ))}
        {view === 'profile' && (
          <ProfilePanel
            school={s}
            user={user}
            follows={follows.data?.items ?? []}
            busy={busy}
            run={run}
          />
        )}
        {view === 'feedback' && <FeedbackForm school={s} busy={busy} run={run} />}
      </div>
    </ConfigProvider>
  )
}
