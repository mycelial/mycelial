import * as React from 'react';
import Box from '@mui/material/Box';
import Drawer from '@mui/material/Drawer';
import List from '@mui/material/List';
import IconButton from '@mui/material/IconButton';
import ChevronRightIcon from '@mui/icons-material/ChevronRight';
import ListItem from '@mui/material/ListItem';
import { Paper } from '@mui/material';
import NodeForm from './NodeForm';
import useFlowStore, { selector } from '../stores/flowStore';

const drawerWidth = '22%';

interface EditDrawerProps {
  onClose: () => any;
  open: boolean;
}

const EditDrawer: React.FC<EditDrawerProps> = ({ onClose, open }) => {
  const { activeNode, showActiveNode } = useFlowStore(selector);

  return (
    <Drawer
      sx={{
        width: drawerWidth,
        flexShrink: 0,
        '& .MuiDrawer-paper': {
          width: drawerWidth,
        },
        '&.MuiDrawer-root .MuiDrawer-paper': { marginTop: '130px' },
      }}
      variant="persistent"
      anchor="right"
      open={open}
    >
      <Box
        pt={4}
        sx={{
          height: '24px',
          display: 'flex',
          alignItems: 'center',

          justifyContent: 'flex-start',
        }}
      >
        <IconButton sx={{ color: '#05386B', paddingTop: 0, paddingBottom: 0 }} onClick={onClose}>
          <ChevronRightIcon />
        </IconButton>
      </Box>
      <Box sx={{ overflowY: 'scroll', height: 'calc(100% - 136px)', position: 'static' }}>
        <List sx={{ p: 2 }}>
          <ListItem sx={{ display: 'block' }}>
            <Paper elevation={0}>
              <Box
                sx={{
                  color: '#05386B',
                }}
              >
                {activeNode && showActiveNode ? (
                  <NodeForm key={activeNode.id} />
                ) : (
                  <Box pt={4}>Click on a section for more information and/or to edit</Box>
                )}
              </Box>
            </Paper>
          </ListItem>
        </List>
      </Box>
    </Drawer>
  );
};
export default EditDrawer;
