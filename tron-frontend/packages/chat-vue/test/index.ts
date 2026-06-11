import { createApp } from "vue";
import { TronChatRuntimeWindow } from "../src";
import { createInternalChatAdapter } from "@tron/chat-internal-adapter";
import "@tron/chat-theme";

fetch("/api/v1/auth/refresh", { method: "POST" })
  .then((res) => res.json())
  .then(({ data }) => {
    createApp(TronChatRuntimeWindow, {
      adapter: createInternalChatAdapter({
        getAccessToken: () => {
          return data.accessToken;
        },
      }),
      config: {
        workspaceCode: "WS-EC0B2F07415C03F6",
      },
    }).mount("#app");
  });
