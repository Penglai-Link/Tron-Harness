<script setup lang="ts">
import { computed } from "vue";
import "github-markdown-css/github-markdown.css";
import MarkdownIt from "markdown-it";

// 初始化 markdown-it，并配置代码高亮
const md = new MarkdownIt({
  html: true,
  linkify: true,
  typographer: true,
});

const props = withDefaults(
  defineProps<{
    content?: string;
  }>(),
  {
    content: "",
  },
);

const compiledMarkdown = computed(() => md.render(props.content));
</script>

<template>
  <div class="markdown-body" v-html="compiledMarkdown" />
</template>
<style scoped></style>
