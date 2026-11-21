import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import ts from 'typescript'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')

export function dependencies(file, source) {
  const tree = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true)
  const result = []
  function visit(node) {
    if (
      (ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) &&
      node.moduleSpecifier &&
      ts.isStringLiteral(node.moduleSpecifier)
    )
      result.push(node.moduleSpecifier.text)
    if (
      ts.isImportTypeNode(node) &&
      ts.isLiteralTypeNode(node.argument) &&
      ts.isStringLiteral(node.argument.literal)
    )
      result.push(node.argument.literal.text)
    if (ts.isCallExpression(node) && node.expression.kind === ts.SyntaxKind.ImportKeyword) {
      const argument = node.arguments[0]
      result.push(argument && ts.isStringLiteral(argument) ? argument.text : '<computed-import>')
    }
    ts.forEachChild(node, visit)
  }
  visit(tree)
  return result
}

export function entryViolations(file, source) {
  if (!/^(?:src\/modules\/.+|src\/http)\/index\.ts$/.test(file)) return []
  const tree = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true)
  return tree.statements.flatMap((statement) => {
    if (!ts.isExportDeclaration(statement)) return ['index.ts 只允许显式 export / export type']
    if (!statement.exportClause || !ts.isNamedExports(statement.exportClause))
      return ['index.ts 禁止 export *，请列出公开符号']
    return []
  })
}

export function cycles(graph) {
  const active = new Set()
  const visited = new Set()
  const stack = []
  const errors = []
  function visit(file) {
    if (active.has(file)) {
      errors.push([...stack.slice(stack.indexOf(file)), file].join(' -> '))
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
  return errors
}

export function check(directory = root) {
  const files = []
  function walk(folder) {
    for (const entry of fs.readdirSync(folder, { withFileTypes: true })) {
      const file = path.join(folder, entry.name)
      if (entry.isDirectory()) walk(file)
      else if (file.endsWith('.ts')) files.push(file)
    }
  }
  walk(path.join(directory, 'src'))
  const relative = (file) => path.relative(directory, file).split(path.sep).join('/')
  const errors = []
  const graph = new Map()
  for (const file of files) {
    const name = relative(file)
    const source = fs.readFileSync(file, 'utf8')
    errors.push(...entryViolations(name, source).map((error) => `${name}: ${error}`))
    const edges = []
    for (const specifier of dependencies(file, source)) {
      if (specifier === '<computed-import>') {
        errors.push(`${name}: 动态导入必须使用字面量`)
        continue
      }
      if (!specifier.startsWith('.') && !specifier.startsWith('@linjian/api-contract/')) continue
      const resolved = ts.resolveModuleName(
        specifier,
        file,
        { moduleResolution: ts.ModuleResolutionKind.Bundler },
        ts.sys,
      ).resolvedModule?.resolvedFileName
      if (!resolved) {
        errors.push(`${name}: 无法解析 ${specifier}`)
        continue
      }
      const target = relative(resolved)
      if (target.startsWith('src/')) edges.push(target)
      if (path.basename(file) !== 'index.ts' && path.basename(resolved) === 'index.ts')
        errors.push(`${name}: 具名模块禁止依赖入口 ${specifier}`)
    }
    graph.set(name, edges)
  }
  errors.push(...cycles(graph).map((cycle) => `循环依赖: ${cycle}`))
  if (errors.length) throw new Error(errors.join('\n'))
  return files.length
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url))
  console.log(`Contract boundaries passed (${check()} files).`)
