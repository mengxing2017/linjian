import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { test } from 'node:test'
import { fileURLToPath } from 'node:url'
import ts from 'typescript'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const baseline = JSON.parse(
  fs.readFileSync(new URL('./fixtures/public-entries.json', import.meta.url), 'utf8'),
)

test('entry splitting preserves every existing public symbol and its type/value role', () => {
  for (const [directory, config] of [
    ['apps/web', 'tsconfig.app.json'],
    ['apps/miniapp', 'tsconfig.json'],
    ['packages/china-regions', 'tsconfig.json'],
  ]) {
    const configuration = ts.readConfigFile(path.join(root, directory, config), ts.sys.readFile)
    const parsed = ts.parseJsonConfigFileContent(
      configuration.config,
      ts.sys,
      path.join(root, directory),
    )
    const program = ts.createProgram(parsed.fileNames, parsed.options)
    const checker = program.getTypeChecker()
    for (const [entry, expected] of Object.entries(baseline).filter(([file]) =>
      file.startsWith(directory + '/'),
    )) {
      const source = program.getSourceFile(path.join(root, entry))
      assert.ok(source, entry)
      const symbols = checker
        .getExportsOfModule(checker.getSymbolAtLocation(source))
        .map((symbol) => {
          const target =
            symbol.flags & ts.SymbolFlags.Alias ? checker.getAliasedSymbol(symbol) : symbol
          return {
            name: symbol.name,
            type: Boolean(target.flags & ts.SymbolFlags.Type),
            value: Boolean(target.flags & ts.SymbolFlags.Value),
          }
        })
        .sort((left, right) => left.name.localeCompare(right.name))
      assert.deepEqual(
        {
          types: symbols.filter((symbol) => symbol.type).map((symbol) => symbol.name),
          values: symbols.filter((symbol) => symbol.value).map((symbol) => symbol.name),
        },
        expected,
        entry,
      )
    }
  }
})
