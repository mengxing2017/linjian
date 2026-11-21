import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import ts from 'typescript'

const contractManifest = JSON.parse(
  fs.readFileSync(new URL('../../api-contract/package.json', import.meta.url), 'utf8'),
)
const contractEntries = new Set(
  Object.keys(contractManifest.exports).map((entry) => `@linjian/api-contract/${entry.slice(2)}`),
)
const featureOf = (file) =>
  file.match(/^apps\/(?:web|miniapp)\/src\/features\/(?:admin\/)?[^/]+/)?.[0]

export function imports(source, filename, runtimeOnly = false) {
  const chunks = filename.endsWith('.vue')
    ? [...source.matchAll(/<script\b[^>]*>([\s\S]*?)<\/script>/g)].map((m) => m[1])
    : [source]
  const found = []
  for (const chunk of chunks) {
    const tree = ts.createSourceFile(
      filename + '.tsx',
      chunk,
      ts.ScriptTarget.Latest,
      true,
      ts.ScriptKind.TSX,
    )
    function visit(node) {
      if (
        (ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) &&
        node.moduleSpecifier &&
        ts.isStringLiteral(node.moduleSpecifier)
      ) {
        const clause = ts.isImportDeclaration(node) ? node.importClause : node
        const bindings = ts.isImportDeclaration(node) ? clause?.namedBindings : node.exportClause
        const typeOnly =
          clause?.isTypeOnly ||
          (!clause?.name &&
            bindings &&
            (ts.isNamedImports(bindings) || ts.isNamedExports(bindings)) &&
            bindings.elements.length > 0 &&
            bindings.elements.every((element) => element.isTypeOnly))
        if (!runtimeOnly || !typeOnly) found.push(node.moduleSpecifier.text)
      }
      if (
        ts.isCallExpression(node) &&
        (node.expression.kind === ts.SyntaxKind.ImportKeyword ||
          (ts.isIdentifier(node.expression) && node.expression.text === 'require'))
      ) {
        if (node.arguments[0] && ts.isStringLiteralLike(node.arguments[0]))
          found.push(node.arguments[0].text)
        else found.push('<computed-import>')
      }
      if (
        !runtimeOnly &&
        ts.isImportTypeNode(node) &&
        ts.isLiteralTypeNode(node.argument) &&
        ts.isStringLiteral(node.argument.literal)
      )
        found.push(node.argument.literal.text)
      ts.forEachChild(node, visit)
    }
    visit(tree)
  }
  return found
}
export function actionColumnViolations(file, source) {
  if (!/\.(?:ts|tsx)$/.test(file)) return []
  if (/\/shared\/(?:row-actions|table-action)\.tsx?$/.test(file)) return []
  const tree = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX)
  const errors = []
  const name = (node) =>
    ts.isIdentifier(node) || ts.isStringLiteral(node) ? node.text : undefined
  const literal = (node) => {
    let value = node
    while (ts.isAsExpression(value) || ts.isParenthesizedExpression(value))
      value = value.expression
    return ts.isStringLiteral(value) ? value.text : undefined
  }
  function visit(node) {
    if (ts.isObjectLiteralExpression(node)) {
      let title
      let fixed
      for (const property of node.properties) {
        if (!ts.isPropertyAssignment(property)) continue
        if (name(property.name) === 'title') title = literal(property.initializer)
        if (name(property.name) === 'fixed') fixed = literal(property.initializer)
      }
      if (title === '操作' && fixed !== 'right')
        errors.push('操作列必须设置 fixed: \'right\'')
    }
    ts.forEachChild(node, visit)
  }
  visit(tree)
  return errors
}
export function violations(file, source) {
  const errors = []
  if (/^(?:apps\/(?:web|miniapp)|packages\/[^/]+)\/src\/(?:.*\/)?index\.tsx?$/.test(file)) {
    const tree = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX)
    for (const statement of tree.statements) {
      if (
        !ts.isImportDeclaration(statement) &&
        !ts.isExportDeclaration(statement) &&
        !ts.isInterfaceDeclaration(statement) &&
        !ts.isTypeAliasDeclaration(statement)
      )
        errors.push('index 只允许 import/export/类型声明，实现与初始化应移到具名模块')
      if (ts.isImportDeclaration(statement) && !statement.importClause)
        errors.push('index 禁止副作用导入')
    }
  }
  for (const error of actionColumnViolations(file, source)) errors.push(error)
  for (const spec of imports(source, file)) {
    if (spec === '<computed-import>') errors.push('动态导入必须使用可检查的字面量')
    if (
      spec.startsWith('@linjian/api-contract') &&
      !/\/features\/.+\/api\/(?:index|client|contract)\.ts$/.test(file)
    )
      errors.push('只允许 feature/api/index.ts、client.ts 或 contract.ts 引用 contract')
    if (spec.startsWith('@linjian/api-contract') && !contractEntries.has(spec))
      errors.push('contract 只允许 package.json#exports 声明的公开子路径，禁止深层导入')
    const resolved = spec.startsWith('.')
      ? path.posix.normalize(path.posix.join(path.posix.dirname(file), spec))
      : spec
    if (spec.startsWith('.') && /(?:^|\/)api-contract(?:\/|$)/.test(resolved))
      errors.push('禁止通过源码相对路径绕过 contract 包导出')
    const app = file.match(/apps\/(web|miniapp)\//)?.[1]
    if (
      /\/api\/client(?:\.ts)?$/.test(resolved) &&
      featureOf(file) &&
      featureOf(resolved) &&
      featureOf(file) !== featureOf(resolved)
    )
      errors.push('禁止跨 feature 深层导入 api/client，请使用公开 api 入口或应用级 client')
    if (
      app &&
      ((app === 'web' && resolved.includes('apps/miniapp')) ||
        (app === 'miniapp' && resolved.includes('apps/web')))
    )
      errors.push('客户端禁止互相依赖')
    if (
      file.startsWith('packages/') &&
      (/apps\//.test(resolved) || /react|vue|antd|uni-app/.test(spec))
    )
      errors.push('纯共享包禁止依赖应用或 UI 框架')
    if (/(?:^|\/)uniapp\//.test(resolved) || resolved.includes('uniapp/config'))
      errors.push('禁止引用旧 uniapp 工程')
    if (/\/features\/community\//.test(file) && resolved.includes('/features/admin/'))
      errors.push('社区禁止依赖管理端')
  }
  if (/^apps\/(?:web|miniapp)\/src\//.test(file) && !/\.(?:test|spec)\./.test(file)) {
    const chunks = file.endsWith('.vue')
      ? [...source.matchAll(/<script\b[^>]*>([\s\S]*?)<\/script>/g)].map((match) => match[1])
      : [source]
    for (const chunk of chunks) {
      const tree = ts.createSourceFile(
        file + '.tsx',
        chunk,
        ts.ScriptTarget.Latest,
        true,
        ts.ScriptKind.TSX,
      )
      function visit(node) {
        if (
          ts.isCallExpression(node) &&
          ts.isIdentifier(node.expression) &&
          ['createClient', 'createUniTransport'].includes(node.expression.text) &&
          !/^apps\/(?:web|miniapp)\/src\/app\/api\/client\.ts$/.test(file)
        )
          errors.push('共享 client / transport 只能在 app/api/client.ts 装配')
        ts.forEachChild(node, visit)
      }
      visit(tree)
      for (const statement of tree.statements) {
        if (!ts.isImportDeclaration(statement) || !ts.isStringLiteral(statement.moduleSpecifier))
          continue
        const specifier = statement.moduleSpecifier.text
        const resolved = path.posix.normalize(path.posix.join(path.posix.dirname(file), specifier))
        const bindings = statement.importClause?.namedBindings
        if (
          featureOf(file) &&
          featureOf(resolved) &&
          featureOf(resolved) !== featureOf(file) &&
          /\/api(?:\/index)?(?:\.ts)?$/.test(resolved) &&
          bindings &&
          ts.isNamedImports(bindings) &&
          bindings.elements.some((element) =>
            ['client', 'browserClient'].includes((element.propertyName ?? element.name).text),
          )
        )
          errors.push('共享 client 必须来自 app/api，不能通过其他 feature 的 API 入口取得')
      }
    }
  }
  if (
    file.includes('apps/miniapp/src/') &&
    !file.endsWith('/app/transport.ts') &&
    /\b(?:uni|wx)\s*\.\s*(?:request|uploadFile|downloadFile)\s*\(/.test(source)
  )
    errors.push('业务请求必须通过 contract client 和平台 transport')
  return errors
}
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
export function check() {
  const errors = []
  const sources = new Map()
  function walk(dir) {
    if (!fs.existsSync(dir)) return
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      if (['node_modules', 'dist'].includes(entry.name)) continue
      const file = path.join(dir, entry.name)
      if (entry.isDirectory()) walk(file)
      else if (/\.(?:ts|tsx|vue)$/.test(file)) {
        const relative = path.relative(root, file).split(path.sep).join('/')
        const source = fs.readFileSync(file, 'utf8')
        if (!/\.(?:test|spec)\.|\.d\.ts$/.test(relative)) sources.set(relative, source)
        for (const error of violations(relative, source)) errors.push(`${relative}: ${error}`)
      }
    }
  }
  walk(path.join(root, 'apps'))
  walk(path.join(root, 'packages'))
  errors.push(...dependencyCycles(sources).map((cycle) => `循环依赖: ${cycle.join(' -> ')}`))
  if (errors.length) throw new Error(errors.join('\n'))
  console.log('Frontend import boundaries passed.')
}
export function dependencyCycles(sources) {
  const graph = new Map()
  for (const [file, source] of sources) {
    const edges = []
    for (const specifier of imports(source, file, true)) {
      const base = specifier.startsWith('.')
        ? path.posix.normalize(path.posix.join(path.posix.dirname(file), specifier))
        : specifier === '@linjian/china-regions'
          ? 'packages/china-regions/src/index.ts'
          : undefined
      if (!base) continue
      const target = [
        base,
        ...['.ts', '.tsx', '.vue', '/index.ts', '/index.tsx'].map((suffix) => base + suffix),
      ].find((candidate) => sources.has(candidate))
      if (target) edges.push(target)
    }
    graph.set(file, edges)
  }
  const active = new Set()
  const visited = new Set()
  const stack = []
  const cycles = []
  function visit(file) {
    if (active.has(file)) {
      cycles.push([...stack.slice(stack.indexOf(file)), file])
      return
    }
    if (visited.has(file)) return
    visited.add(file)
    active.add(file)
    stack.push(file)
    for (const target of graph.get(file) ?? []) visit(target)
    stack.pop()
    active.delete(file)
  }
  for (const file of graph.keys()) visit(file)
  return cycles
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) check()
