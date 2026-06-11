<script setup lang="ts">
import { LoaderCircle } from "lucide-vue-next";
</script>

<template>
  <section aria-label="Waiting for model" class="tron-chat__rail">
    <div class="tron-chat__waiting">
      <span class="tron-chat__waiting-line" />
      <span class="tron-chat__rail-toggle-label">
        <LoaderCircle class="animate-spin" :size="14" />
        <span>Waiting for model...</span>
      </span>
      <span class="tron-chat__waiting-line" />
    </div>
  </section>
</template>
