import { computed, ref } from 'vue'
import { identityApi, type LegalAcceptance, type OnboardingProgress, type OnboardingReady, type OnboardingResult, type OnboardingSchool } from './api'
import { legalDocuments, type LegalSummary } from '../legal-documents/api'

export function createOnboarding(dependencies: {
  identity: Pick<typeof identityApi, 'onboarding' | 'onboardingSchools' | 'cancelOnboarding'>
  documents: Pick<typeof legalDocuments, 'list'>
  start: () => Promise<OnboardingResult>
  ready: (value: OnboardingReady) => Promise<void>
  requestKey: () => string
}) {
  const documents = ref<LegalSummary[]>([]), schools = ref<OnboardingSchool[]>([])
  const selected = ref(''), bound = ref(''), query = ref(''), page = ref(1), more = ref(false)
  const checked = ref<Record<string, number>>({})
  const progress = ref<OnboardingProgress>()
  const busy = ref(false), error = ref(''), loaded = ref(false)
  let generation = 0, key = '', fingerprint = ''
  function reconcile(rows: LegalSummary[], accepted: LegalAcceptance[] = []) {
    const next: Record<string, number> = {}
    for (const row of rows) {
      if (checked.value[row.kind] === row.revision || accepted.some(value => value.kind === row.kind && value.revision === row.revision)) next[row.kind] = row.revision
    }
    checked.value = next
    documents.value = rows
  }
  function apply(value: OnboardingProgress) {
    progress.value = value
    bound.value = value.school_id ?? ''
    if (bound.value) selected.value = bound.value
    reconcile(value.documents, value.accepted)
    loaded.value = true
  }
  const missing = computed(() => progress.value?.first_login === false ? [] : documents.value.filter(row => checked.value[row.kind] !== row.revision))
  async function search(reset: boolean, current: () => boolean) {
    const next = reset ? 1 : page.value + 1
    const rows = await dependencies.identity.onboardingSchools(query.value, next)
    if (!current()) return
    schools.value = reset ? rows : [...schools.value, ...rows]
    page.value = next; more.value = rows.length === 20
  }
  async function run(action: (current: () => boolean) => Promise<void>) {
    if (busy.value) return
    const turn = ++generation
    busy.value = true; error.value = ''
    try { await action(() => turn === generation) }
    catch (failure) {
      if (turn !== generation) return
      error.value = failure instanceof Error ? failure.message : '请求失败，请重试'
      if (failure && typeof failure === 'object' && 'status' in failure && failure.status === 401) { progress.value = undefined; checked.value = {}; key = '' }
    } finally { if (turn === generation) busy.value = false }
  }
  async function receive(value: OnboardingResult, current: () => boolean) {
    if (!current()) return false
    if (value.state === 'ready') { await dependencies.ready(value); return false }
    apply(value)
    return true
  }
  function show() {
    return run(async current => {
      const rows = await dependencies.documents.list()
      if (!current()) return
      reconcile(rows); loaded.value = true
      if (progress.value) {
        const result = await dependencies.identity.onboarding('status', { ticket: progress.value.ticket })
        if (!(await receive(result, current))) return
      }
      const rowsSchools = await dependencies.identity.onboardingSchools(query.value, 1)
      if (current()) { schools.value = rowsSchools; page.value = 1; more.value = rowsSchools.length === 20 }
    })
  }
  function submit() {
    return run(async current => {
      if (!loaded.value) throw new Error('请先重试加载协议与学校')
      if (!progress.value) {
        const result = await dependencies.start()
        if (!(await receive(result, current))) return
      }
      if (missing.value.length) throw new Error('请阅读并勾选全部已配置的协议与政策')
      const ticket = progress.value!.ticket
      const confirmation = await dependencies.identity.onboarding('confirm', { ticket, acceptances: documents.value.filter(row => checked.value[row.kind] === row.revision).map(row => ({ kind: row.kind, revision: row.revision })) })
      if (!(await receive(confirmation, current))) return
      if (progress.value?.state === 'consent_required') throw new Error('协议已更新，请重新阅读并确认')
      if (!selected.value) throw new Error('请选择你的学校')
      const nextFingerprint = `${ticket}:${selected.value}`
      if (!key || fingerprint !== nextFingerprint) { key = dependencies.requestKey(); fingerprint = nextFingerprint }
      const result = await dependencies.identity.onboarding('complete', { ticket, school_id: selected.value, request_key: key })
      if (await receive(result, current)) throw new Error('协议已更新，请重新阅读并确认')
    })
  }
  function find(reset = true) { return run(async current => { await search(reset, current) }) }
  function choose(id: string) { if (!bound.value) selected.value = id }
  function toggle(values: string[]) {
    checked.value = Object.fromEntries(documents.value.filter(row => values.includes(row.kind)).map(row => [row.kind, row.revision]))
  }
  async function cancel() {
    const ticket = progress.value?.ticket
    generation++; busy.value = false; progress.value = undefined; checked.value = {}; selected.value = ''; bound.value = ''; key = ''; error.value = '尚未登录，可重新选择学校并阅读协议后继续。'
    const turn = generation
    if (ticket) { try { await dependencies.identity.cancelOnboarding(ticket) } catch { if (turn === generation) error.value = '本次登录已在本机取消，请重新开始；服务端取消未确认。' } }
  }
  function dispose() { generation++; progress.value = undefined; checked.value = {}; busy.value = false }
  return { documents, schools, selected, bound, query, more, checked, progress, busy, error, loaded, missing, show, submit, find, choose, toggle, cancel, dispose }
}
