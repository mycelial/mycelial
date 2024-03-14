import React from 'react';
import Paper from '@mui/material/Paper';
import Button from '@mui/material/Button';
import { toTitleCase } from '../utils';
import Box from '@mui/material/Box';
import { useFormik } from 'formik';
import { useState } from 'react';
import DoneIcon from '@mui/icons-material/Done';
import renderNodeFormField from './NodeFormField';
import DataChip from './DataChip';
import CloseButton from './CloseButton';
import useFlowStore, { selector } from '../stores/flowStore';
import { FlowType } from '../types';

const NodeForm = () => {
  const { setNodes, activeNode, nodes, setShowActiveNode, getSavedNodeData } = useFlowStore(selector);

  if (!activeNode) return null;
  const original = getSavedNodeData(activeNode?.id);

  const id = activeNode?.id ?? '';
  const {
    display_name,
    id: dataId,
    type,
    clientId,
    clientName,
    source,
    destination,
    name, 
    client, // todo
    ...fields
  } = activeNode?.data ?? {};

  const handleSubmit = async (values: any) => {
    const nodeData = { ...activeNode?.data, ...values };
    const updated = nodes.map((node) => {
      if (node.id === id) {
        node.data = nodeData;
      }
      return node;
    });
    setNodes(updated);
  };

  const formik = useFormik({
    initialValues: fields,
    enableReinitialize: true,
    onSubmit: handleSubmit,
  });

  const sortedFields = Object.keys(fields)
    .sort()
    .reduce((objEntries, key) => {
      objEntries[key] = fields[key];
      return objEntries;
    }, {});

  return (
    <>
      <Paper elevation={6} sx={{ border: '1px solid #9e9e9e' }}>
        <Box
          m={4}
          sx={{
            display: 'flex',
            flexDirection: 'column',
            gap: 2,
            width: '88%',
          }}
        >
          <Box>
            <Box sx={{ display: 'inline-block' }}>
              {source && (
                <Box mb={1} sx={{ float: 'left' }}>
                  <DataChip flowType={FlowType.Source} small={false} />
                </Box>
              )}
              {destination && (
                <Box>
                  <DataChip flowType={FlowType.Destination} small={false} />
                </Box>
              )}
            </Box>
            <CloseButton onClick={() => setShowActiveNode(false)} />
          </Box>
          <Box mt={4} mb={1} ml={1}>
            <h4 style={{ margin: 0, color: '#05386B' }}>{toTitleCase(clientName ?? '')}</h4>

            <h3
              style={{
                marginTop: '2px',
                color: '#05386B',
              }}
            >
              {toTitleCase(display_name ?? '')}
            </h3>
          </Box>
          <Box
            component="form"
            sx={{
              '& .MuiTextField-root': { m: 1, width: '95%' },
              '& .MuiFormLabel-input': { fontSize: '1rem' },
            }}
            onSubmit={formik.handleSubmit}
            onBlur={formik.handleSubmit}
            onChange={formik.handleSubmit}
          >
            {Object.keys(sortedFields).map((key) => renderNodeFormField({ key, formik, original: original?.data}))}

          </Box>
        </Box>
      </Paper>
    </>
  );
};

export default NodeForm;
