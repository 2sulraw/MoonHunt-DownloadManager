<template>
  <img class="language-flag" :src="flags[code]" alt="" />
</template>

<script setup lang="ts">
import enFlag from '../assets/flags/en.png'
import frFlag from '../assets/flags/fr.png'
import faFlag from '../assets/flags/fa.png'
import esFlag from '../assets/flags/es.png'
import ptFlag from '../assets/flags/pt.png'
import ruFlag from '../assets/flags/ru.png'
import jaFlag from '../assets/flags/ja.png'
import zhFlag from '../assets/flags/zh.png'
import arFlag from '../assets/flags/ar.png'

defineProps<{ code: string }>()

const flags: Record<string, string> = {
  en: enFlag,
  fr: frFlag,
  fa: faFlag,
  es: esFlag,
  pt: ptFlag,
  ru: ruFlag,
  ja: jaFlag,
  zh: zhFlag,
  ar: arFlag,
}
</script>
