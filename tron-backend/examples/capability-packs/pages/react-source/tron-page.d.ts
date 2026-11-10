interface TronPageContext { packId: string; pageId: string; locale: string; appearance: "light" | "dark"; tokens: Record<string, string>; }
interface Window { tronPage: { readonly protocolVersion: 1; readonly context?: TronPageContext; onContext(listener: (context: TronPageContext) => void): () => void; }; }
