import { beforeEach, vi } from 'vitest'
import { configure } from '@testing-library/react'

// Ant Design async renders can exceed the default one-second query wait on CI.
configure({ asyncUtilTimeout: 5000 })

// JSDOM computes element styles, but does not implement pseudo-element styles.
// Keep real visibility checks while allowing Ant Design scrollbar measurements.
const getComputedStyle = window.getComputedStyle.bind(window)

beforeEach(() => {
  vi.stubGlobal('getComputedStyle', (element: Element) => getComputedStyle(element))
  vi.stubGlobal('ResizeObserver', class { observe() {} unobserve() {} disconnect() {} })
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: false, media: query, onchange: null,
    addListener: vi.fn(), removeListener: vi.fn(),
    addEventListener: vi.fn(), removeEventListener: vi.fn(), dispatchEvent: vi.fn(),
  }))
})
