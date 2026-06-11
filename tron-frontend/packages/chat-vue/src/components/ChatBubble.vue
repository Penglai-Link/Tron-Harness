<script setup lang="ts">
import { Bot, Pencil, UserRound } from "lucide-vue-next";
import { computed } from "vue";
import { formatTime } from "@tron/chat-core";
import type { ChatMessage } from "@tron/chat-core";
import { cx } from "../utils";
import MarkdownBlock from "./MarkdownBlock.vue";
import CopyValueButton from "./CopyValueButton.vue";

const props = defineProps<{
  currentUserDisplayName?: string;
  message: ChatMessage;
}>();

const emit = defineEmits<{
  editMessage: [content: string];
}>();

const isUser = computed(() => props.message.role === "USER");
const actorName = computed(() =>
  isUser.value ? (props.currentUserDisplayName || props.message.actor) : props.message.actor
);
</script>

<template>
  <article
    :class="cx('tron-chat__message', isUser && 'tron-chat__message--user')"
  >
    <div class="tron-chat__message-header">
      <div class="tron-chat__message-actor">
        <UserRound v-if="isUser" :size="16" />
        <Bot v-else :size="16" />
        <span>{{ actorName }}</span>
      </div>
      <div class="tron-chat__message-actions">
        <CopyValueButton
          aria-label="Copy raw markdown"
          title="Copy raw markdown"
          :value="props.message.content"
        />
        <button
          v-if="isUser"
          aria-label="Edit message"
          class="tron-chat__icon-button"
          type="button"
          @click="emit('editMessage', props.message.content)"
        >
          <Pencil :size="14" />
        </button>
        <span class="tron-chat__message-time">{{ formatTime(props.message.createdAt) }}</span>
      </div>
    </div>
    <div v-if="isUser" class="tron-chat__message-body">
      {{ props.message.content }}
    </div>
    <MarkdownBlock v-else :content="props.message.content" />
  </article>
</template>
