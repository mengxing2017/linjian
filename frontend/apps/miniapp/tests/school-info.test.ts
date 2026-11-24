import { defineComponent, h, nextTick, reactive, ref } from 'vue'
import { expect, it, vi } from 'vitest'
import { component, findAll, mount, tap, text } from './support/vue-component'

it('shows only the bound school and routes school changes through the application process', async () => {
  const context = reactive({ school: undefined as { id: string; name: string } | undefined })
  const requireSchool = vi.fn(async () => {
    context.school = { id: 'bound-school', name: '当前绑定学校' }
    return true
  })
  const navigate = vi.fn()
  const blank = defineComponent({ setup: () => () => h('view') })
  const screen = component(new URL('../src/features/school-context/SchoolInfo.vue', import.meta.url), {
    './context': { schoolContext: context, requireSchool },
    '../../shared/use-page': { navigate, usePage: () => ({ busy: ref(false), error: ref(''), run: (action: () => Promise<void>) => action() }) },
    '../../shared/PageStatus.vue': { default: blank },
    './SchoolHeader.vue': { default: blank },
    '@linjian/china-regions': { formatRegion: () => '当前学校所在地区' },
  })
  const mounted = mount(screen)
  expect(text(mounted.root)).not.toContain('学校暂不可用')
  await mounted.exposed.value.show()
  await nextTick()
  expect(requireSchool).toHaveBeenCalledOnce()
  expect(text(mounted.root)).toContain('当前绑定学校')
  expect(text(mounted.root)).toContain('不能直接切换学校')
  expect(findAll(mounted.root, target => target.type === 'input')).toHaveLength(0)
  const buttons = findAll(mounted.root, target => target.type === 'button')
  expect(buttons).toHaveLength(1)
  tap(buttons[0])
  expect(navigate).toHaveBeenCalledWith('/pages/user/schoolChange')
  mounted.unmount()
})
