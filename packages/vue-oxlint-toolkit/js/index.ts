import type { Comment, Diagnostic, Range } from '@oxlint/plugins'
import type { NativeParseResult, NativeRange } from '../bindings'
import { nativeParse } from '../bindings'

export interface ParseResult {
  // ast: AST.ESLintProgram (the import of AST brings a lot of unnecessary types definition in dts, remove it temporarily)
  ast: any
  comments: Comment[]
  irregularWhitespaces: Range[]
  errors: Diagnostic[]
}

export function parse(_path: string, source: string, _options?: {}): ParseResult {
  const result: NativeParseResult = nativeParse(source)
  const locator = createLocator(source)
  const ast = JSON.parse(result.astJson)

  hydrateAstLocations(ast, locator)
  ast.comments = result.comments.map((comment) => ({
    type: comment.type,
    value: comment.value,
    start: locator.toIndex(comment.start),
    end: locator.toIndex(comment.end),
    range: toRange(comment, locator),
    loc: toLocation(comment, locator),
  }))
  ast.tokens = result.templateTokens.map((token) => ({
    type: token.type,
    value: token.value,
    range: toRange(token, locator),
    loc: toLocation(token, locator),
  }))

  return {
    ast,
    comments: ast.comments,
    irregularWhitespaces: result.irregularWhitespaces.map((range) => toRange(range, locator)),
    errors: result.errors.map((error) => ({
      message: error.message,
      loc: toLocation(error, locator),
    })),
  }
}

function toRange(range: NativeRange, locator: ReturnType<typeof createLocator>): Range {
  return [locator.toIndex(range.start), locator.toIndex(range.end)]
}

function toLocation(range: NativeRange, locator: ReturnType<typeof createLocator>) {
  return {
    start: locator(range.start),
    end: locator(range.end),
  }
}

function createLocator(source: string) {
  const lineStarts = [{ byte: 0, index: 0 }]
  const byteToIndex = new Map<number, number>([[0, 0]])
  let byteOffset = 0

  for (let index = 0; index < source.length; ) {
    const codePoint = source.codePointAt(index)!
    const codeUnitLength = codePoint > 0xffff ? 2 : 1

    byteOffset += utf8ByteLength(codePoint)
    index += codeUnitLength
    byteToIndex.set(byteOffset, index)

    if (codePoint === 10) {
      lineStarts.push({ byte: byteOffset, index })
    }
  }

  const toIndex = (offset: number) => {
    const index = byteToIndex.get(offset)

    if (index === undefined) {
      throw new RangeError(`Offset ${offset} is not on a UTF-8 character boundary.`)
    }

    return index
  }

  const locator = (offset: number) => {
    let low = 0
    let high = lineStarts.length - 1

    while (low <= high) {
      const mid = (low + high) >> 1

      if (lineStarts[mid].byte <= offset) {
        low = mid + 1
      } else {
        high = mid - 1
      }
    }

    const lineIndex = Math.max(0, high)
    const index = toIndex(offset)

    return {
      line: lineIndex + 1,
      column: index - lineStarts[lineIndex].index,
    }
  }

  locator.toIndex = toIndex

  return locator
}

function utf8ByteLength(codePoint: number) {
  if (codePoint <= 0x7f) {
    return 1
  }

  if (codePoint <= 0x7ff) {
    return 2
  }

  if (codePoint <= 0xffff) {
    return 3
  }

  return 4
}

function hydrateAstLocations(node: any, locator: ReturnType<typeof createLocator>) {
  if (!node || typeof node !== 'object') {
    return
  }

  if (Array.isArray(node.range)) {
    const [start, end] = node.range
    node.range = [locator.toIndex(start), locator.toIndex(end)]
    node.loc = {
      start: locator(start),
      end: locator(end),
    }
  }

  for (const value of Object.values(node)) {
    if (Array.isArray(value)) {
      for (const item of value) {
        hydrateAstLocations(item, locator)
      }
    } else if (value && typeof value === 'object') {
      hydrateAstLocations(value, locator)
    }
  }
}
