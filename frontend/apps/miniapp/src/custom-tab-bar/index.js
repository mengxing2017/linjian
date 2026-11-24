Component({
  data: {
    items: [],
    selected: '',
    badge: '',
    theme: '',
    ready: false,
  },
  lifetimes: {
    attached() { getApp().linjianTabbar?.attach(this) },
    detached() { getApp().linjianTabbar?.detach(this) },
  },
  pageLifetimes: {
    show() { getApp().linjianTabbar?.attach(this) },
  },
  methods: {
    activate(event) {
      getApp().linjianTabbar?.activate(event.currentTarget.dataset.id)
    },
  },
})
