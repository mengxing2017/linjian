import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import * as Vue from 'vue'
import { compileScript, parse } from 'vue/compiler-sfc'
import ts from 'typescript'

export interface TestNode {
  type: string
  text: string
  props: Record<string, unknown>
  children: TestNode[]
  parent: TestNode | null
}

function node(type: string, text = ''): TestNode {
  return { type, text, props: {}, children: [], parent: null }
}

export function component(url: URL, dependencies: Record<string, unknown> = {}) {
  const filename = fileURLToPath(url)
  const isCustomElement = (tag: string) =>
    ['view', 'text', 'image', 'button', 'input', 'textarea', 'scroll-view', 'swiper', 'swiper-item', 'page-container'].includes(
      tag,
    )
  const { descriptor } = parse(readFileSync(filename, 'utf8'), {
    filename,
    templateParseOptions: { isCustomElement },
  })
  const script = compileScript(descriptor, {
    id: filename,
    inlineTemplate: true,
    templateOptions: {
      compilerOptions: { isCustomElement },
    },
  })
  const { outputText } = ts.transpileModule(script.content, {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
      esModuleInterop: true,
    },
  })
  const exports: { default?: Vue.Component } = {}
  const require = (name: string) => {
    if (name === 'vue') return Vue
    if (name in dependencies) {
      const dependency = dependencies[name]
      return dependency && typeof dependency === 'object' && 'default' in dependency
        ? { ...dependency, __esModule: true }
        : dependency
    }
    throw new Error(`Missing component dependency: ${name}`)
  }
  new Function('require', 'exports', outputText)(require, exports)
  return exports.default!
}

export function mount(component: Vue.Component, props: Record<string, unknown> = {}) {
  const renderer = Vue.createRenderer<TestNode, TestNode>({
    createElement: (type) => node(type),
    createText: (text) => node('#text', text),
    createComment: (text) => node('#comment', text),
    setText: (target, text) => {
      target.text = text
    },
    setElementText: (target, text) => {
      target.text = text
      target.children = []
    },
    parentNode: (target) => target.parent,
    nextSibling: (target) => {
      const siblings = target.parent?.children ?? []
      return siblings[siblings.indexOf(target) + 1] ?? null
    },
    patchProp: (target, key, _previous, value) => {
      target.props[key] = value
    },
    insert: (target, parent, anchor) => {
      if (target.parent) {
        const previous = target.parent.children
        previous.splice(previous.indexOf(target), 1)
      }
      const index = anchor ? parent.children.indexOf(anchor) : -1
      parent.children.splice(index < 0 ? parent.children.length : index, 0, target)
      target.parent = parent
    },
    remove: (target) => {
      const siblings = target.parent?.children
      if (siblings) siblings.splice(siblings.indexOf(target), 1)
      target.parent = null
    },
  })
  const root = node('root')
  const state = Vue.reactive(props)
  const exposed = Vue.shallowRef()
  const app = renderer.createApp({ render: () => Vue.h(component, { ...state, ref: exposed }) })
  app.mount(root)
  return { root, props: state, exposed, unmount: () => app.unmount() }
}

export function findAll(root: TestNode, predicate: (target: TestNode) => boolean): TestNode[] {
  return [
    ...(predicate(root) ? [root] : []),
    ...root.children.flatMap((child) => findAll(child, predicate)),
  ]
}

export function text(root: TestNode): string {
  return root.type === '#comment' ? '' : root.text + root.children.map(text).join('')
}

export function tap(target: TestNode) {
  let stopped = false
  const event = {
    stopPropagation: () => {
      stopped = true
    },
  }
  let current: TestNode | null = target
  while (current) {
    const handler = current.props.onTap as
      | ((event: { stopPropagation: () => void }) => void)
      | undefined
    handler?.(event)
    if (stopped) break
    current = current.parent
  }
}
