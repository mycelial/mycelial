import { useAuth0 } from "@auth0/auth0-react";

export default function LoginGate({ children }) {
  const { isLoading, isAuthenticated, loginWithRedirect } = useAuth0();

  if (isLoading) {
    return <div>Loading...</div>;
  }

  if (!isLoading && !isAuthenticated) {
    loginWithRedirect();
    return <div>Loading...</div>;
  }

  return children;
}
