import { account } from "./account";
import { auth } from "./auth";
import { common } from "./common";
import { consoleMessages } from "./console";
import { home } from "./home";
import { pages } from "./pages";
import { shell } from "./shell";
import { setup } from "./setup";

export const zhCN = {
  account,
  auth,
  common,
  console: consoleMessages,
  home,
  pages,
  shell,
  setup,
} as const;
