import { computed, ref, shallowRef } from 'vue'
import { mergePageWindow } from '../../shared/page-window'
import type { communityApi, CommunityPost, Comment, PostInteractionState } from './api'

type DetailApi = Pick<typeof communityApi, 'post' | 'interactionState' | 'comments' | 'react' | 'follow' | 'comment' | 'deleteComment'>
interface Environment {
  api: DetailApi
  school: () => string
  id: () => string
  key: () => string
  invalidate: (kind?: string) => void
  failure: (error: unknown) => string
  notify: (message: string) => void
  confirm: (message: string) => Promise<boolean>
  beforePublicAction?: () => Promise<boolean>
}
export function createPostDetail(env: Environment) {
  const post = shallowRef<CommunityPost>()
  const interaction = shallowRef<PostInteractionState>()
  const comments = shallowRef<Comment[]>([])
  const content = ref('')
  const parent = shallowRef<{ id: string; authorName: string }>()
  const error = ref('')
  const loading = ref(false)
  const loadingMore = ref(false)
  const operation = ref('')
  const more = ref(false)
  const visible = ref(false)
  const validated = ref(false)
  let generation = 0
  let commentGeneration = 0
  let stateGeneration = 0
  const page = ref(1)
  let writeSequence = 0
  let disposed = false
  let fullLoad: Promise<void> | undefined
  const busy = computed(() => loading.value || !!operation.value)
  const canInteract = computed(() => visible.value && validated.value && !!interaction.value?.can_interact)
  const canShare = computed(() => visible.value && validated.value && !!interaction.value?.can_share)
  function token() {
    const turn = generation
    const scope = env.key()
    return () => !disposed && turn === generation && scope === env.key()
  }
  function failure(cause: unknown, prefix = '') {
    const current = token()
    if (!(cause && typeof cause === 'object' && 'code' in cause && cause.code === 'phone_verification_required')) validated.value = false
    const message = env.failure(cause)
    if (current()) error.value = prefix + message
  }
  function applyState(state: PostInteractionState) {
    interaction.value = state
    if (post.value) post.value = { ...post.value, audit_status: state.audit_status, visibility: state.visibility, like_count: state.like_count, comment_count: state.comment_count }
    if (state.audit_status !== 'approved' || state.visibility !== 'visible') { comments.value = []; more.value = false }
    validated.value = true
  }
  async function sync(current: () => boolean, resetComments = false) {
    const stateTurn = ++stateGeneration
    const commentsTurn = resetComments ? ++commentGeneration : commentGeneration
    if (resetComments) loadingMore.value = false
    const [state, result] = await Promise.all([
      env.api.interactionState(env.school(), env.id()),
      resetComments ? env.api.comments(env.school(), env.id(), 1) : undefined,
    ])
    if (!current() || stateTurn !== stateGeneration) return
    applyState(state)
    if (result && commentsTurn === commentGeneration) {
      comments.value = state.audit_status === 'approved' && state.visibility === 'visible' ? result.items : []
      page.value = 1
      more.value = state.audit_status === 'approved' && state.visibility === 'visible' && !!result.page?.has_more
    }
  }
  async function read() {
    if (busy.value || disposed) return
    generation++
    const stateTurn = ++stateGeneration
    const current = token()
    const commentsTurn = ++commentGeneration
    loadingMore.value = false
    loading.value = true
    validated.value = false
    error.value = ''
    try {
      const [item, result, state] = await Promise.all([
        env.api.post(env.school(), env.id()),
        env.api.comments(env.school(), env.id()),
        env.api.interactionState(env.school(), env.id()),
      ])
      if (!current() || stateTurn !== stateGeneration) return
      post.value = item
      if (commentsTurn === commentGeneration) {
        comments.value = result.items
        page.value = 1
        more.value = !!result.page?.has_more
      }
      applyState(state)
    } catch (cause) {
      if (current()) { post.value = undefined; interaction.value = undefined; comments.value = []; failure(cause) }
    } finally {
      if (current()) loading.value = false
    }
  }
  function load(): Promise<void> {
    if (fullLoad) return fullLoad
    const request = read().finally(() => { if (fullLoad === request) fullLoad = undefined })
    fullLoad = request
    return request
  }
  async function resume() {
    if (loading.value || operation.value || disposed) return
    const current = token()
    validated.value = false
    error.value = ''
    try { await sync(current) } catch (cause) {
      if (current()) {
        if (cause && typeof cause === 'object' && 'status' in cause && [401, 403, 404].includes(Number(cause.status))) { post.value = undefined; comments.value = [] }
        failure(cause)
      }
    }
  }
  async function write(kind: string, action: () => Promise<unknown>, after?: (result: unknown) => void, resetComments = false, publicAction = false) {
    if (busy.value || !canInteract.value) return
    const current = token()
    const turn = ++writeSequence
    operation.value = kind
    error.value = ''
    let committed = false
    try {
      if (publicAction && env.beforePublicAction && !await env.beforePublicAction()) return
      if (!current()) return
      const result = await action()
      if (!current()) return
      committed = true
      env.invalidate(kind)
      after?.(result)
      await sync(current, resetComments)
    } catch (cause) {
      if (current()) {
        failure(cause, committed ? '操作已完成，状态同步失败：' : '')
        if (!committed && current() && cause && typeof cause === 'object' && 'status' in cause && cause.status === 400) validated.value = !!interaction.value
        if (!committed && current() && cause && typeof cause === 'object' && 'status' in cause && [0, 404, 409].includes(Number(cause.status))) {
          env.invalidate(kind)
          if (kind === 'comment' && parent.value && cause.status === 404) error.value = '回复对象或帖子已不可用，请刷新核对并重新选择回复对象；草稿已保留'
          try { await sync(current, cause.status === 409 || cause.status === 404) } catch (syncCause) {
            if (current()) {
              validated.value = false
              if (syncCause && typeof syncCause === 'object' && 'status' in syncCause && [401, 403, 404].includes(Number(syncCause.status))) { post.value = undefined; comments.value = []; env.failure(syncCause) }
            }
          }
        }
      }
    } finally {
      if (current() && turn === writeSequence) operation.value = ''
    }
  }
  function react(kind: 'like' | 'collect') {
    const desired = !(kind === 'like' ? interaction.value?.liked : interaction.value?.collected)
    return write('reaction', () => env.api.react(env.school(), env.id(), kind, desired), undefined, false, kind === 'like' && desired)
  }
  function follow() {
    if (!post.value) return Promise.resolve()
    const author = post.value.author_id
    const desired = !interaction.value?.author_followed
    return write('follow', () => env.api.follow(env.school(), author, desired))
  }
  async function comment() {
    if (!content.value.trim()) { error.value = '请填写评论内容'; return }
    if ([...content.value.trim()].length > 2000) { error.value = '评论最多2000字'; return }
    const draft = content.value
    const reply = parent.value
    return write('comment', () => env.api.comment(env.school(), env.id(), draft.trim(), reply?.id), result => {
      if (content.value === draft && parent.value?.id === reply?.id) { content.value = ''; parent.value = undefined }
      if (visible.value) env.notify((result as Comment).audit_status === 'approved' ? '评论发布成功' : '已提交，等待审核')
    }, true, true)
  }
  async function removeComment(item: Comment) {
    if (item.school_id !== env.school() || item.post_id !== env.id()) return
    const current = token()
    if (!(await env.confirm('确认删除评论？')) || !current()) return
    return write('delete', () => env.api.deleteComment(env.school(), item), undefined, true)
  }
  async function loadComments(target: number) {
    if (loadingMore.value || busy.value || !validated.value || target < 1) return
    const current = token()
    const turn = commentGeneration
    loadingMore.value = true
    try {
      const result = await env.api.comments(env.school(), env.id(), target)
      if (!current() || turn !== commentGeneration) return
      comments.value = mergePageWindow(comments.value, page.value, result.items, target)
      page.value = target
      more.value = !!result.page?.has_more
    } catch (cause) { if (current() && turn === commentGeneration) failure(cause) } finally {
      if (current() && turn === commentGeneration) loadingMore.value = false
    }
  }
  function next() {
    return more.value ? loadComments(page.value + 1) : Promise.resolve()
  }
  function reply(item: Comment) {
    if (canInteract.value && item.school_id === env.school() && item.post_id === env.id() && item.audit_status === 'approved' && item.visibility === 'visible' && !item.deleted)
      parent.value = { id: item.id, authorName: item.author_name }
  }
  function reset() {
    fullLoad = undefined
    generation++
    commentGeneration++
    stateGeneration++
    writeSequence++
    post.value = undefined
    interaction.value = undefined
    comments.value = []
    content.value = ''
    parent.value = undefined
    loading.value = false
    loadingMore.value = false
    operation.value = ''
    validated.value = false
    more.value = false
    error.value = ''
    page.value = 1
  }
  return {
    post, interaction, comments, content, parent, error, loading, loadingMore, operation, more, busy, canInteract, canShare, page,
    load, resume, react, follow, comment, removeComment, next, loadComments, reply, reset,
    show: () => { visible.value = true; validated.value = false },
    hide: () => { visible.value = false; validated.value = false },
    dispose: () => { reset(); disposed = true; visible.value = false },
  }
}
