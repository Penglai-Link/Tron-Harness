import { BrowserRouter } from "react-router";
import { ThemeProvider } from "@/shared";
import { TronMotionProvider } from "@/shared/motion";

import RoutedApp from "./RoutedApp";

export function App() {
  return (
    <BrowserRouter>
      <ThemeProvider>
        <TronMotionProvider>
          <RoutedApp />
        </TronMotionProvider>
      </ThemeProvider>
    </BrowserRouter>
  );
}
