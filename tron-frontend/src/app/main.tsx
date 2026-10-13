import { installBrowserTracing } from "@tron/chat-core";
import { buildApiUrl } from "@/shared/api/httpClient";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { AnchoredToastProvider, ToastProvider } from "@/components/ui/toast"
import { TooltipProvider } from "@/components/ui/tooltip";
import { QueryProvider } from "./providers/QueryProvider";
import { App } from "./App";
import { RenderErrorBoundary } from "./RenderErrorBoundary";
import "@/i18n";
import "@eliteforge/agent-ui-chat-theme";
import "./styles/globals.css";

installBrowserTracing(buildApiUrl("/api/v1/observability/browser-spans"));

const root = document.getElementById("root");

if (!root) {
  throw new Error("Tron root element was not found.");
}

root.dataset.tronApp = "";
root.dataset.themePart = "app-shell";
document.body.dataset.tronApp = "";

createRoot(root).render(
  <StrictMode>
    <RenderErrorBoundary fullScreen>
    <ToastProvider position="top-center">
      <AnchoredToastProvider>
        <TooltipProvider>
          <QueryProvider>
            <App />
          </QueryProvider>
        </TooltipProvider>
      </AnchoredToastProvider>
    </ToastProvider>
    </RenderErrorBoundary>
  </StrictMode>,
);
