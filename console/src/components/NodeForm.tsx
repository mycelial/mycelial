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
  const { setNodes, activeNode, nodes, setShowActiveNode } = useFlowStore(selector);
  const [staged, setStaged] = useState(false);

  const id = activeNode?.id ?? '';
  const {
    display_name,
    id: dataId,
    type,
    clientId,
    clientName,
    source,
    destination,
    password,
    ...fields
  } = activeNode?.data ?? {};

  const handleSubmit = async (values: any) => {
    const nodeData = { ...activeNode?.data, ...values };
    // const connectedEdges = getConnectedEdges([node], edges);
    // for (const edge of connectedEdges) {
    //   let response;
    //   if (edge.source === id) {
    //     const targetNodeData = getNode(edge.target)?.data;
    //     response = await createPipe({
    //       id: edge.data.id,
    //       sourceNodeData: nodeData,
    //       targetNodeData,
    //     });
    //   }

    //   if (edge.target === id) {
    //     const sourceNodeData = getNode(edge.source)?.data;
    //     response = await createPipe({
    //       id: edge.data.id,
    //       sourceNodeData,
    //       targetNodeData: nodeData,
    //     });
    //   }
    //   if (response === 200) {
    //     setUpdated(true);
    //     setTimeout(() => setUpdated(false), 2000);
    //   }
    // }
    const updated = nodes.map((node) => {
      if (node.id === id) {
        node.data = nodeData;
      }
      return node;
    });
    setNodes(updated);

    setStaged(true);
    setTimeout(() => setStaged(false), 2000);
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
          >
            {Object.keys(sortedFields).map((key) => renderNodeFormField({ key, formik }))}

            <Box sx={{ display: 'flex', justifyContent: 'center', mt: 4 }}>
              <Button
                variant="contained"
                type="submit"
                sx={{
                  boxShadow: 3,
                  '&:hover': {
                    boxShadow: 6,
                  },
                }}
              >
                {staged ? (
                  <>
                    Staged
                    <DoneIcon />
                  </>
                ) : (
                  'Stage'
                )}
              </Button>
            </Box>
          </Box>
        </Box>
      </Paper>
    </>
  );
};

export default NodeForm;
