<script setup>
// Parentheses in binary expressions should be preserved
const a = (1 + 2) * 3
const b = x ?? (y || z)

// Numeric literal raw forms should be preserved
const hex = 0xFF
const octal = 0o77
const binary = 0b1010
const separated = 1_000_000

// BigInt raw forms should be preserved
const bigHex = 0x10n
const bigSep = 1_000n

// Shorthand should only be used when explicitly written
const foo = 1
const obj1 = { foo }        // shorthand: true
const obj2 = { foo: foo }   // shorthand: false

// Destructuring shorthand
const { foo: bar } = obj2   // shorthand: false
const { foo: foo2 } = obj2  // shorthand: false

// Arrow function params should always have parens
const f1 = (x) => x
const f2 = x => x

// new expression should always have ()
const inst = new Foo()
</script>

<template>
  <div>{{ a }}</div>
</template>
