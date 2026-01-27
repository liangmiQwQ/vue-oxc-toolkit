<template>
  <div :class="'w-100'" />
  <div :[some]="2" />
  <Some #default="{ a }" />
  <input v-model="text" />
  <!-- <div v-bind:["some".concat("2")]="5" /> -->
</template>
