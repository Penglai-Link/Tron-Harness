import { auth } from "./auth";
import { common } from "./common";
import { consoleMessages } from "./console";
import { home } from "./home";
import { shell } from "./shell";

export const enUS = {
  auth,
  common,
  console: consoleMessages,
  home,
  shell,
} as const;
