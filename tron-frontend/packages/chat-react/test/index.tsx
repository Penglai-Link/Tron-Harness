import { createRoot } from "react-dom/client";

import { TronChatRuntimeWindow } from "../src";
import { createOpenApiChatAdapter } from "@tron/chat-openapi-adapter";
import "@tron/chat-theme";
import "../src/styles.css";

fetch("/api/v1/auth/refresh", { method: "POST" })
  .then((res) => res.json())
  .then(({ data }) => {
    const root = document.getElementById("root");
    createRoot(root!).render(
      <TronChatRuntimeWindow
        adapter={createOpenApiChatAdapter({
          accessToken: data.accessToken,
          appKey: "LycT3LVqDxemYEhDJa9sc7JCg4qXVP-7sy8pnhKP348",
          openAppCode: "OPENAPP-0A331A39DC1F0001",
          userCode: "USR-ADMIN",
          getAccessToken: () => {
            return data.accessToken;
          },
        })}
        config={{ workspaceCode: "WS-EC0B2F07415C03F6", title: "智能助手" }}
      />,
    );
  });
