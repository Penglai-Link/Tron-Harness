<script setup lang="ts">
import { renderMarkdownHtml } from "../utils";

const props = withDefaults(defineProps<{
  content?: string;
}>(), {
  content: "",
});
</script>

<template>
  <div
    class="tron-chat__markdown tron-chat__message-body"
    v-html="renderMarkdownHtml(props.content)"
  />
</template>
