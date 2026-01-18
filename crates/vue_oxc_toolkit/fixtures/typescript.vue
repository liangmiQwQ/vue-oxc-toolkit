<script lang="ts" setup>
const a = someFunction<{ msg: 1 }>()
</script>

<template>
  <div>{{ a.msg }}</div>
  <div> I think 1 < 2, isn't it? </div>
</template>
