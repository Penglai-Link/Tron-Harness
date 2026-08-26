<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import type { ChatMessageAttachment } from "@tron/chat-core";

const props = defineProps<{
  attachments?: ChatMessageAttachment[];
  downloadAttachment?: (attachment: ChatMessageAttachment) => Promise<Blob>;
}>();

const emit = defineEmits<{
  previewImage: [source: string, alt: string];
}>();

const imageSources = ref<Record<string, string>>({});
const objectUrls = new Map<string, string>();

const imageAttachments = computed(() => (props.attachments ?? []).filter((attachment) => {
  const contentType = attachment.contentType?.toLowerCase();
  return attachment.kind === "image"
    || Boolean(attachment.imageUrl)
    || Boolean(contentType?.startsWith("image/"));
}));

function revokeSources() {
  objectUrls.forEach((source) => URL.revokeObjectURL(source));
  objectUrls.clear();
  imageSources.value = {};
}

async function loadSources(attachments: ChatMessageAttachment[]) {
  revokeSources();
  const nextSources: Record<string, string> = {};
  await Promise.all(attachments.map(async (attachment) => {
    if (attachment.imageUrl) {
      nextSources[attachment.id] = attachment.imageUrl;
      return;
    }
    if (!props.downloadAttachment) return;
    try {
      const blob = await props.downloadAttachment(attachment);
      const source = URL.createObjectURL(blob);
      objectUrls.set(attachment.id, source);
      nextSources[attachment.id] = source;
    } catch {
      // The resource can have been deleted together with its chat session.
    }
  }));
  imageSources.value = nextSources;
}

watch(
  [imageAttachments, () => props.downloadAttachment],
  ([attachments]) => { void loadSources(attachments); },
  { immediate: true },
);

onBeforeUnmount(revokeSources);
</script>

<template>
  <section
    v-if="imageAttachments.length"
    aria-label="Image attachments"
    class="tron-chat__inspection-attachments"
  >
    <div class="tron-chat__inspection-previews">
      <figure
        v-for="attachment in imageAttachments"
        v-show="imageSources[attachment.id]"
        :key="attachment.id"
        class="tron-chat__inspection-preview"
      >
        <button
          :aria-label="`Preview ${attachment.summary}`"
          class="tron-chat__inspection-preview-button"
          type="button"
          @click="emit('previewImage', imageSources[attachment.id], attachment.summary)"
        >
          <img :alt="attachment.summary" :src="imageSources[attachment.id]" />
        </button>
      </figure>
    </div>
  </section>
</template>
