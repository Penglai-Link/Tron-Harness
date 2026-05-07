import {
  clsx,
  type ClassValue,
} from "clsx";

import { twMerge } from "tailwind-merge";

/**
 * Combines class names using clsx and merges them with Tailwind CSS.
 * @param inputs An array of class names or class name objects to be combined and merged.
 * @returns A single string of combined and merged class names.
 */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
