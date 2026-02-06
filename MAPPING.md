# Vue-Oxc-Toolkit Node Mapping

This document explains how Vue template nodes are transformed into [Oxc](https://github.com/oxc-project/oxc) AST nodes. The toolkit represents Vue templates using a standard JavaScript/JSX AST, enabling use of existing JavaScript tooling.

## SFC Structure

A Vue Single File Component (SFC) is transformed into a standard `Program`.

- **`<script>`**: Parsed as standard JavaScript/TypeScript statements in the top-level scope.
- **`<script setup>`**: Parsed as JavaScript statements, move the `import` statements to the top-level scope, and keep the rest of the code in a `setup` function in the `export default` object.
- **`<template>`**: Transformed into a `JSXFragment` returned by `setup` function mentioned above.
- **`<style>`**: Treated as a normal element containing raw text.

### Example

```vue
<script>
export default {
  data() {
    return {
      count: 0
    };
  }
}
</script>

<script setup>
import { ref } from 'vue';

const count = ref(0);
</script>

<template>
  <div>{{ count }}</div>
</template>
```

```jsx
import { ref } from 'vue';

export default {
  ...{ data() {
    return {
      count: 0
    };
  } },
  setup() {
    const count = ref(0);
    return <>
      <script></script>
      <script setup></script>
      <template>
        <div>{ count }</div>
      </template>
    </>;
  }
}
```

## Elements and Components

Vue elements are mapped to `JSXElement` or `JSXFragment`.

- **HTML Elements** (`<div>`): Mapped to `JSXOpeningElement` with a lowercased `JSXIdentifier`.
- **Components** (`<MyComponent />`): Mapped to `JSXOpeningElement` with a `JSXIdentifierReference`.
- **Namespaced Components** (`<motion.div />`): Mapped to `JSXOpeningElement` with a `JSXMemberExpression`.
- **Kebab-case Components** (`<my-component />`): Transformed to PascalCase (`MyComponent`) as a `JSXIdentifierReference`.

## Attributes and Directives

Attributes are mapped to `JSXAttributeItem`.

- **Static Attributes** (`class="foo"`): Mapped to `JSXAttribute` with a `StringLiteral` value.
- **Directives** (`v-bind`, `v-on`, `v-slot`): Mapped to `JSXAttribute` where the name is a `JSXNamespacedName`.
  - **Namespace**: The directive type (e.g., `v-bind`, `v-on`, `v-slot`).
  - **Name**: The argument (e.g., `class`, `click`).
  - **Shorthands**: Normalized to full names (`:` -> `v-bind`, `@` -> `v-on`, `#` -> `v-slot`).
  - **Value**: Wrapped in `JSXExpressionContainer`. If the directive only has a name (e.g., `v-else`), use `None`.

### Dynamic Arguments

Dynamic arguments (e.g., `:[arg]="val"`) are wrapped in brackets within the `JSXNamespacedName` or handled via `ObjectExpression` when transformed.

## Structural Transformations

Some directives require structural changes to represent Vue's logic in JSX.

### `v-if` / `v-else-if` / `v-else`

Conditional chains are transformed into nested `ConditionalExpression` nodes wrapping the elements.

### Example

```vue
<div v-if="ok" />
<p v-else />
```

The parent's children will contain a `JSXExpressionContainer` with a ternary operator: `ok ? <div v-if:={}/> : <p v-else:/>`.

---

### `v-for`

Transformed into a `CallExpression` on the data source, wrapping the element in an `ArrowFunctionExpression`.

### Example

```vue
<div v-for="item in items" :key="item.id" />
```

- The list is wrapped in `items(item => <div />)`.
- The element inside the arrow function body retains the `v-for` attribute (with `JSXEmptyExpression`) to keep the source mapping.

---

### `v-slot`

Slots are collected into an `ObjectExpression` within the `children` of a component. Each property in the object represents a slot.

### Example

```vue
<Comp>
  <template #header="{ message }">
    {{ message }}
  </template>

  <template #[id]="{ message }">
    {{ message }}
  </template>
</Comp>
```

The `Comp` element's children contain a `JSXExpressionContainer` holding an `ObjectExpression`:

```jsx
<template>{{
  header: ({ message }) => <>{ message }</>
}}<template>

<template>{{
  [id]: ({ message }) => <>{ message }</>
}}</template>
```

## Text and Interpolation

- **Plain Text**: Mapped to `JSXText`.
- **Interpolation** (`{{ msg }}`): Mapped to `JSXExpressionContainer` containing the JavaScript expression.

## Comments

Template comments are captured as AST comments. They are represented by empty `JSXExpressionContainer` nodes to maintain their relative position in the tree.

Comments in JavaScript will be just treated as normal comments, collecting them in the `Program`.
