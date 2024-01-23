import React from 'react';
import MuiAppBar, { AppBarProps as MuiAppBarProps } from '@mui/material/AppBar';
import Button from '@mui/material/Button';
import DoneIcon from '@mui/icons-material/Done';
import Box from '@mui/material/Box';

interface WorkspaceAppBarProps extends MuiAppBarProps {
  published: boolean;
  name: string;
  onPublish: () => void;
}

const WorkspaceAppBar: React.FC<WorkspaceAppBarProps> = ({ onPublish, published, name }) => (
  <MuiAppBar
    position="static"
    sx={{
      height: '64px',
      bgcolor: 'primary.light',
      zIndex: (theme) => theme.zIndex.drawer + 1,
      flexDirection: 'row',
    }}
    elevation={0}
  >
    <Box
      sx={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        width: '66%',
        marginLeft: '26%',
      }}
    >
      <Box
        sx={{
          color: '#white',
          fontWeight: '500',
          fontSize: '1rem',
        }}
      >
        {name ? `Workspace: ${name}` : ''}
      </Box>
      <Button variant="contained" onClick={onPublish} sx={{ backgroundColor: '#153462' }}>
        {published ? <DoneIcon /> : 'Publish'}
      </Button>
    </Box>
  </MuiAppBar>
);

export default WorkspaceAppBar;
