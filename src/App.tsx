import { ArrowRight, LayoutDashboard } from "lucide-react";

import { Button } from "@/components/ui/button";
import { DesignStandardPage } from "@/pages/design-standard";

function HomePage() {
  return (
    <main className="min-h-screen bg-[var(--color-background)] px-6 py-8 text-[var(--color-on-surface)]">
      <section className="mx-auto flex min-h-[calc(100vh-4rem)] max-w-6xl flex-col justify-center gap-8">
        <div className="max-w-3xl">
          <div className="mb-4 inline-flex items-center gap-2 rounded-full border border-[var(--color-outline-variant)] bg-white px-3 py-1 text-sm text-[var(--color-on-surface-variant)] shadow-sm">
            <LayoutDashboard size={16} aria-hidden="true" />
            Tron frontend workspace
          </div>
          <h1 className="text-4xl font-semibold leading-tight tracking-normal text-[var(--color-on-surface)]">
            Enterprise agent governance console
          </h1>
          <p className="mt-4 max-w-2xl text-base leading-7 text-[var(--color-on-surface-variant)]">
            This Vite workspace is initialized for the Tron onboarding flow. The first routable artifact is the design-standard preview extracted from the supplied design docs.
          </p>
          <div className="mt-8 flex flex-wrap gap-3">
            <Button asChild>
              <a href="/design-standard">
                Open design standard
                <ArrowRight size={16} aria-hidden="true" />
              </a>
            </Button>
          </div>
        </div>
      </section>
    </main>
  );
}

export function App() {
  const path = window.location.pathname;

  if (path === "/design-standard") {
    return <DesignStandardPage />;
  }

  return <HomePage />;
}
