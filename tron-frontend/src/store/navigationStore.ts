import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

import type {
  NavigationDomainOption,
  NavigationRuntimeTree,
} from "@/shared/types/Navigation";

interface NavigationState {
  domains: NavigationDomainOption[];
  menuTree: NavigationRuntimeTree | null;
  selectedDomainCode: string;
  setDomains: (domains: NavigationDomainOption[]) => void;
  setMenuTree: (menuTree: NavigationRuntimeTree | null) => void;
  setSelectedDomainCode: (domainCode: string) => void;
}

export const useNavigationStore = create<NavigationState>()(
  persist(
    (set) => ({
      domains: [],
      menuTree: null,
      selectedDomainCode: "",
      setDomains: (domains) => set({ domains }),
      setMenuTree: (menuTree) => set({ menuTree }),
      setSelectedDomainCode: (selectedDomainCode) => set({ selectedDomainCode }),
    }),
    {
      name: "tron-navigation",
      partialize: (state) => ({ selectedDomainCode: state.selectedDomainCode }),
      storage: createJSONStorage(() => localStorage),
    },
  ),
);
