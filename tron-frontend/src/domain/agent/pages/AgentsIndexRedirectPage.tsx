import { Navigate } from "react-router";

export function AgentsIndexRedirectPage() {
  return <Navigate replace to="/agents/internal" />;
}
