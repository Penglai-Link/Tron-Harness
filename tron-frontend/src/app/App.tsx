import { BrowserRouter } from "react-router";
import { ThemeProvider } from "@/shared";
import { TronMotionProvider } from "@/shared/motion";
import { ShortcutHintsProvider } from "@/shared/shortcuts/ShortcutHints";

import RoutedApp from "./RoutedApp";

export function App() {
  return (
    <BrowserRouter>
      <ThemeProvider>
        <TronMotionProvider>
          <ShortcutHintsProvider>
            <RoutedApp />
          </ShortcutHintsProvider>
        </TronMotionProvider>
      </ThemeProvider>
    </BrowserRouter>
  );
}
