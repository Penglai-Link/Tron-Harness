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
  passwordVariableKey: string;
  privateKeyPassphraseVariableKey: string;
  privateKeyVariableKey: string;
  username: string;
}

interface ServerAccountFieldsProps {
  idPrefix: string;
  onChange: (value: ServerAccountFormValue) => void;
  value: ServerAccountFormValue;
}

export const emptyServerAccount: ServerAccountFormValue = {
  accountCode: "",
  authenticationType: "Password",
  description: "",
  name: "",
  passwordVariableKey: "",
  privateKeyPassphraseVariableKey: "",
  privateKeyVariableKey: "",
  username: "",
};

export function ServerAccountFields({ idPrefix, onChange, value }: ServerAccountFieldsProps) {
  const { t } = useTranslation("pages");
  const pageT = (key: string) => t(`resourceConnections.${key}`);
  const set = (key: keyof ServerAccountFormValue) => (next: string) => onChange({ ...value, [key]: next });

  return (
    <>
      <ConnectionTextField id={`${idPrefix}-code`} label={pageT("fields.accountCode")} onChange={set("accountCode")} readOnly required value={value.accountCode} />
      <ConnectionTextField id={`${idPrefix}-name`} label={pageT("fields.accountName")} onChange={set("name")} placeholder={pageT("placeholders.accountName")} required value={value.name} />
      <ConnectionTextField id={`${idPrefix}-username`} label={pageT("fields.username")} onChange={set("username")} placeholder="deploy" required value={value.username} />
      <ConnectionSelectField id={`${idPrefix}-auth-type`} label={pageT("fields.authenticationType")} onValueChange={(next) => onChange({ ...value, authenticationType: next as ServerAuthenticationType })} value={value.authenticationType}>
        <ConnectionSelectItem value="Password">{pageT("authentication.password")}</ConnectionSelectItem>
        <ConnectionSelectItem value="PrivateKey">{pageT("authentication.privateKey")}</ConnectionSelectItem>
      </ConnectionSelectField>
      {value.authenticationType === "Password" ? (
        <div className="sm:col-span-2">
          <ConnectionTextField description={pageT("hints.variableKey")} id={`${idPrefix}-password-key`} label={pageT("fields.passwordVariableKey")} onChange={set("passwordVariableKey")} required value={value.passwordVariableKey} />
        </div>
      ) : (
        <>
          <ConnectionTextField description={pageT("hints.variableKey")} id={`${idPrefix}-private-key`} label={pageT("fields.privateKeyVariableKey")} onChange={set("privateKeyVariableKey")} required value={value.privateKeyVariableKey} />
          <ConnectionTextField description={pageT("hints.optionalVariableKey")} id={`${idPrefix}-passphrase-key`} label={pageT("fields.passphraseVariableKey")} onChange={set("privateKeyPassphraseVariableKey")} value={value.privateKeyPassphraseVariableKey} />
        </>
      )}
      <div className="sm:col-span-2">
        <ConnectionTextareaField id={`${idPrefix}-description`} label={pageT("fields.accountDescription")} onChange={set("description")} value={value.description} />
      </div>
    </>
  );
}
