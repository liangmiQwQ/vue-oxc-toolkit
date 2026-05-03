import { it, expect } from 'vite-plus/test'
import * as vueEslintParser from 'vue-eslint-parser'
import { parse } from '../js'

it('parses Vue SFCs through the native parser path', () => {
  const source = `<script setup lang="ts">
const msg: string = 'hello'
</script>
<template>{{ msg }}</template>`
  const result = parse('fixture.vue', source)
  const elements = result.ast.templateBody.children.filter((node: any) => node.type === 'VElement')

  expect(result.ast.type).toBe('Program')
  expect(elements[0]).toMatchObject({
    type: 'VElement',
    rawName: 'script',
    script: {
      kind: 'setup',
      bodyLength: 1,
    },
  })
  expect(elements[1]).toMatchObject({
    type: 'VElement',
    rawName: 'template',
    children: [
      {
        type: 'VExpressionContainer',
        expression: {
          type: 'Identifier',
        },
      },
    ],
  })
  expect(result.errors).toEqual([])
})

it('keeps script comments and converts native byte offsets to JavaScript locations', () => {
  const result = parse(
    'fixture.vue',
    `<script>
const s = "你好" // hello
</script>`,
  )

  expect(result.comments[0]).toMatchObject({
    type: 'Line',
    value: ' hello',
    range: [24, 32],
    loc: {
      start: { line: 2, column: 15 },
      end: { line: 2, column: 23 },
    },
  })
})

it('returns directive metadata from the native parser path', () => {
  const result = parse(
    'fixture.vue',
    `<template><button :[className].foo="cls" @click.stop="submit" /></template>`,
  )
  const template = result.ast.templateBody.children.find((node: any) => node.rawName === 'template')
  const button = template.children.find((node: any) => node.rawName === 'button')

  expect(button.startTag.attributes[0]).toMatchObject({
    type: 'VDirective',
    key: {
      name: 'bind',
      argument: {
        raw: '[className]',
        kind: 'dynamic',
        expression: {
          type: 'Identifier',
        },
      },
      modifiers: [{ name: 'foo' }],
    },
    value: {
      raw: 'cls',
      expression: {
        type: 'Identifier',
      },
    },
  })
  expect(button.startTag.attributes[1]).toMatchObject({
    key: {
      name: 'on',
      argument: {
        raw: 'click',
      },
      modifiers: [{ name: 'stop' }],
    },
  })
})

it('returns special directive expression metadata from the native parser path', () => {
  const result = parse(
    'fixture.vue',
    `<template><div v-for="(item, index) in items" v-slot="slotProps" @click="count += 1" /></template>`,
  )
  const template = result.ast.templateBody.children.find((node: any) => node.rawName === 'template')
  const div = template.children.find((node: any) => node.rawName === 'div')

  expect(div.startTag.attributes[0].value.expression).toMatchObject({
    type: 'VForExpression',
    left: {
      type: 'FormalParameters',
      count: 2,
    },
    right: {
      type: 'Identifier',
    },
  })
  expect(div.startTag.attributes[1].value.expression).toMatchObject({
    type: 'VSlotExpression',
    params: {
      count: 1,
    },
  })
  expect(div.startTag.attributes[2].value.expression).toMatchObject({
    type: 'VOnExpression',
    bodyLength: 1,
  })
})

it('matches vue-eslint-parser V-tree shape for representative fixtures', () => {
  for (const source of [
    `<template><div id="app">{{ msg }}</div></template>`,
    `<template><button :class.foo="cls" disabled /></template>`,
    `<template>hello<input disabled></template>`,
  ]) {
    const ours = parse('fixture.vue', source).ast
    const expected = vueEslintParser.parse(source, { ecmaVersion: 'latest' })

    expect(comparableTemplate(ours)).toEqual(comparableTemplate(expected))
  }
})

function comparableTemplate(ast: any) {
  const template =
    ast.templateBody?.type === 'VDocumentFragment'
      ? ast.templateBody.children.find((node: any) => node.rawName === 'template')
      : ast.templateBody

  return comparableNode(template)
}

function comparableNode(node: any): any {
  if (!node) {
    return null
  }

  if (node.type === 'VElement') {
    return {
      type: 'VElement',
      rawName: node.rawName,
      range: node.range,
      startTag: {
        range: node.startTag.range,
        selfClosing: node.startTag.selfClosing,
        attributes: node.startTag.attributes.map(comparableAttribute),
      },
      endTag: node.endTag && { range: node.endTag.range },
      children: node.children.map(comparableNode),
    }
  }

  if (node.type === 'VExpressionContainer') {
    return {
      type: 'VExpressionContainer',
      range: node.range,
      expression: {
        type: node.expression?.type,
        range: node.expression?.range,
      },
    }
  }

  if (node.type === 'VText') {
    return {
      type: 'VText',
      value: node.value,
      range: node.range,
    }
  }

  if (node.type === 'VComment') {
    return {
      type: 'VComment',
      value: node.value,
      range: node.range,
    }
  }

  return {
    type: node.type,
    range: node.range,
  }
}

function comparableAttribute(attribute: any) {
  if (attribute.type === 'VDirective') {
    return {
      directive: true,
      range: attribute.range,
      key: {
        name: attribute.key.name,
        argument: attribute.key.argument && {
          raw: attribute.key.argument.raw,
          kind: attribute.key.argument.kind,
          range: attribute.key.argument.range,
        },
        modifiers: attribute.key.modifiers.map((modifier: any) => ({
          name: modifier.name,
          range: modifier.range,
        })),
        range: attribute.key.range,
      },
      value: attribute.value && {
        range: attribute.value.range,
        expression: {
          type: attribute.value.expression?.type,
          range: attribute.value.expression?.range,
        },
      },
    }
  }

  if (attribute.directive) {
    return {
      directive: true,
      range: attribute.range,
      key: {
        name: attribute.key.name.name,
        argument: attribute.key.argument && {
          raw: attribute.key.argument.rawName,
          kind: attribute.key.argument.type === 'VExpressionContainer' ? 'dynamic' : 'static',
          range: attribute.key.argument.range,
        },
        modifiers: attribute.key.modifiers.map((modifier: any) => ({
          name: modifier.name,
          range: modifier.range,
        })),
        range: attribute.key.range,
      },
      value: attribute.value && {
        range: attribute.value.range,
        expression: {
          type: attribute.value.expression?.type,
          range: attribute.value.expression?.range,
        },
      },
    }
  }

  return {
    directive: false,
    range: attribute.range,
    key: {
      name: attribute.key.name,
      range: attribute.key.range,
    },
    value: attribute.value && {
      value: attribute.value.value,
      range: attribute.value.range,
    },
  }
}
