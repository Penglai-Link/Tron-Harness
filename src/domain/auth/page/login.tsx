import { useState } from "react";
import {
  RefreshCw,
  Sparkles,
} from "lucide-react";

import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Field,
  TextInput,
} from "@/components/tron";
import { useNavigate } from "@/router/client-router";

export function LoginPage() {
  const { t } = useTranslation(["auth", "common"]);
  const [captcha, setCaptcha] = useState("K7Q4");
  const [status, setStatus] = useState(() => t("auth:status"));
  const navigate = useNavigate();

  return (
    <main className="flex min-h-screen items-center justify-center bg-[var(--color-background)] px-6 py-10 text-[var(--color-on-surface)]">
      <section className="w-full max-w-md rounded-xl border border-[var(--color-outline-variant)] bg-white p-10 shadow-sm">
        <div className="mb-8 text-center">
          <div className="mx-auto mb-4 flex size-12 items-center justify-center rounded-xl bg-[var(--color-primary)] text-white">
            <Sparkles size={22} aria-hidden="true" />
          </div>
          <h1 className="text-2xl font-semibold">{t("auth:title")}</h1>
          <p className="mt-2 text-sm text-[var(--color-on-surface-variant)]">{t("auth:subtitle")}</p>
        </div>
        <form className="space-y-5" onSubmit={(event) => {
          event.preventDefault();
          setStatus(t("auth:signingIn"));
          navigate("/dashboard");
        }}>
          <Field label={t("auth:fields.operatorId")}>
            <TextInput placeholder="TRON-XXXX" />
          </Field>
          <Field label={t("auth:fields.passkey")}>
            <TextInput type="password" placeholder={t("auth:passkeyPlaceholder")} />
          </Field>
          <div>
            <div className="mb-2 flex items-center justify-between">
              <span className="text-[11px] font-semibold uppercase text-[var(--color-on-surface-variant)]">{t("auth:captcha.label")}</span>
              <Button size="sm" variant="ghost" type="button" onClick={() => setCaptcha((current) => (current === "K7Q4" ? "M2R8" : "K7Q4"))}>
                <RefreshCw size={14} aria-hidden="true" />
                {t("common:actions.refresh")}
              </Button>
            </div>
            <div className="flex gap-3">
              <div className="flex w-1/3 items-center justify-center rounded-lg border border-[var(--color-outline-variant)] bg-[var(--color-surface-container)] font-mono text-lg font-semibold tracking-[0.2em]">{captcha}</div>
              <TextInput className="w-2/3" placeholder={t("auth:captcha.placeholder")} />
            </div>
          </div>
          <Button className="w-full" type="submit">{t("common:actions.signIn")}</Button>
          <div className="flex justify-between text-[13px] text-[var(--color-on-surface-variant)]">
            <button type="button" onClick={() => setStatus(t("auth:recoveryRequested"))}>{t("auth:forgotPasskey")}</button>
            <span>{status}</span>
          </div>
        </form>
        <p className="mt-10 text-center text-xs text-[var(--color-on-surface-variant)]">{t("auth:copyright")}</p>
      </section>
    </main>
  );
}
