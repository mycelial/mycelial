"use client";

import * as React from "react";
import { ClientContextType, IClient } from "../@types/client";

export const ClientContext = React.createContext<ClientContextType | null>(
  null
);

async function getClients(token: string) {
  try {
    const response = await fetch("/api/clients", {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
        "X-Authorization": "Bearer " + btoa(token),
      },
    });
    const result = await response.json();
    return result;
  } catch (error) {
    console.error(error);
  }
}

async function registerClient() {
  try {
    const response = await fetch("/api/client", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        client_config: {
          node: {
            unique_id: "ui",
            display_name: "UI",
            storage_path: ""
          },
          server: {
            endpoint: "localhost",
            token: ""
          },
          sources: [],
          destinations: []
        }
      }),
    });
    const result = await response.json();
    return result;
  } catch (error) {
    console.error(error);
  }
}

async function getToken() {
  try {
    const response = await fetch("/api/tokens", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ client_id: "ui" }),
    });
    const result = await response.json();
    return result;
  } catch (error) {
    console.error(error);
  }
}

async function registerClientAndGetToken() {
  return registerClient()
    .then((result) => {
      getToken().then((result) => {
        return result.id;
      });
    })
    .catch((error) => {
      console.error(error);
      return "error";
    });
}

const ClientProvider: React.FC<React.PropsWithChildren> = ({ children }) => {
  const [clients, setClients] = React.useState<IClient[]>([
    {
      id: "post 1",
      display_name: "Post Dev",
      sources: [],
      destinations: []
    },
    {
      id: "post 2",
      display_name: "Post Prod",
      sources: [],
      destinations: [],
    },
  ]);

  const [token, setToken] = React.useState<string>("");

  React.useEffect(() => {
    registerClientAndGetToken().then((result) => {
      if (result) {
        setToken(result);
      }
    });
  }, []);

  React.useEffect(() => {
    getClients(token).then((result) => {
      setClients(result.clients);
    });
  }, [token]);

  return (
    <ClientContext.Provider value={{ clients, token }}>
      {children}
    </ClientContext.Provider>
  );
};

export default ClientProvider;
