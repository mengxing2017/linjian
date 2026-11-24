const attachAttempts = new WeakMap()
function attach(component) {
  const app = getApp()
  const attempts = (attachAttempts.get(component) || 0) + 1
  if (app && app.linjianTabbar) {
    attachAttempts.delete(component)
    app.linjianTabbar.attach(component)
    return
  }
  if (attempts > 40) return
  attachAttempts.set(component, attempts)
  setTimeout(() => attach(component), 50)
}
Component({
  data: {
    items: [],
    selected: '',
    badge: '',
    theme: '',
    ready: false,
    failedSources: {},
  },
  lifetimes: {
    attached() { attach(this) },
    detached() {
      const app = getApp()
      attachAttempts.delete(this)
      if (app && app.linjianTabbar) app.linjianTabbar.detach(this)
    },
  },
  pageLifetimes: {
    show() { attach(this) },
  },
  methods: {
    iconError(event) {
      const source = event.currentTarget.dataset.source
      if (!this.data.items.some(item => item.icon === source || item.selectedIcon === source)) return
      this.setData({ failedSources: { ...this.data.failedSources, [source]: true } })
    },
    activate(event) {
      const app = getApp()
      if (app && app.linjianTabbar) app.linjianTabbar.activate(event.currentTarget.dataset.id)
    },
  },
})
