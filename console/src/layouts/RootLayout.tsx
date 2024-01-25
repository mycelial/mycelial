import * as React from 'react';
import CssBaseline from '@mui/material/CssBaseline';
import Navbar from '../components/Navbar';
import LoginGate from '../components/LoginGate';
import { Outlet } from 'react-router-dom';
import Box from '@mui/material/Box';

const styles = {
  appContainer: {
    alignItems: 'flex-start',
    flexGrow: '1',
    flexDirection: 'column',
  },
  navbarContainer: { width: '100%' },
  subnav: {
    marginTop: '20px',
    width: '100vh',
  },
};

export default function RootLayout() {
  let content = (
      <Box sx={styles.appContainer}>
        <CssBaseline />
        <header>
          <Navbar />
        </header>
        <main>
          <Outlet />
        </main>
      </Box>
  );

  if (import.meta.env.VITE_USE_AUTH0 === "true") {
    return (
      <LoginGate>
        {content}
      </LoginGate>
    );
  }

  return content;
}
