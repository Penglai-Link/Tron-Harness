import { createContext, useContext } from "react";

const VisibilityContext = createContext(true);

export function TabVisibilityProvider({
  children,
  visible,
}: {
  children: React.ReactNode;
  visible: boolean;
}) {
  const parentVisible = useContext(VisibilityContext);

  return (
    <VisibilityContext.Provider value={parentVisible && visible}>
      {children}
    </VisibilityContext.Provider>
  );
}

export function useTabVisibility() {
  return useContext(VisibilityContext);
}
