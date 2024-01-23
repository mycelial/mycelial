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

const routesConfig = [
  {
    element: <RootLayout />,
    errorElement: <ErrorPage />,

    children: [
      {
        path: 'workspaces',
        element: <Workspaces />,
        loader: workspacesLoader,
      },
      {
        path: 'workspaces/:workspaceId',
        element: <FlowWithProvider />,
        loader: dataLoader,
      },
      {
        path: 'clients',
        element: <ClientsTable />,
        loader: dataLoader,
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
  ReactDOM.createRoot(rootElement).render(
    <React.StrictMode>
      <ThemeProvider theme={theme}>
        <RouterProvider router={router} />
      </ThemeProvider>
    </React.StrictMode>
  );
}
