import { create } from "zustand";

interface ProfileAgreementState {
  checked: boolean;
  setChecked: (checked: boolean) => void;
}

// Consent lasts only for this app session; opening the sign-in page resets it.
export const useProfileAgreementStore = create<ProfileAgreementState>((set) => ({
  checked: false,
  setChecked: (checked) => set({ checked }),
}));
