import React, { MouseEventHandler, useEffect } from 'react';
import { Box, Button, Table, TableBody, TableCell, TableContainer, TableRow } from '@mui/material';
import { NavLink, useLoaderData } from 'react-router-dom';
import { Workspace, WorkspacesData } from '../types';
import { createWorkspace, deleteWorkspace } from '../actions/workspaces';
import Container from '@mui/material/Container';
import useWorkspacesStore, { selector } from '../stores/workspacesStore';
import { useFormik } from 'formik';
import TextField from '@mui/material/TextField';
import TableHead from '@mui/material/TableHead';
import Paper from '@mui/material/Paper';

const Workspaces = () => {
  const workspacesData = useLoaderData() as WorkspacesData;
  const { setWorkspaces, addWorkspace, workspaces } = useWorkspacesStore(selector);

  useEffect(() => {
    setWorkspaces(workspacesData);
  }, []);

  const [addNewWorkspace, setAddNewWorkspace] = React.useState(false);

  const handleSubmit = async (name: any) => {
    const response = await createWorkspace(name);
    if (response.id) {
      addWorkspace(response);

      return setAddNewWorkspace(false);
    }
  };

  const formik = useFormik({
    initialValues: { name: '' },
    enableReinitialize: true,
    onSubmit: handleSubmit,
  });

  function createData(id: string, name: string, created_at: string) {
    return { id, name, created_at };
  }

  const onDelete = (id: string) => {
    if (confirm('Are you sure you want to delete this node?')) {
      deleteWorkspace(id);
      const updatedWorkspaces = workspaces.filter((space) => space.id !== id);
      return setWorkspaces(updatedWorkspaces);
    }
  };

  const createRows = (workspaces: Workspace[]) =>
    workspaces.map((workspace) => createData(workspace.id, workspace.name, workspace.created_at));
  const rows = workspaces && workspaces.length ? createRows(workspaces) : [];

  return (
    <Container fixed>
      <h2>Workspaces</h2>

      <TableContainer component={Paper} sx={{ width: '70%' }}>
        <Table
          aria-label="simple table"
          stickyHeader
          sx={{
            '& .MuiTableRow-root:hover': {
              backgroundColor: '#f6f6f6',
              height: '1px',
            },
          }}
        >
          <TableHead>
            <TableRow className="tableHeader">
              <TableCell align="left">Name</TableCell>
              <TableCell align="right">Created At</TableCell>
              <TableCell align="right"></TableCell>
            </TableRow>
          </TableHead>
          <TableBody sx={{ borderCollapse: 'collapse' }}>
            {rows.map((row) => (
              <TableRow key={row.id} sx={{ height: '100%' }}>
                <TableCell align="left" sx={{ height: '100%', padding: '12px' }}>
                  <NavLink
                    to={`/workspaces/${row.id}`}
                    style={{
                      textDecoration: 'none',
                      color: 'black',
                      padding: '20px',
                      display: 'block',
                    }}
                  >
                    {row.name}
                  </NavLink>
                </TableCell>
                <TableCell align="right">{new Date(row.created_at).toLocaleString()}</TableCell>
                <TableCell align="right">
                  <Button
                    variant="outlined"
                    type="button"
                    color="error"
                    onClick={() => onDelete(row.id)}
                  >
                    Delete
                  </Button>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </TableContainer>

      {!addNewWorkspace && (
        <Button
          variant="contained"
          type="button"
          color="primary"
          onClick={() => setAddNewWorkspace(true)}
        >
          Add New Workspace
        </Button>
      )}
      {addNewWorkspace && (
        <Box
          component="form"
          onSubmit={formik.handleSubmit}
          m={2}
          p={2}
          sx={{ width: '30%', border: '1px solid navy', borderRadius: '4px' }}
        >
          <Box mb={2}>
            <h3>New Workspace</h3>
            <TextField
              required
              fullWidth
              id="name"
              name="name"
              label="Name"
              value={formik.values.name}
              onChange={formik.handleChange}
              onBlur={formik.handleBlur}
              error={formik.touched.name && Boolean(formik.errors.name)}
              helperText={formik.touched.name && formik.errors.name}
            />
          </Box>
          <Button
            variant="contained"
            type="button"
            color="primary"
            onClick={formik.handleSubmit as unknown as MouseEventHandler<HTMLButtonElement>}
          >
            Create New Workspace
          </Button>
        </Box>
      )}
    </Container>
  );
};
export default Workspaces;
