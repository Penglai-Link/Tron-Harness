import { createApp } from "vue";
import { TronChatRuntimeWindow } from "../src";
import { createOpenApiChatAdapter } from "@tron/chat-openapi-adapter";
import "@tron/chat-theme";
import "../src/styles.css";

fetch("/api/v1/auth/refresh", { method: "POST" })
  .then((res) => res.json())
  .then(({ data }) => {
    createApp(TronChatRuntimeWindow, {
      adapter: createOpenApiChatAdapter({
        openAppCode: "",
        appKey: "",
        userCode: "",
        getAccessToken: () => {
          return data.accessToken;
        },
      }),
      config: {
        workspaceCode: "WS-EC0B2F07415C03F6",
      },
    }).mount("#app");
  });
