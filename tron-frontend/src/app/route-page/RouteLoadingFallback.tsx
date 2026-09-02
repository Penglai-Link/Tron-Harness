export default function RouteLoadingFallback({
  fullScreen = false,
}: {
  fullScreen?: boolean;
}) {
  return (
    <section
      aria-busy="true"
      className={
        fullScreen
          ? "flex min-h-screen items-center justify-center bg-[var(--color-background)] px-6"
          : "flex min-h-[calc(100vh-7rem)] items-center justify-center"
      }
    >
      <div className="h-1.5 w-32 overflow-hidden rounded-full bg-[var(--color-surface-container-high)]">
        <div className="h-full w-1/2 animate-pulse rounded-full bg-[var(--color-primary)]" />
      </div>
    </section>
  );
}
