import { useTranslation } from "react-i18next";

import type { ServerAuthenticationType } from "@/domain/resource/api";

import {
  ConnectionSelectField,
  ConnectionSelectItem,
  ConnectionTextareaField,
  ConnectionTextField,
} from "./ConnectionFormFields";

export interface ServerAccountFormValue {
  accountCode: string;
  authenticationType: ServerAuthenticationType;
  description: string;
  name: string;
  password: string;
  privateKeyPassphrase: string;
  privateKey: string;
  username: string;
}

interface ServerAccountFieldsProps {
  idPrefix: string;
  onChange: (value: ServerAccountFormValue) => void;
  value: ServerAccountFormValue;
  credentialsRequired?: boolean;
}

export const emptyServerAccount: ServerAccountFormValue = {
  accountCode: "",
  authenticationType: "Password",
  description: "",
  name: "",
  password: "",
  privateKeyPassphrase: "",
  privateKey: "",
  username: "",
};

export function ServerAccountFields({ idPrefix, onChange, value, credentialsRequired = true }: ServerAccountFieldsProps) {
  const { t } = useTranslation("pages");
  const pageT = (key: string) => t(`resourceConnections.${key}`);
  const set = (key: keyof ServerAccountFormValue) => (next: string) => onChange({ ...value, [key]: next });

  return (
    <>
      <ConnectionTextField id={`${idPrefix}-code`} label={pageT("fields.accountCode")} onChange={set("accountCode")} readOnly required value={value.accountCode} />
      <ConnectionTextField id={`${idPrefix}-name`} label={pageT("fields.accountName")} onChange={set("name")} placeholder={pageT("placeholders.accountName")} required value={value.name} />
      <ConnectionTextField id={`${idPrefix}-username`} label={pageT("fields.username")} onChange={set("username")} placeholder="deploy" required value={value.username} />
      <ConnectionSelectField required id={`${idPrefix}-auth-type`} label={pageT("fields.authenticationType")} onValueChange={(next) => onChange({ ...value, authenticationType: next as ServerAuthenticationType, password: "", privateKey: "", privateKeyPassphrase: "" })} value={value.authenticationType}>
        <ConnectionSelectItem value="Password">{pageT("authentication.password")}</ConnectionSelectItem>
        <ConnectionSelectItem value="PrivateKey">{pageT("authentication.privateKey")}</ConnectionSelectItem>
      </ConnectionSelectField>
      {value.authenticationType === "Password" ? (
        <div className="sm:col-span-2">
          <ConnectionTextField autoComplete="new-password" description={pageT(credentialsRequired ? "serverAccount.secretHint" : "serverAccount.keepPassword")} id={`${idPrefix}-password`} label={pageT("fields.password")} onChange={set("password")} required={credentialsRequired} type="password" value={value.password} />
        </div>
      ) : (
        <>
          <div className="sm:col-span-2">
            <ConnectionTextareaField description={pageT(credentialsRequired ? "serverAccount.secretHint" : "serverAccount.keepPrivateKey")} id={`${idPrefix}-private-key`} label={pageT("serverAccount.privateKey")} onChange={set("privateKey")} required={credentialsRequired} value={value.privateKey} />
          </div>
          <div className="sm:col-span-2">
            <ConnectionTextField autoComplete="new-password" description={pageT("serverAccount.passphraseHint")} disabled={!value.privateKey} id={`${idPrefix}-passphrase`} label={pageT("serverAccount.passphrase")} onChange={set("privateKeyPassphrase")} type="password" value={value.privateKeyPassphrase} />
          </div>
        </>
      )}
      <div className="sm:col-span-2">
        <ConnectionTextareaField id={`${idPrefix}-description`} label={pageT("fields.accountDescription")} onChange={set("description")} value={value.description} />
      </div>
    </>
  );
}
