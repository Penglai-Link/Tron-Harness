import { BrowserRouter } from "react-router";
import { ThemeProvider } from "@/shared";

import RoutedApp from "./RoutedApp";

export function App() {
  return (
    <BrowserRouter>
      <ThemeProvider>
        <RoutedApp />
      </ThemeProvider>
    </BrowserRouter>
  );
}
