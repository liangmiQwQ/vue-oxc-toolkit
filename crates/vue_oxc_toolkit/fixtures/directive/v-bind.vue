<template>
  <div :class="'w-100'" />
  <div :[some]="2" />
  <div v-bind:["some".concat("2")]="5" />
</template>
