import React from 'react';
import ReactDOM from 'react-dom/client';
import { createBrowserRouter, RouterProvider, Navigate } from 'react-router-dom';
import RootLayout from './layouts/RootLayout.js';
import Workspaces from './components/Workspaces.tsx';
import ErrorPage from './errorPage.jsx';
import './app.css';
import { ThemeProvider } from '@mui/material';
import theme from './theme';
import FlowWithProvider from './components/Flow';
import ClientsTable from './components/ClientsTable';
import workspacesLoader from './actions/workspacesLoader.ts';
import dataLoader from './actions/workspaceDataLoader.ts';
import './fonts/AeonikFono.ttf';
import './fonts/AeonikMono.ttf';
import { Auth0Provider } from '@auth0/auth0-react';

type DataLoaderParams = { params: { workspaceId: string } };

function paramsLoader({ params }: DataLoaderParams) {
  return { workspaceId: params.workspaceId };
}

const routesConfig = [
  {
    element: <RootLayout />,
    errorElement: <ErrorPage />,

    children: [
      {
        path: 'workspaces',
        element: <Workspaces />,
        loader: workspacesLoader, // interesting -- for some reason, if I remove this, you're continually redirected to the login page, even if it isn't used in the component. 
      },
      {
        path: 'workspaces/:workspaceId',
        element: <FlowWithProvider />,
        loader: paramsLoader,
      },
      {
        path: 'clients',
        element: <ClientsTable />,
        loader: dataLoader, // todo
      },
    ],
  },
  {
    path: '*',
    element: <Navigate to="/workspaces" />,
  },
];

const router = createBrowserRouter(routesConfig);

const rootElement = document.getElementById('root');
if (rootElement) {
  const domain = import.meta.env.VITE_AUTH0_DOMAIN;
  const auth0ClientId = import.meta.env.VITE_AUTH0_CLIENT_ID;
  const auth0Audience = import.meta.env.VITE_AUTH0_AUDIENCE;
  ReactDOM.createRoot(rootElement).render(
    // todo: put these values in a .env file
    <Auth0Provider
    domain={domain}
    clientId={auth0ClientId}
    cacheLocation="localstorage"
    authorizationParams={{
      redirect_uri: window.location.origin,
      audience: auth0Audience,
    }}
  >
    <React.StrictMode>
      <ThemeProvider theme={theme}>
        <RouterProvider router={router} />
      </ThemeProvider>
    </React.StrictMode>
  </Auth0Provider>
  );
}