import { computed, ref } from 'vue'
import type { ContactContext } from '../operations/use-customer-contacts'

interface Contacts {
  prepare: () => void
  show: (context: ContactContext) => Promise<void>
  hide: () => void
}
interface Environment {
  contacts: () => Contacts | undefined
  restore: () => Promise<void>
  loggedIn: () => boolean
  loadSchools: () => Promise<boolean>
  school: () => string
  revision: () => number
  key: () => string
  profile: (current: () => boolean) => Promise<void>
  failure: (cause: unknown) => string
}

export function createContactsPage(env: Environment) {
  const profileBusy = ref(false)
  const preparationBusy = ref(false)
  const busy = computed(() => profileBusy.value || preparationBusy.value)
  const error = ref('')
  let generation = 0
  let profileGeneration = 0
  let preparationGeneration = 0
  let visible = false
  let disposed = false
  let preparing: Promise<void> | undefined
  let preparedIdentity = false
  let identityRevision = -1

  async function readProfile() {
    if (!visible || disposed || !env.loggedIn()) return
    const turn = ++profileGeneration
    const page = generation
    const scope = env.key()
    const current = () => visible && !disposed && generation === page && profileGeneration === turn && scope === env.key()
    profileBusy.value = true
    error.value = ''
    try { await env.profile(current) }
    catch (cause) {
      if (current()) {
        const message = env.failure(cause)
        if (current()) error.value = message
      }
    } finally { if (current()) profileBusy.value = false }
  }

  function prepare(profile: boolean) {
    if (!visible || disposed) return Promise.resolve()
    if (preparing) return preparing
    const turn = ++preparationGeneration
    const page = generation
    const revision = env.revision()
    const current = () => visible && !disposed && generation === page && preparationGeneration === turn && revision === env.revision()
    env.contacts()?.prepare()
    preparationBusy.value = profile
    if (profile) error.value = ''
    const pending = (async () => {
      let context: ContactContext
      try {
        await env.restore()
        if (!current()) return
        preparedIdentity = true
        identityRevision = revision
        if (!env.loggedIn()) {
          await env.contacts()?.show({ state: 'login-required' })
          return
        }
        try {
          const available = await env.loadSchools()
          context = available && env.school() ? { state: 'ready', school: env.school() } : { state: 'school-required' }
        } catch (cause) { context = { state: 'error', cause } }
      } catch (cause) {
        if (!current()) return
        preparedIdentity = false
        if (profile) {
          const message = env.failure(cause)
          if (current()) error.value = message
        }
        context = { state: 'error', cause }
      }
      if (!current()) return
      preparing = undefined
      preparationBusy.value = false
      await Promise.allSettled([
        profile && env.loggedIn() ? readProfile() : Promise.resolve(),
        env.contacts()?.show(context),
      ])
    })().finally(() => {
      if (preparing === pending) preparing = undefined
      if (current()) preparationBusy.value = false
    })
    preparing = pending
    return pending
  }

  function show() {
    if (disposed) return Promise.resolve()
    hide()
    visible = true
    return prepare(true)
  }

  function hide() {
    visible = false
    generation++
    preparationGeneration++
    profileGeneration++
    preparing = undefined
    profileBusy.value = false
    preparationBusy.value = false
    env.contacts()?.hide()
  }

  function resetProfile() {
    profileGeneration++
    profileBusy.value = false
    preparationBusy.value = false
    error.value = ''
  }

  function dispose() { hide(); disposed = true }

  const needsIdentity = () => !preparedIdentity || identityRevision !== env.revision()
  return { busy, error, show, hide, dispose, resetProfile, reloadProfile: () => needsIdentity() ? prepare(true) : readProfile(), retryPrepare: () => prepare(needsIdentity()) }
}
