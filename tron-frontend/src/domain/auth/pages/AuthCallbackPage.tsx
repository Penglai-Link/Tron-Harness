import {
  startTransition,
  useEffect,
} from "react";
import {
  ArrowRight,
  Orbit,
} from "lucide-react";

import { useTranslation } from "react-i18next";
import {
  useLocation,
  useNavigate,
} from "react-router";

import { buildLoginUrl, resolveAuthRedirect } from "@/domain/auth/model";
import { Button } from "@/shared/ui/Button";
import { useAuthSession } from "@/domain/auth/hooks";

export function AuthCallbackPage() {
  const { t } = useTranslation("auth");
  const location = useLocation();
  const navigate = useNavigate();
  const { status } = useAuthSession();
  const redirectTarget = resolveAuthRedirect(location.search);

  useEffect(() => {
    if (status !== "authenticated") return;

    startTransition(() => {
      navigate(redirectTarget, { replace: true });
    });
  }, [navigate, redirectTarget, status]);

  return (
    <main className="flex min-h-screen items-center justify-center bg-[radial-gradient(circle_at_top,_rgba(188,220,255,0.28),_transparent_42%),linear-gradient(180deg,_#f7fafc_0%,_#eef3f8_100%)] px-6 py-10 text-[var(--color-on-surface)]">
      <section className="w-full max-w-lg rounded-[28px] border border-white/80 bg-white p-8 text-center shadow-[0_28px_80px_rgba(42,71,110,0.14)]">
        <div className="mx-auto flex size-14 items-center justify-center rounded-2xl bg-slate-950 text-white">
          <Orbit size={22} aria-hidden="true" />
        </div>
        <h1 className="mt-6 text-2xl font-semibold text-slate-950">{t("callback.title")}</h1>
        <p className="mt-3 text-sm leading-6 text-[var(--color-on-surface-variant)]">
          {status === "checking" ? t("callback.progress") : t("callback.failed")}
        </p>

        {status === "unauthenticated" ? (
          <Button className="mt-6" type="button" variant="secondary" onClick={() => navigate(buildLoginUrl(redirectTarget), { replace: true })}>
            {t("callback.backToLogin")}
            <ArrowRight size={16} aria-hidden="true" />
          </Button>
        ) : null}
      </section>
    </main>
  );
}
