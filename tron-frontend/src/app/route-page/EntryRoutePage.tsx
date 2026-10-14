import { Navigate } from "react-router";

/** The profile page owns session validation and the approach/entry camera sequence. */
export default function EntryRoutePage() {
  return <Navigate replace to="/profiles" />;
}
