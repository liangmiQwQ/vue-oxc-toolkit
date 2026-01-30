<template>
  <Comp #="{ message }" />
  <Comp #header="{ message }">
    {{ message }}
  </Comp>
  <Comp v-slot="{ message }" />
  <Comp v-slot:header="{ message }">
    {{ message }}
  </Comp>
  <Comp v-slot:="{ message }" />
  <Comp v-slot:[key]="{ message }">
    {{ message }}
  </Comp>
</template>
