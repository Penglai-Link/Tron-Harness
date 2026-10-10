import { observability } from "./observability";
import { common } from "./common";
import { consoleMessages } from "./console";
import { pages } from "./pages";
import { shell } from "./shell";

export const enUS = {
  observability,
  common,
  console: consoleMessages,
  pages,
  shell,
} as const;
