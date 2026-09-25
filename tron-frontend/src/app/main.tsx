import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { AnchoredToastProvider, ToastProvider } from "@/components/ui/toast"
import { TooltipProvider } from "@/components/ui/tooltip";
import { QueryProvider } from "./providers/QueryProvider";
import { App } from "./App";
import "@/i18n";
import "@tron/chat-theme";
import "./styles/globals.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error("Tron root element was not found.");
}

root.dataset.tronApp = "";
root.dataset.themePart = "app-shell";
document.body.dataset.tronApp = "";

createRoot(root).render(
  <StrictMode>
    <ToastProvider position="top-center">
      <AnchoredToastProvider>
        <TooltipProvider delay={0}>
          <QueryProvider>
            <App />
          </QueryProvider>
        </TooltipProvider>
      </AnchoredToastProvider>
    </ToastProvider>
  </StrictMode>,
);
