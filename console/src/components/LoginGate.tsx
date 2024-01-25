import { useAuth0 } from "@auth0/auth0-react";

export default function LoginGate({ children}) {
  const { isLoading, isAuthenticated, loginWithRedirect ,getAccessTokenSilently } = useAuth0();

  if (isLoading) {
    console.log("isLoading");
    return <div>Loading...</div>;
  }

  if (!isLoading && !isAuthenticated) {
    console.log("not loading but not authenticated")
    loginWithRedirect();
    return <div>Loading...</div>;
  }

  return children;
}