import { defineConfig } from 'vite-plus'

export default defineConfig({
  pack: {
    fixedExtension: true,
    platform: 'node',
    entry: {
      'vue-oxlint-toolkit': './js/index.ts',
    },
    dts: true,
  },
})
