import {
  useEffect,
  useState,
  type FormEvent,
} from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Spinner } from "@/components/ui/spinner";
import { toastManager } from "@/components/ui/toast";
import {
  createDataSourceConnection,
  type DataSourceKindCode,
} from "@/domain/resource/api";
import { createMockManagedResource } from "@/domain/resource/mock";
import { getServiceDataSource } from "@/shared/api/serviceConfig";

import {
  ConnectionSelectField,
  ConnectionSelectItem,
  ConnectionTextareaField,
  ConnectionTextField,
} from "./ConnectionFormFields";
import { generateConnectionResourceCode } from "./connectionResourceCode";

interface DataSourceCreateDialogProps {
  onCreated: () => void;
  onOpenChange: (open: boolean) => void;
  open: boolean;
  workspaceCode: string;
}

interface EndpointFormValue {
  host: string;
  port: string;
  role: "PRIMARY" | "REPLICA";
}

type PasswordSource = "DIRECT" | "VARIABLE";

const DEFAULT_PORTS: Record<DataSourceKindCode, string> = {
  KAFKA_CONNECTION: "9092",
  MYSQL_CONNECTION: "3306",
  POSTGRESQL_CONNECTION: "5432",
  REDIS_CONNECTION: "6379",
};

function createTls(mode: string, caCertificate: string) {
  return {
    mode,
    ...(caCertificate.trim() ? { caCertificate: caCertificate.trim() } : {}),
  };
}

function endpointValue(port: string): EndpointFormValue {
  return { host: "", port, role: "REPLICA" };
}

