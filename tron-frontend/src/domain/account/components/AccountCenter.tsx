import {
  useEffect,
  useMemo,
  useState,
  type FormEvent,
  type ReactNode,
} from "react";
import {
  BadgeCheck,
  Check,
  Circle,
  Eye,
  EyeOff,
  KeyRound,
  LoaderCircle,
  LockKeyhole,
  RefreshCw,
  Save,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { SelectItem } from "@/components/ui/select";
import { useAccountCenter } from "@/domain/account/hooks";
import type {
  AccountProfile,
  UpdateAccountProfilePayload,
} from "@/domain/account/model";
import { useAuthSession } from "@/domain/auth";
import { useLocale, type SupportedLocale } from "@/i18n";
import {
  Button,
  Field,
  PageHeader,
  SelectInput,
  TextInput,
  useLocaleFormatters,
  useTheme,
  useToast,
} from "@/shared";
import { isApiError } from "@/shared/api/httpClient";
import { cn } from "@/shared/lib/utils";

const timezoneSuggestions = [
  "UTC",
  "Asia/Shanghai",
  "Asia/Tokyo",
  "Europe/London",
  "America/New_York",
  "America/Los_Angeles",
];

export function AccountCenter() {
  const { t } = useTranslation("account");
  const {
    changePassword,
    changingPassword,
    loadError,
    loading,
    profile,
    refresh,
    saveProfile,
    savingProfile,
  } = useAccountCenter();
  const { session } = useAuthSession();

  if (loading) {
    return <AccountLoading title={t("title")} />;
  }

  if (loadError || !profile) {
    return (
      <>
        <PageHeader title={t("title")} />
        <div className="flex min-h-[420px] items-center justify-center border-y border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] px-6 lg:rounded-xl lg:border">
          <div className="text-center">
            <p className="text-sm text-[var(--color-on-surface-variant)]">{t("messages.loadFailed")}</p>
            <Button className="mt-4" type="button" variant="secondary" onClick={refresh}>
              <RefreshCw size={16} aria-hidden="true" />
              {t("actions.retry")}
            </Button>
          </div>
        </div>
      </>
    );
  }

  const sourceLabel = profile.sourceType === "GitHub"
    ? t("source.github")
    : profile.sourceType === "LDAP"
      ? t("source.ldap")
      : t("source.internal");
  const providerName = session?.provider?.name ?? sourceLabel;

  return (
    <>
      <PageHeader title={t("title")} />

      <div className="overflow-hidden border-y border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] shadow-[0_16px_36px_rgba(15,23,42,0.04)] lg:rounded-xl lg:border">
        <header className="border-b border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-5 py-5 sm:px-7 lg:px-8">
          <AccountIdentity profile={profile} sourceLabel={sourceLabel} />
        </header>

        <div className="divide-y divide-[var(--color-outline-variant)]">
          <div className="px-5 py-7 sm:px-7 lg:px-8 lg:py-8">
            <div className="mx-auto max-w-4xl">
              <ProfileSection
                profile={profile}
                saving={savingProfile}
                onSave={saveProfile}
              />
            </div>
          </div>
          <div className="px-5 py-7 sm:px-7 lg:px-8 lg:py-8">
            <div className="mx-auto max-w-4xl">
              <PreferencesSection
                saving={savingProfile}
                onSaveProfile={saveProfile}
              />
            </div>
          </div>
          <div className="px-5 py-7 sm:px-7 lg:px-8 lg:py-8">
            <div className="mx-auto max-w-4xl">
              <SecuritySection
                changing={changingPassword}
                passwordChangeAllowed={profile.passwordChangeAllowed}
                passwordResetRequired={profile.passwordResetRequired}
                providerName={providerName}
                onChangePassword={changePassword}
              />
            </div>
          </div>
        </div>
      </div>
    </>
  );
}

function AccountLoading({ title }: { title: string }) {
  return (
    <>
      <PageHeader title={title} />
      <div className="min-h-[620px] animate-pulse overflow-hidden border-y border-[var(--color-outline-variant)] bg-[var(--color-elevated-surface)] lg:rounded-xl lg:border">
        <div className="flex items-center gap-4 border-b border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] p-5 sm:px-7 lg:px-8">
          <div className="size-14 rounded-full bg-[var(--color-surface-container-high)]" />
          <div>
            <div className="h-4 w-28 rounded bg-[var(--color-surface-container-high)]" />
            <div className="mt-2 h-3 w-40 rounded bg-[var(--color-surface-container)]" />
          </div>
        </div>
        <div className="space-y-5 p-5 sm:p-7 lg:p-8">
          <div className="h-6 w-44 rounded bg-[var(--color-surface-container-high)]" />
          <div className="grid gap-4 sm:grid-cols-2">
            {Array.from({ length: 6 }).map((_, index) => (
              <div className="h-16 rounded-lg bg-[var(--color-surface-container-low)]" key={index} />
            ))}
          </div>
        </div>
      </div>
    </>
  );
}

function AccountIdentity({ profile, sourceLabel }: { profile: AccountProfile; sourceLabel: string }) {
  return (
    <div className="flex min-w-0 items-center gap-4">
      <Avatar profile={profile} />
      <div className="min-w-0">
        <p className="truncate text-sm font-semibold text-[var(--color-on-surface)]">{profile.displayName}</p>
        <p className="mt-0.5 truncate text-xs text-[var(--color-on-surface-variant)]">{profile.email}</p>
        <div className="mt-2 flex flex-wrap gap-1.5">
          <span className="inline-flex rounded-full bg-[var(--color-surface-container-high)] px-2 py-0.5 text-[11px] font-medium text-[var(--color-on-surface-variant)]">
            {sourceLabel}
          </span>
          {profile.roleCodes.slice(0, 1).map((role) => (
            <span className="inline-flex rounded-full bg-[var(--color-accent-container)] px-2 py-0.5 text-[11px] font-medium text-[var(--color-on-accent-container)]" key={role}>
              {role}
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}

function Avatar({ profile }: { profile: AccountProfile }) {
  const [imageFailed, setImageFailed] = useState(false);
  const initials = useMemo(() => profile.displayName
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase())
    .join("") || "U", [profile.displayName]);

  useEffect(() => {
    setImageFailed(false);
  }, [profile.avatarUrl]);

  return (
    <div className="flex size-14 shrink-0 items-center justify-center overflow-hidden rounded-full bg-[var(--color-primary)] text-base font-semibold text-[var(--color-on-primary)] ring-4 ring-[var(--color-elevated-surface)]">
      {profile.avatarUrl && !imageFailed ? (
        <img
          alt={profile.displayName}
          className="size-full object-cover"
          src={profile.avatarUrl}
          onError={() => setImageFailed(true)}
        />
      ) : initials}
    </div>
  );
}

interface ProfileFormState {
  avatarUrl: string;
  displayName: string;
  firstName: string;
  lastName: string;
  phoneCode: string;
  phoneNumber: string;
  timezone: string;
}

type ProfileFormErrors = Partial<Record<keyof ProfileFormState, string>>;

function profileToForm(profile: AccountProfile): ProfileFormState {
  return {
    avatarUrl: profile.avatarUrl ?? "",
    displayName: profile.displayName,
    firstName: profile.firstName,
    lastName: profile.lastName,
    phoneCode: profile.phone?.countryCode ?? "",
    phoneNumber: profile.phone?.number ?? "",
    timezone: profile.timezone,
  };
}

function ProfileSection({
  profile,
  saving,
  onSave,
}: {
  profile: AccountProfile;
  saving: boolean;
  onSave: (payload: UpdateAccountProfilePayload) => Promise<AccountProfile>;
}) {
  const { t } = useTranslation("account");
  const toast = useToast();
  const { formatDateTime } = useLocaleFormatters();
  const [form, setForm] = useState<ProfileFormState>(() => profileToForm(profile));
  const [errors, setErrors] = useState<ProfileFormErrors>({});

  useEffect(() => {
    setForm(profileToForm(profile));
  }, [profile]);

  function updateField(field: keyof ProfileFormState, value: string) {
    setForm((current) => ({ ...current, [field]: value }));
    setErrors((current) => ({ ...current, [field]: undefined }));
  }

  function validate() {
    const nextErrors: ProfileFormErrors = {};
    (["displayName", "firstName", "lastName", "timezone"] as const).forEach((field) => {
      if (!form[field].trim()) nextErrors[field] = t("validation.required");
    });

    if (Boolean(form.phoneCode.trim()) !== Boolean(form.phoneNumber.trim())) {
      nextErrors.phoneNumber = t("validation.phoneIncomplete");
    }

    if (form.avatarUrl.trim()) {
      try {
        const url = new URL(form.avatarUrl.trim());
        if (url.protocol !== "http:" && url.protocol !== "https:") {
          nextErrors.avatarUrl = t("validation.invalidAvatarUrl");
        }
      } catch {
        nextErrors.avatarUrl = t("validation.invalidAvatarUrl");
      }
    }

    setErrors(nextErrors);
    return Object.keys(nextErrors).length === 0;
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!validate()) return;

    try {
      await onSave({
        avatarUrl: form.avatarUrl.trim() || null,
        displayName: form.displayName.trim(),
        firstName: form.firstName.trim(),
        lastName: form.lastName.trim(),
        phone: form.phoneCode.trim() && form.phoneNumber.trim() ? {
          countryCode: form.phoneCode.trim(),
          number: form.phoneNumber.trim(),
        } : null,
        timezone: form.timezone.trim(),
      });
      toast.success(t("messages.profileSaved"));
    } catch (error) {
      toast.error(resolveAccountError(error, t("messages.saveFailed"), {
        ACCOUNT_PROFILE_FIELD_REQUIRED: t("validation.required"),
      }));
    }
  }

  return (
    <section aria-labelledby="account-profile-heading">
      <SectionHeading
        action={(
          <span className="inline-flex items-center gap-1.5 rounded-full bg-[var(--color-success-container)] px-2.5 py-1 text-xs font-medium text-[var(--color-on-success-container)]">
            <BadgeCheck size={14} aria-hidden="true" />
            {profile.status === "Archived" ? t("profile.status.archived") : t("profile.status.active")}
          </span>
        )}
        id="account-profile-heading"
        title={t("sections.profile")}
      />

      <form onSubmit={(event) => void handleSubmit(event)}>
        <SectionLabel>{t("profile.identity")}</SectionLabel>
        <div className="grid gap-4 sm:grid-cols-2">
          <Field label={t("profile.fields.username")}>
            <TextInput disabled value={profile.username} />
          </Field>
          <Field label={t("profile.fields.email")}>
            <TextInput disabled value={profile.email} />
          </Field>
        </div>
        <p className="mt-3 text-xs text-[var(--color-on-surface-variant)]">
          {t("profile.userCode")}: <span className="font-mono">{profile.userCode}</span>
        </p>

        <div className="my-7 border-t border-[var(--color-outline-variant)]" />
        <SectionLabel>{t("profile.editable")}</SectionLabel>
        <div className="grid gap-4 sm:grid-cols-2">
          <Field error={errors.displayName} label={t("profile.fields.displayName")}>
            <TextInput
              autoComplete="name"
              value={form.displayName}
              onChange={(event) => updateField("displayName", event.target.value)}
            />
          </Field>
          <div className="hidden sm:block" />
          <Field error={errors.firstName} label={t("profile.fields.firstName")}>
            <TextInput
              autoComplete="given-name"
              value={form.firstName}
              onChange={(event) => updateField("firstName", event.target.value)}
            />
          </Field>
          <Field error={errors.lastName} label={t("profile.fields.lastName")}>
            <TextInput
              autoComplete="family-name"
              value={form.lastName}
              onChange={(event) => updateField("lastName", event.target.value)}
            />
          </Field>
          <Field error={errors.phoneCode} label={t("profile.fields.phoneCode")}>
            <TextInput
              autoComplete="tel-country-code"
              placeholder={t("profile.placeholders.phoneCode")}
              value={form.phoneCode}
              onChange={(event) => updateField("phoneCode", event.target.value)}
            />
          </Field>
          <Field error={errors.phoneNumber} label={t("profile.fields.phoneNumber")}>
            <TextInput
              autoComplete="tel-national"
              inputMode="tel"
              placeholder={t("profile.placeholders.phoneNumber")}
              value={form.phoneNumber}
              onChange={(event) => updateField("phoneNumber", event.target.value)}
            />
          </Field>
          <Field error={errors.timezone} label={t("profile.fields.timezone")}>
            <TextInput
              list="account-timezone-options"
              value={form.timezone}
              onChange={(event) => updateField("timezone", event.target.value)}
            />
            <datalist id="account-timezone-options">
              {timezoneSuggestions.map((timezone) => <option key={timezone} value={timezone} />)}
            </datalist>
          </Field>
          <Field error={errors.avatarUrl} label={t("profile.fields.avatarUrl")}>
            <TextInput
              inputMode="url"
              placeholder={t("profile.placeholders.avatarUrl")}
              value={form.avatarUrl}
              onChange={(event) => updateField("avatarUrl", event.target.value)}
            />
          </Field>
        </div>

        <div className="mt-8 flex flex-wrap items-center justify-between gap-3 border-t border-[var(--color-outline-variant)] pt-5">
          <p className="text-xs text-[var(--color-on-surface-variant)]">
            {t("profile.lastUpdated", {
              value: formatDateTime(profile.updatedAt, { dateStyle: "medium", timeStyle: "short" }),
            })}
          </p>
          <Button disabled={saving} type="submit">
            {saving ? <LoaderCircle className="animate-spin" size={16} aria-hidden="true" /> : <Save size={16} aria-hidden="true" />}
            {saving ? t("actions.updating") : t("actions.update")}
          </Button>
        </div>
      </form>
    </section>
  );
}

interface PasswordVisibility {
  confirm: boolean;
  current: boolean;
  next: boolean;
}

function SecuritySection({
  changing,
  passwordChangeAllowed,
  passwordResetRequired,
  providerName,
  onChangePassword,
}: {
  changing: boolean;
  passwordChangeAllowed: boolean;
  passwordResetRequired: boolean;
  providerName: string;
  onChangePassword: (payload: { currentPassword: string; newPassword: string }) => Promise<unknown>;
}) {
  const { t } = useTranslation("account");
  const toast = useToast();
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [formError, setFormError] = useState("");
  const [visibility, setVisibility] = useState<PasswordVisibility>({
    confirm: false,
    current: false,
    next: false,
  });
  const rules = passwordRules(newPassword);
  const allRulesPass = rules.every((rule) => rule.valid);

  if (!passwordChangeAllowed) {
    return (
      <section aria-labelledby="account-security-heading">
        <SectionHeading id="account-security-heading" title={t("sections.security")} />
        <div className="flex items-start gap-4 border-y border-[var(--color-outline-variant)] bg-[var(--color-surface-container-low)] px-4 py-5 sm:rounded-lg sm:border">
          <div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-[var(--color-surface-container-high)] text-[var(--color-on-surface)]">
            <LockKeyhole size={18} aria-hidden="true" />
          </div>
          <div className="min-w-0">
            <h3 className="text-sm font-semibold text-[var(--color-on-surface)]">{t("security.managedTitle")}</h3>
            <p className="mt-1 text-sm leading-6 text-[var(--color-on-surface-variant)]">
              {t("security.managedDescription", { provider: providerName })}
            </p>
          </div>
        </div>
      </section>
    );
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setFormError("");

    if (!allRulesPass) {
      setFormError(t("validation.passwordWeak"));
      return;
    }
    if (newPassword !== confirmPassword) {
      setFormError(t("validation.passwordMismatch"));
      return;
    }

    try {
      await onChangePassword({ currentPassword, newPassword });
      setCurrentPassword("");
      setNewPassword("");
      setConfirmPassword("");
      toast.success(t("messages.passwordChanged"));
    } catch (error) {
      const message = resolveAccountError(error, t("messages.saveFailed"), {
        ACCOUNT_CURRENT_PASSWORD_INCORRECT: t("validation.currentPasswordIncorrect"),
        ACCOUNT_PASSWORD_UNCHANGED: t("validation.passwordUnchanged"),
        ACCOUNT_PASSWORD_WEAK: t("validation.passwordWeak"),
        ACCOUNT_PASSWORD_CHANGE_UNSUPPORTED: t("validation.passwordUnsupported"),
      });
      setFormError(message);
    }
  }

  return (
    <section aria-labelledby="account-security-heading">
      <SectionHeading id="account-security-heading" title={t("sections.security")} />

      <form onSubmit={(event) => void handleSubmit(event)}>
        <div className="grid gap-7 lg:grid-cols-[minmax(0,1fr)_minmax(280px,0.8fr)] lg:gap-10">
          <div className="space-y-4">
            <Field label={t("security.currentPassword")}>
              <PasswordInput
                autoComplete="current-password"
                visible={visibility.current}
                value={currentPassword}
                onChange={setCurrentPassword}
                onToggle={() => setVisibility((current) => ({ ...current, current: !current.current }))}
              />
            </Field>
            <Field label={t("security.newPassword")}>
              <PasswordInput
                autoComplete="new-password"
                visible={visibility.next}
                value={newPassword}
                onChange={setNewPassword}
                onToggle={() => setVisibility((current) => ({ ...current, next: !current.next }))}
              />
            </Field>
            <Field label={t("security.confirmPassword")}>
              <PasswordInput
                autoComplete="new-password"
                visible={visibility.confirm}
                value={confirmPassword}
                onChange={setConfirmPassword}
                onToggle={() => setVisibility((current) => ({ ...current, confirm: !current.confirm }))}
              />
            </Field>
          </div>

          <div className="border-t border-[var(--color-outline-variant)] pt-6 lg:border-l lg:border-t-0 lg:pl-8 lg:pt-0">
            {passwordResetRequired ? (
              <div className="mb-6 flex items-start gap-3 rounded-lg bg-[var(--color-warning-container)] px-4 py-3 text-[var(--color-on-warning-container)]">
                <LockKeyhole className="mt-0.5 shrink-0" size={16} aria-hidden="true" />
                <p className="text-sm leading-5">{t("security.resetRequired")}</p>
              </div>
            ) : null}
            <p className="text-sm font-semibold text-[var(--color-on-surface)]">{t("security.requirements")}</p>
            <div className="mt-4 space-y-3">
              {rules.map((rule) => (
                <div className={cn("flex items-center gap-2.5 text-sm", rule.valid ? "text-[var(--color-success)]" : "text-[var(--color-on-surface-variant)]")} key={rule.key}>
                  {rule.valid ? <Check className="shrink-0" size={15} aria-hidden="true" /> : <Circle className="shrink-0" size={13} aria-hidden="true" />}
                  {t(`security.rules.${rule.key}`)}
                </div>
              ))}
            </div>
          </div>
        </div>

        <div className="mt-7 flex flex-wrap items-center justify-between gap-4 border-t border-[var(--color-outline-variant)] pt-5">
          <div className="min-h-5">
            {formError ? <p className="text-sm text-[var(--color-error)]" role="alert">{formError}</p> : null}
          </div>
          <Button disabled={changing || !currentPassword || !newPassword || !confirmPassword} type="submit">
            {changing ? <LoaderCircle className="animate-spin" size={16} aria-hidden="true" /> : <KeyRound size={16} aria-hidden="true" />}
            {changing ? t("actions.saving") : t("security.changePassword")}
          </Button>
        </div>
      </form>
    </section>
  );
}

function PasswordInput({
  autoComplete,
  onChange,
  onToggle,
  value,
  visible,
}: {
  autoComplete: string;
  onChange: (value: string) => void;
  onToggle: () => void;
  value: string;
  visible: boolean;
}) {
  const { t } = useTranslation("account");
  const ToggleIcon = visible ? EyeOff : Eye;
  const label = visible ? t("security.hidePassword") : t("security.showPassword");

  return (
    <div className="relative">
      <TextInput
        autoComplete={autoComplete}
        className="pr-11"
        type={visible ? "text" : "password"}
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
      <button
        aria-label={label}
        className="absolute right-1 top-1/2 flex size-8 -translate-y-1/2 items-center justify-center rounded-md text-[var(--color-on-surface-variant)] transition-colors hover:bg-[var(--color-hover-surface)] hover:text-[var(--color-on-surface)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-focus-ring)]"
        title={label}
        type="button"
        onClick={onToggle}
      >
        <ToggleIcon size={16} aria-hidden="true" />
      </button>
    </div>
  );
}

function passwordRules(password: string) {
  return [
    { key: "length", valid: password.length >= 8 },
    { key: "uppercase", valid: /[A-Z]/.test(password) },
    { key: "lowercase", valid: /[a-z]/.test(password) },
    { key: "digit", valid: /\d/.test(password) },
    { key: "symbol", valid: /[^A-Za-z0-9]/.test(password) },
  ] as const;
}

function PreferencesSection({
  saving,
  onSaveProfile,
}: {
  saving: boolean;
  onSaveProfile: (payload: UpdateAccountProfilePayload) => Promise<AccountProfile>;
}) {
  const { t } = useTranslation(["account", "common"]);
  const toast = useToast();
  const { changeLocale, currentLocale, supportedLocales } = useLocale();
  const { setTheme, theme } = useTheme();
  const [savingLocale, setSavingLocale] = useState(false);

  function changeTheme(nextTheme: typeof theme) {
    if (nextTheme === theme) return;
    setTheme(nextTheme);
    toast.success(t("account:messages.preferenceSaved"));
  }

  async function handleLocaleChange(nextLocale: SupportedLocale) {
    if (nextLocale === currentLocale) return;
    const previousLocale = currentLocale;
    setSavingLocale(true);
    await changeLocale(nextLocale);

    try {
      await onSaveProfile({ locale: nextLocale });
      toast.success(t("account:messages.preferenceSaved", { lng: nextLocale }));
    } catch {
      await changeLocale(previousLocale);
      toast.error(t("account:messages.saveFailed", { lng: previousLocale }));
    } finally {
      setSavingLocale(false);
    }
  }

  return (
    <section aria-labelledby="account-preferences-heading">
      <SectionHeading id="account-preferences-heading" title={t("account:sections.preferences")} />

      <div>
        <p className="mb-3 text-sm font-medium text-[var(--color-on-surface)]">{t("account:preferences.theme")}</p>
        <div className="grid gap-3 sm:grid-cols-2">
          <ThemeOption
            active={theme === "light"}
            label={t("account:preferences.themes.light")}
            preview="light"
            onClick={() => changeTheme("light")}
          />
          <ThemeOption
            active={theme === "matrix-dark"}
            label={t("account:preferences.themes.matrixDark")}
            preview="matrix-dark"
            onClick={() => changeTheme("matrix-dark")}
          />
        </div>
      </div>

      <div className="my-7 border-t border-[var(--color-outline-variant)]" />
      <div className="max-w-md">
        <Field label={t("account:preferences.language")}>
          <SelectInput
            disabled={savingLocale || saving}
            value={currentLocale}
            onValueChange={(value) => void handleLocaleChange(value as SupportedLocale)}
          >
            {supportedLocales.map((locale) => (
              <SelectItem key={locale.code} value={locale.code}>
                {t(locale.labelKey)}
              </SelectItem>
            ))}
          </SelectInput>
        </Field>
      </div>
    </section>
  );
}

function ThemeOption({
  active,
  label,
  onClick,
  preview,
}: {
  active: boolean;
  label: string;
  onClick: () => void;
  preview: "light" | "matrix-dark";
}) {
  const dark = preview === "matrix-dark";
  return (
    <button
      aria-pressed={active}
      className={cn(
        "group relative overflow-hidden rounded-lg border p-3 text-left transition-[border-color,background-color,transform] duration-150 active:translate-y-px",
        active
          ? "border-[var(--color-accent)] bg-[var(--color-accent-container)]"
          : "border-[var(--color-outline-variant)] bg-[var(--color-surface-container-lowest)] hover:border-[var(--color-outline)]",
      )}
      type="button"
      onClick={onClick}
    >
      <div className={cn("h-20 overflow-hidden rounded-md border", dark ? "border-[#29452f] bg-[#0d120f]" : "border-[#d8d8d8] bg-[#fbfbfb]")}>
        <div className={cn("flex h-5 items-center gap-1 border-b px-2", dark ? "border-[#29452f] bg-[#151c17]" : "border-[#e3e3e3] bg-white")}>
          <span className={cn("size-1.5 rounded-full", dark ? "bg-[#7ee787]" : "bg-[#2f5f9f]")} />
          <span className={cn("h-1 w-8 rounded", dark ? "bg-[#395a40]" : "bg-[#d7d7d7]")} />
        </div>
        <div className="grid h-[60px] grid-cols-[18px_1fr] gap-2 p-2">
          <div className={cn("rounded-sm", dark ? "bg-[#1b271e]" : "bg-[#eeeeee]")} />
          <div className="space-y-1.5 pt-0.5">
            <div className={cn("h-1.5 w-1/2 rounded", dark ? "bg-[#7ee787]" : "bg-[#242424]")} />
            <div className={cn("h-1 w-full rounded", dark ? "bg-[#2c4632]" : "bg-[#dddddd]")} />
            <div className={cn("h-1 w-4/5 rounded", dark ? "bg-[#2c4632]" : "bg-[#e6e6e6]")} />
          </div>
        </div>
      </div>
      <div className="mt-3 flex items-center justify-between gap-3">
        <span className="text-sm font-medium text-[var(--color-on-surface)]">{label}</span>
        <span className={cn("flex size-5 items-center justify-center rounded-full border", active ? "border-[var(--color-accent)] bg-[var(--color-accent)] text-[var(--color-on-accent)]" : "border-[var(--color-outline-variant)] text-transparent")}>
          <Check size={12} aria-hidden="true" />
        </span>
      </div>
    </button>
  );
}

function SectionHeading({ action, id, title }: { action?: ReactNode; id: string; title: string }) {
  return (
    <div className="mb-7 flex flex-wrap items-center justify-between gap-3">
      <h2 className="text-xl font-semibold leading-[1.3] text-[var(--color-on-surface)]" id={id}>{title}</h2>
      {action}
    </div>
  );
}

function SectionLabel({ children }: { children: ReactNode }) {
  return (
    <p className="mb-4 text-[11px] font-semibold uppercase tracking-[0.05em] text-[var(--color-on-surface-variant)]">
      {children}
    </p>
  );
}

function resolveAccountError(
  error: unknown,
  fallback: string,
  messages: Record<string, string>,
) {
  if (!isApiError(error) || !error.code) return fallback;
  return messages[error.code] ?? fallback;
}
