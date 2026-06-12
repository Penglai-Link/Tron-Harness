import { createRoot } from "react-dom/client";

import { TronChatRuntimeWindow } from "../src";
import { createInternalChatAdapter } from "@tron/chat-internal-adapter";
import "@tron/chat-theme";
import "../src/styles.css";

fetch("/api/v1/auth/refresh", { method: "POST" })
  .then((res) => res.json())
  .then(({ data }) => {
    const root = document.getElementById("root");
    createRoot(root!).render(
      <TronChatRuntimeWindow
        adapter={createInternalChatAdapter({
          getAccessToken: () => {
            return data.accessToken;
          },
        })}
        config={{ workspaceCode: "WS-EC0B2F07415C03F6" }}
      />,
    );
  });
