import assert from 'node:assert/strict'
import fs from 'node:fs'
import test from 'node:test'

test('web CSP permits authenticated blob image previews without relaxing script policy', () => {
  const caddyfile = fs.readFileSync(new URL('../Caddyfile', import.meta.url), 'utf8')
  const header = caddyfile.match(/Content-Security-Policy\s+"([^"]+)"/)
  assert.ok(header, 'the web server must send a Content-Security-Policy header')
  const directives = new Map(
    header[1].split(';').map((directive) => {
      const [name, ...sources] = directive.trim().split(/\s+/)
      return [name, sources]
    }),
  )

  assert.ok(directives.get('img-src')?.includes('blob:'), 'image previews use object URLs')
  assert.ok(directives.get('img-src')?.includes("'self'"))
  assert.deepEqual(directives.get('default-src'), ["'self'"])
  assert.deepEqual(directives.get('script-src'), ["'self'"])
  assert.deepEqual(directives.get('connect-src'), ["'self'"])
  assert.deepEqual(directives.get('object-src'), ["'none'"])
  assert.deepEqual(directives.get('frame-ancestors'), ["'none'"])
})
