import { auth } from "./auth";
import { common } from "./common";
import { consoleMessages } from "./console";
import { home } from "./home";
import { pages } from "./pages";
import { shell } from "./shell";

export const enUS = {
  auth,
  common,
  console: consoleMessages,
  home,
  pages,
  shell,
} as const;
