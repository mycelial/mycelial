import React, { DragEvent, useState } from 'react';
import styles from './styles';
import { Box, IconButton } from '@mui/material';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';
import Accordion from '@mui/material/Accordion';
import AccordionSummary from '@mui/material/AccordionSummary';
import AccordionDetails from '@mui/material/AccordionDetails';
import { NodeData, Client } from '../../types';
import DataChip from '../DataChip';
import { toTitleCase } from '../../utils';
import Drawer from '@mui/material/Drawer';
import ChevronLeftIcon from '@mui/icons-material/ChevronLeft';
import { Paper } from '@mui/material';
import CloseButton from '../CloseButton';

interface ClientDrawerProps {
  clients: Client[];
  onClose: () => void;
  open: boolean;
}
const drawerWidth = '22%';
const ClientDrawer: React.FC<ClientDrawerProps> = ({ clients, onClose, open }) => {
  const [showHelperText, setShowHelpterText] = React.useState(true);

  const onDragStart = (event: DragEvent<HTMLDivElement>, section: NodeData) => {
    event.dataTransfer.setData('application/reactflow', section.display_name);
    event.dataTransfer.setData('text/plain', section.clientId ?? '');
    event.dataTransfer.effectAllowed = 'move';
  };

  return (
    <Drawer
      anchor="left"
      variant="persistent"
      sx={{
        '&.MuiDrawer-root .MuiDrawer-paper': {
          marginTop: '130px',
          width: drawerWidth,
        },
        ...styles.aside,
      }}
      open={open}
    >
      <Box
        sx={{
          width: '100%',
          zIndex: '2',
          boxShadow: '0px 1px 8px 0px #153462',
        }}
      >
        {' '}
        <Box
          pt={4}
          sx={{
            height: '24px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'flex-end',
          }}
        >
          <IconButton sx={{ color: '#05386B', paddingTop: 0, paddingBottom: 0 }} onClick={onClose}>
            <ChevronLeftIcon />
          </IconButton>
        </Box>
        <Box className="description" ml={5} mr={3} mt={3} sx={{ fontSize: '0.7rem' }}>
          {showHelperText && (
            <Paper elevation={0} sx={{ border: '0.5px solid #9e9e9e', padding: 2 }}>
              <CloseButton onClick={() => setShowHelpterText(false)} color="#acacac" />
              Drag a section to the right to create a new data pipe
            </Paper>
          )}
        </Box>
        <Box mt={5} ml={5} pb={5} sx={{ fontSize: '0.9rem', fontWeight: 'bold', color: '#05386B' }}>
          Clients
        </Box>
      </Box>
      <Box
        sx={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'flex-start',
          overflowY: 'scroll',
          height: 'calc(100% - 300px)',
          gap: 1,
        }}
      >
        {clients &&
          clients.map((client) => {
            return (
              <Box
                mt={3}
                mx={3}
                sx={{
                  alignItems: 'center',
                  width: '92% ',
                  border: '0.5px solid #ACACAC',
                  borderRadius: 1,
                }}
                key={client.id}
              >
                <Accordion defaultExpanded disableGutters>
                  <AccordionSummary
                    expandIcon={<ExpandMoreIcon />}
                    aria-controls="panel1a-content"
                    id="panel1a-header"
                  >
                    <Box ml={3}>{toTitleCase(client.displayName)}</Box>
                  </AccordionSummary>
                  <AccordionDetails>
                    <Box
                      sx={{
                        display: 'flex',
                        flexDirection: 'column',
                        justifyContent: 'center',
                        alignItems: 'left',
                        rowGap: 1,
                        pl: 7,
                      }}
                    >
                      {client.sections &&
                        client.sections.map((section) => {
                          return (
                            <Box
                              sx={styles.node}
                              onDragStart={(event) => onDragStart(event, section)}
                              draggable
                              key={`${section.clientId + section.display_name + section.source}`}
                            >
                              <Box sx={{ alignSelf: 'flex-start' }}>
                                {section.destination && <DataChip flowType="destination" short />}
                                {section.source && <DataChip flowType="source" short />}
                              </Box>
                              <Box
                                sx={{
                                  display: 'flex',
                                  flexGrow: 4,
                                  justifyContent: 'center',
                                  textAlign: 'center',
                                  paddingLeft: '4px',
                                  marginRight: '-4px',
                                }}
                              >
                                <Box sx={{ display: 'flex', fontSize: '0.8rem' }}>
                                  {section.display_name}
                                </Box>
                              </Box>
                            </Box>
                          );
                        })}
                    </Box>
                  </AccordionDetails>
                </Accordion>
              </Box>
            );
          })}
        <Box my={3} sx={{ color: '#acacac', textAlign: 'center', width: '100%' }}>
          -- End --
        </Box>
      </Box>
      <Box
        sx={{
          zIndex: 200000,
          height: '88px',
          width: '100%',
          boxShadow: '0px -3px 8px 0px #153462',
        }}
      ></Box>
    </Drawer>
  );
};

export default ClientDrawer;
