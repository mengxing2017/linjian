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
    attached() { getApp().linjianTabbar?.attach(this) },
    detached() { getApp().linjianTabbar?.detach(this) },
  },
  pageLifetimes: {
    show() { getApp().linjianTabbar?.attach(this) },
  },
  methods: {
    iconError(event) {
      const source = event.currentTarget.dataset.source
      if (!this.data.items.some(item => item.icon === source || item.selectedIcon === source)) return
      this.setData({ failedSources: { ...this.data.failedSources, [source]: true } })
    },
    activate(event) {
      getApp().linjianTabbar?.activate(event.currentTarget.dataset.id)
    },
  },
})
