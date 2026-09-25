import { Component, type ErrorInfo, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";

function RenderErrorFallback({ fullScreen }: { fullScreen?: boolean }) {
  const { t } = useTranslation("common");
  return (
    <section role="alert" className={`flex flex-col items-center justify-center gap-3 bg-background p-6 text-foreground ${fullScreen ? "min-h-screen" : "min-h-64"}`}>
      <h2 className="text-base font-semibold">{t("renderError.title")}</h2>
      <p className="max-w-md text-center text-sm text-muted-foreground">{t("renderError.description")}</p>
      <Button variant="outline" onClick={() => window.location.reload()}>{t("actions.refresh")}</Button>
    </section>
  );
}

/** Keep a failed lazy page from unmounting the shell and other open tabs. */
export class RenderErrorBoundary extends Component<{
  children: ReactNode;
  resetKey?: string;
  fullScreen?: boolean;
}, { failed: boolean }> {
  state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[Tron] Render failed", error, info.componentStack);
  }

  componentDidUpdate(previous: Readonly<typeof this.props>) {
    if (this.state.failed && previous.resetKey !== this.props.resetKey) {
      this.setState({ failed: false });
    }
  }

  render() {
    return this.state.failed
      ? <RenderErrorFallback fullScreen={this.props.fullScreen} />
      : this.props.children;
  }
}
