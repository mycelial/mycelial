import React, { SyntheticEvent, MouseEvent, useState } from 'react';
import AppBar from '@mui/material/AppBar';
import Box from '@mui/material/Box';
import Toolbar from '@mui/material/Toolbar';
import IconButton from '@mui/material/IconButton';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import AccountCircle from '@mui/icons-material/AccountCircle';
import { ReactComponent as Logo } from '../assets/logo.svg';
import { NavLink } from 'react-router-dom';
import { Tab, Tabs } from '@mui/material';

const styles = {
  tabs: {
    paddingBottom: 2,
    '& a': { textTransform: 'none', color: '#fff' },
    '& a.Mui-selected': { color: 'primary.contrastText' },
  },
};

function samePageLinkNavigation(event: React.MouseEvent<HTMLAnchorElement, MouseEvent>) {
  if (
    event.defaultPrevented ||
    event.button !== 0 || // ignore everything but left-click
    event.metaKey ||
    event.ctrlKey ||
    event.altKey ||
    event.shiftKey
  ) {
    return false;
  }
  return true;
}
export default function Navbar() {
  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null);

  const handleMenu = (event: MouseEvent<HTMLElement>) => {
    setAnchorEl(event.currentTarget);
  };

  const handleClose = () => {
    setAnchorEl(null);
  };

  const [value, setValue] = useState(0);

  const handleChange = (event: SyntheticEvent, newValue: number) => {
    // event.type can be equal to focus with selectionFollowsFocus.
    if (
      event.type !== 'click' ||
      (event.type === 'click' &&
        samePageLinkNavigation(event as unknown as React.MouseEvent<HTMLAnchorElement, MouseEvent>))
    ) {
      setValue(newValue);
    }
  };

  return (
    <Box sx={{ flexGrow: 1 }}>
      <AppBar sx={{ bgcolor: 'primary.dark', position: 'static' }} elevation={0}>
        <Toolbar>
          <Logo
            style={{ height: '24px', width: '120px', marginLeft: '20px' }}
            sx={{ paddingBottom: '16px', zIndex: '1400', marginTop: '12px' }}
          />
          <Box
            sx={{
              display: 'flex',
              flexGrow: 1,
              justifyContent: 'space-between',
              alignItems: 'center',
            }}
          >
            <Box ml={6} sx={{ display: 'flex' }}>
              <Tabs
                indicatorColor="secondary"
                centered
                value={value}
                onChange={handleChange}
                TabIndicatorProps={{
                  style: {
                    display: 'none',
                  },
                }}
                aria-label="nav tabs"
                sx={styles.tabs}
              >
                <Tab
                  component={NavLink}
                  sx={{ marginTop: '10px', fontSize: '0.8rem' }}
                  to="workspaces"
                  label="Workspaces"
                />
                {/* <Tab
                  component={NavLink}
                  sx={{ marginTop: '10px' }}
                  to="clients"
                  label="Clients"
                /> */}
              </Tabs>
            </Box>
            <Box>
              <IconButton
                size="large"
                aria-label="account of current user"
                aria-controls="menu-appbar"
                aria-haspopup="true"
                onClick={handleMenu}
                color="inherit"
              >
                <AccountCircle />
              </IconButton>
              <Menu
                id="menu-appbar"
                anchorEl={anchorEl}
                anchorOrigin={{
                  vertical: 'top',
                  horizontal: 'right',
                }}
                keepMounted
                transformOrigin={{
                  vertical: 'top',
                  horizontal: 'right',
                }}
                open={Boolean(anchorEl)}
                onClose={handleClose}
              >
                <MenuItem onClick={handleClose}>Profile</MenuItem>
                <MenuItem onClick={handleClose}>Log Out</MenuItem>
              </Menu>
            </Box>
          </Box>
        </Toolbar>
      </AppBar>
    </Box>
  );
}
