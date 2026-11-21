import path from 'node:path'
import ts from 'typescript'
import { expect, it } from 'vitest'
import manifest from '../package.json'
import baseline from './fixtures/public-exports.json'

it('preserves all public symbols and their type/value roles', () => {
  const configuration = ts.readConfigFile('tsconfig.json', ts.sys.readFile)
  const parsed = ts.parseJsonConfigFileContent(configuration.config, ts.sys, '.')
  const program = ts.createProgram(parsed.fileNames, parsed.options)
  const checker = program.getTypeChecker()
  const entries = {
    ...manifest.exports,
    'internal/product-workflows': './src/modules/product-workflows/index.ts',
  }
  expect(Object.keys(entries).sort()).toEqual(Object.keys(baseline).sort())
  for (const [entry, file] of Object.entries(entries)) {
    const source = program.getSourceFile(path.resolve(file))!
    const symbols = checker
      .getExportsOfModule(checker.getSymbolAtLocation(source)!)
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
    const additions = entry === './source-access' ? ['createSourceAccessClient'] : []
    const existing = symbols.filter((symbol) => !additions.includes(symbol.name))
    expect(
      {
        types: existing.filter((symbol) => symbol.type).map((symbol) => symbol.name),
        values: existing.filter((symbol) => symbol.value).map((symbol) => symbol.name),
      },
      entry,
    ).toEqual(baseline[entry as keyof typeof baseline])
    for (const name of additions)
      expect(symbols.find((symbol) => symbol.name === name)).toEqual({
        name,
        type: false,
        value: true,
      })
  }
})