export function DataSourceCreateDialog({
  onCreated,
  onOpenChange,
  open,
  workspaceCode,
}: DataSourceCreateDialogProps) {
  const { t } = useTranslation("pages");
  const [kind, setKind] = useState<DataSourceKindCode>("MYSQL_CONNECTION");
  const [resourceCode, setResourceCode] = useState("");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [connectionMode, setConnectionMode] = useState("SINGLE");
  const [endpoints, setEndpoints] = useState<EndpointFormValue[]>([endpointValue(DEFAULT_PORTS.MYSQL_CONNECTION)]);
  const [database, setDatabase] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [passwordSource, setPasswordSource] = useState<PasswordSource>("DIRECT");
  const [tlsMode, setTlsMode] = useState("DISABLED");
  const [caCertificateVariableKey, setCaCertificateVariableKey] = useState("");
  const [securityProtocol, setSecurityProtocol] = useState("PLAINTEXT");
  const [saslMechanism, setSaslMechanism] = useState("PLAIN");
  const [redisTopology, setRedisTopology] = useState("STANDALONE");
  const [sentinelMasterName, setSentinelMasterName] = useState("");
  const [postgresqlLoadBalance, setPostgresqlLoadBalance] = useState("DISABLED");
  const [postgresqlTargetSession, setPostgresqlTargetSession] = useState("ANY");
  const [submitting, setSubmitting] = useState(false);

  const pageT = (key: string) => t(`resourceConnections.${key}`);
  const isKafka = kind === "KAFKA_CONNECTION";
  const isRedis = kind === "REDIS_CONNECTION";
  const usesSasl = isKafka && securityProtocol.startsWith("SASL_");
  const usesKafkaTls = isKafka && securityProtocol.endsWith("SSL");
  const isMysqlReplication = kind === "MYSQL_CONNECTION" && connectionMode === "REPLICATION";
  const minimumEndpointCount = isMysqlReplication ? 2 : 1;
  const supportsMultipleEndpoints = isKafka
    || kind === "POSTGRESQL_CONNECTION"
    || (kind === "MYSQL_CONNECTION" && connectionMode !== "SINGLE")
    || (isRedis && redisTopology !== "STANDALONE");

  useEffect(() => {
    setEndpoints([endpointValue(DEFAULT_PORTS[kind])]);
    setConnectionMode("SINGLE");
    setPostgresqlLoadBalance("DISABLED");
    setPostgresqlTargetSession("ANY");
    setRedisTopology("STANDALONE");
    setSentinelMasterName("");
    setTlsMode(kind === "KAFKA_CONNECTION" ? "REQUIRED" : "DISABLED");
  }, [kind]);

  useEffect(() => {
    if (open) setResourceCode(generateConnectionResourceCode("DATA_SOURCE"));
  }, [open]);

  function attributes(): Record<string, unknown> {
    const connectionEndpoints = endpoints.map(({ host, port, role }) => ({
      host: host.trim(),
      port: Number(port),
      ...(isMysqlReplication ? { role } : {}),
    }));
    if (kind === "MYSQL_CONNECTION") {
      return {
        connectionMode,
        credentials: { username: username.trim(), password: passwordReference() },
        database: database.trim(),
        endpoints: connectionEndpoints,
        tls: createTls(tlsMode, caCertificateVariableKey),
      };
    }
    if (kind === "POSTGRESQL_CONNECTION") {
      return {
        credentials: { username: username.trim(), password: passwordReference() },
        database: database.trim(),
        endpoints: connectionEndpoints,
        loadBalance: postgresqlLoadBalance,
        targetSession: postgresqlTargetSession,
        tls: createTls(tlsMode, caCertificateVariableKey),
      };
    }
    if (kind === "REDIS_CONNECTION") {
      const credentials = password.trim()
        ? {
            password: passwordReference(),
            ...(username.trim() ? { username: username.trim() } : {}),
          }
        : undefined;
      return {
        ...(credentials ? { credentials } : {}),
        database: Number(database || "0"),
        endpoints: connectionEndpoints,
        tls: createTls(tlsMode, caCertificateVariableKey),
        topology: redisTopology,
        ...(redisTopology === "SENTINEL" ? {
          sentinel: {
            masterName: sentinelMasterName.trim(),
            ...(credentials ? { credentials } : {}),
            tls: createTls(tlsMode, caCertificateVariableKey),
          },
        } : {}),
      };
    }
    return {
      brokers: connectionEndpoints,
      securityProtocol,
      ...(usesSasl ? {
        sasl: {
          mechanism: saslMechanism,
          password: passwordReference(),
          username: username.trim(),
        },
      } : {}),
      ...(usesKafkaTls ? { tls: createTls(tlsMode, caCertificateVariableKey) } : {}),
    };
  }

  function passwordReference() {
    return passwordSource === "DIRECT"
      ? { source: "DIRECT", value: password }
      : { source: "VARIABLE", variableKey: password.trim() };
  }

  function updateEndpoint(index: number, key: keyof EndpointFormValue, value: string) {
    setEndpoints((current) => current.map((endpoint, endpointIndex) => (
      endpointIndex === index ? { ...endpoint, [key]: value } : endpoint
    )));
  }

  function updateConnectionMode(value: string) {
    setConnectionMode(value);
    if (value === "SINGLE") {
      setEndpoints((current) => current.slice(0, 1));
      return;
    }
    if (value !== "REPLICATION") return;
    setEndpoints((current) => {
      const nodes = current.length >= 2
        ? current
        : [...current, endpointValue(DEFAULT_PORTS.MYSQL_CONNECTION)];
      return nodes.map((endpoint, index) => ({
        ...endpoint,
        role: index === 0 ? "PRIMARY" : "REPLICA",
      }));
    });
  }

  function updateRedisTopology(value: string) {
    setRedisTopology(value);
    if (value === "STANDALONE") setEndpoints((current) => current.slice(0, 1));
  }

  function addEndpoint() {
    setEndpoints((current) => [...current, endpointValue(DEFAULT_PORTS[kind])]);
  }

  function removeEndpoint(index: number) {
    setEndpoints((current) => current.length <= minimumEndpointCount
      ? current
      : current.filter((_, endpointIndex) => endpointIndex !== index));
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSubmitting(true);
    try {
      const connectionAttributes = attributes();
      if (getServiceDataSource("resource") === "api") {
        await createDataSourceConnection({
          allowInheritance: false,
          attributes: connectionAttributes,
          kind,
          description: description.trim() || null,
          name: name.trim(),
          resourceCode: resourceCode.trim(),
          workspaceCode,
        });
      } else {
        createMockManagedResource({
          allowInheritance: false,
          attributes: connectionAttributes,
          description: description.trim(),
          file: null,
          kind,
          name: name.trim(),
          resourceCode: resourceCode.trim(),
          workspaceCode,
        });
      }
      toastManager.add({
        description: pageT("messages.dataSourceCreated"),
        title: pageT("messages.connectionVerified"),
        type: "success",
      });
      onOpenChange(false);
      onCreated();
    } catch (cause) {
      toastManager.add({
        description: cause instanceof Error ? cause.message : pageT("messages.createFailed"),
        title: pageT("messages.connectionFailed"),
        type: "error",
      });
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Dialog onOpenChange={onOpenChange} open={open}>
      <DialogContent className="max-h-[88vh] overflow-y-auto sm:max-w-2xl">
        <form onSubmit={submit}>
          <DialogHeader>
            <DialogTitle>{pageT("dataSource.createTitle")}</DialogTitle>
          </DialogHeader>

          <div className="grid gap-4 py-5 sm:grid-cols-2">
            <ConnectionSelectField id="data-source-type" label={pageT("fields.type")} onValueChange={(value) => setKind(value as DataSourceKindCode)} value={kind}>
              <ConnectionSelectItem value="MYSQL_CONNECTION">MySQL</ConnectionSelectItem>
              <ConnectionSelectItem value="POSTGRESQL_CONNECTION">PostgreSQL</ConnectionSelectItem>
              <ConnectionSelectItem value="REDIS_CONNECTION">Redis</ConnectionSelectItem>
              <ConnectionSelectItem value="KAFKA_CONNECTION">Kafka</ConnectionSelectItem>
            </ConnectionSelectField>
            <ConnectionTextField id="data-source-code" label={pageT("fields.code")} onChange={setResourceCode} readOnly required value={resourceCode} />
            <ConnectionTextField id="data-source-name" label={pageT("fields.name")} onChange={setName} placeholder={pageT("placeholders.name")} required value={name} />
            <div className="sm:col-span-2">
              <ConnectionTextareaField id="data-source-description" label={pageT("fields.description")} onChange={setDescription} value={description} />
            </div>
            {kind === "MYSQL_CONNECTION" ? (
              <ConnectionSelectField id="data-source-mysql-mode" label={pageT("fields.connectionMode")} onValueChange={updateConnectionMode} value={connectionMode}>
                {['SINGLE', 'FAILOVER', 'LOAD_BALANCE', 'REPLICATION'].map((value) => <ConnectionSelectItem key={value} value={value}>{pageT(`connectionModes.${value}`)}</ConnectionSelectItem>)}
              </ConnectionSelectField>
            ) : null}
            {kind === "POSTGRESQL_CONNECTION" ? (
              <>
                <ConnectionSelectField id="data-source-postgresql-load-balance" label={pageT("fields.loadBalance")} onValueChange={setPostgresqlLoadBalance} value={postgresqlLoadBalance}>
                  {['DISABLED', 'RANDOM'].map((value) => <ConnectionSelectItem key={value} value={value}>{pageT(`loadBalances.${value}`)}</ConnectionSelectItem>)}
                </ConnectionSelectField>
                <ConnectionSelectField id="data-source-postgresql-target-session" label={pageT("fields.targetSession")} onValueChange={setPostgresqlTargetSession} value={postgresqlTargetSession}>
                  {['ANY', 'READ_WRITE', 'READ_ONLY', 'PRIMARY', 'STANDBY', 'PREFER_STANDBY'].map((value) => <ConnectionSelectItem key={value} value={value}>{pageT(`targetSessions.${value}`)}</ConnectionSelectItem>)}
                </ConnectionSelectField>
              </>
            ) : null}
            {isRedis ? (
              <ConnectionSelectField id="data-source-redis-topology" label={pageT("fields.redisTopology")} onValueChange={updateRedisTopology} value={redisTopology}>
                {['STANDALONE', 'SENTINEL', 'CLUSTER'].map((value) => <ConnectionSelectItem key={value} value={value}>{pageT(`redisTopologies.${value}`)}</ConnectionSelectItem>)}
              </ConnectionSelectField>
            ) : null}
              <div className="space-y-3 sm:col-span-2">
                <div className="flex items-center justify-between gap-3">
                <p className="text-sm font-medium">{pageT(isKafka ? "fields.brokers" : supportsMultipleEndpoints ? "fields.nodes" : "fields.node")}</p>
                {supportsMultipleEndpoints ? (
                  <Button disabled={submitting} size="sm" type="button" variant="outline" onClick={addEndpoint}>{pageT("actions.addNode")}</Button>
                ) : null}
              </div>
              <div className="space-y-3">
                {endpoints.map((endpoint, index) => (
                  <div className={`grid gap-3 rounded-lg border border-border p-3 ${isMysqlReplication ? "sm:grid-cols-[minmax(0,1fr)_8rem_auto]" : "sm:grid-cols-[minmax(0,1fr)_8rem]"}`} key={index}>
                    <ConnectionTextField id={`data-source-node-${index}-host`} label={isKafka ? pageT("fields.brokerHost") : pageT("fields.host")} onChange={(value) => updateEndpoint(index, "host", value)} placeholder="db.internal" required value={endpoint.host} />
                    <ConnectionTextField id={`data-source-node-${index}-port`} label={pageT("fields.port")} min={1} onChange={(value) => updateEndpoint(index, "port", value)} required type="number" value={endpoint.port} />
                    {isMysqlReplication ? (
                      <ConnectionSelectField id={`data-source-node-${index}-role`} label={pageT("fields.role")} onValueChange={(value) => updateEndpoint(index, "role", value)} value={endpoint.role}>
                        <ConnectionSelectItem value="PRIMARY">{pageT("roles.PRIMARY")}</ConnectionSelectItem>
                        <ConnectionSelectItem value="REPLICA">{pageT("roles.REPLICA")}</ConnectionSelectItem>
                      </ConnectionSelectField>
                    ) : null}
                    {supportsMultipleEndpoints ? (
                      <div className={isMysqlReplication ? "sm:col-span-3 flex justify-end" : "sm:col-span-2 flex justify-end"}>
                        <Button disabled={submitting || endpoints.length <= minimumEndpointCount} size="sm" type="button" variant="ghost" onClick={() => removeEndpoint(index)}>{pageT("actions.removeNode")}</Button>
                      </div>
                    ) : null}
                  </div>
                ))}
              </div>
            </div>
            {isRedis && redisTopology === "SENTINEL" ? (
              <ConnectionTextField id="data-source-sentinel-master" label={pageT("fields.sentinelMasterName")} onChange={setSentinelMasterName} required value={sentinelMasterName} />
            ) : null}
            {!isKafka ? (
              <ConnectionTextField id="data-source-database" label={pageT("fields.database")} onChange={setDatabase} required={!isRedis} type={isRedis ? "number" : "text"} value={database} />
            ) : (
              <ConnectionSelectField id="data-source-security" label={pageT("fields.securityProtocol")} onValueChange={setSecurityProtocol} value={securityProtocol}>
                {['PLAINTEXT', 'SSL', 'SASL_PLAINTEXT', 'SASL_SSL'].map((value) => <ConnectionSelectItem key={value} value={value}>{value}</ConnectionSelectItem>)}
              </ConnectionSelectField>
            )}
            {usesSasl ? (
              <ConnectionSelectField id="data-source-sasl" label={pageT("fields.saslMechanism")} onValueChange={setSaslMechanism} value={saslMechanism}>
                {['PLAIN', 'SCRAM_SHA_256', 'SCRAM_SHA_512'].map((value) => <ConnectionSelectItem key={value} value={value}>{value}</ConnectionSelectItem>)}
              </ConnectionSelectField>
            ) : null}
            {!isKafka || usesSasl ? (
              <>
                <ConnectionTextField id="data-source-username" label={pageT("fields.username")} onChange={setUsername} required={!isRedis} value={username} />
                <ConnectionSelectField id="data-source-password-source" label={pageT("fields.passwordSource")} onValueChange={(value) => setPasswordSource(value as PasswordSource)} value={passwordSource}>
                  <ConnectionSelectItem value="DIRECT">{pageT("passwordSources.DIRECT")}</ConnectionSelectItem>
                  <ConnectionSelectItem value="VARIABLE">{pageT("passwordSources.VARIABLE")}</ConnectionSelectItem>
                </ConnectionSelectField>
                <ConnectionTextField description={passwordSource === "VARIABLE" ? pageT("hints.variableKey") : undefined} id="data-source-password" label={pageT(passwordSource === "DIRECT" ? "fields.password" : "fields.passwordVariableKey")} onChange={setPassword} required={!isRedis || usesSasl} type={passwordSource === "DIRECT" ? "password" : "text"} value={password} />
              </>
            ) : null}
            {!isKafka ? (
              <ConnectionSelectField id="data-source-tls" label={pageT("fields.tlsMode")} onValueChange={setTlsMode} value={tlsMode}>
                {(kind === "MYSQL_CONNECTION"
                  ? ['DISABLED', 'PREFERRED', 'REQUIRED', 'VERIFY_CA', 'VERIFY_IDENTITY']
                  : kind === "POSTGRESQL_CONNECTION"
                    ? ['DISABLED', 'ALLOW', 'PREFERRED', 'REQUIRED', 'VERIFY_CA', 'VERIFY_FULL']
                    : ['DISABLED', 'REQUIRED', 'VERIFY_CA', 'VERIFY_FULL'])
                  .map((value) => <ConnectionSelectItem key={value} value={value}>{value}</ConnectionSelectItem>)}
              </ConnectionSelectField>
            ) : null}
            {(usesKafkaTls || (!isKafka && tlsMode.startsWith("VERIFY"))) ? (
              <ConnectionTextField description={pageT("hints.variableKey")} id="data-source-ca-key" label={pageT("fields.caCertificateVariableKey")} onChange={setCaCertificateVariableKey} required={tlsMode.startsWith("VERIFY")} value={caCertificateVariableKey} />
            ) : null}
          </div>

          <DialogFooter>
            <Button disabled={submitting} type="button" variant="outline" onClick={() => onOpenChange(false)}>{pageT("actions.cancel")}</Button>
            <Button disabled={submitting} type="submit">
              {submitting ? <Spinner /> : null}
              {submitting ? pageT("actions.verifying") : pageT("actions.createAndVerify")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
