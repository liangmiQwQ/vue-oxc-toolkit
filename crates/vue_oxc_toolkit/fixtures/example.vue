<template>
  <div class="box"> {{ count }} </div>
  <p>{{ msg }}</p>

  <div :key="item.id" v-for="item in list">
    {{ item.name }}
  </div>

  <Input v-model:[index].name="name" />
  <Component #default="{ data }"> {{ data }} </Component>

  <div> Hi There </div>
  <p class="box">Good Morning</p>
</template>

<script lang="ts">
export default {
  data() {
    return {
      count: 1
    }
  }
}
</script>

<script lang="ts" setup>
import { ref, computed, defineProps } from 'vue';

defineProps<{ msg: string }>()

const count = ref(2);
const fontSize = computed(() => (count.value * 20) + "px")

const name = ref('Liang Mi');

const index = 1;
const list = [{ name: "Tim", id: 1 }, { name: "Jack", id: 2 }]
</script>

<style>
.box {
  font-size: v-bind(fontSize);
}
</style>
