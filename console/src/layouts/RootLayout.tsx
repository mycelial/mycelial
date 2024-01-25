import * as React from 'react';
import CssBaseline from '@mui/material/CssBaseline';
import Navbar from '../components/Navbar';
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
  return (
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
}
