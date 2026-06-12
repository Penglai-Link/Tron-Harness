import {
  Navigate,
  useSearchParams,
} from "react-router";

export function EditPlatformVariablePage() {
  const [searchParams] = useSearchParams();
  const variableKey = searchParams.get("variableKey") ?? "";

  if (!variableKey) {
    return <Navigate replace to="/settings/variables" />;
  }

  return (
    <Navigate
      replace
      to={`/settings/variables/detail?variableKey=${encodeURIComponent(variableKey)}&mode=edit`}
    />
  );
}
