import React from "react";
import { createRoot } from "react-dom/client";
function Page() { const [context, setContext] = React.useState(window.tronPage.context); React.useEffect(() => window.tronPage.onContext(setContext), []); return <main><h1>Tron</h1><pre>{JSON.stringify(context, null, 2)}</pre></main>; }
createRoot(document.getElementById("root")!).render(<Page />);
