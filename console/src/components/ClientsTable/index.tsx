import * as React from 'react';
import Table from '@mui/material/Table';
import TableBody from '@mui/material/TableBody';
import TableCell from '@mui/material/TableCell';
import TableContainer from '@mui/material/TableContainer';
import TableHead from '@mui/material/TableHead';
import TableRow from '@mui/material/TableRow';
import Paper from '@mui/material/Paper';
import Container from '@mui/material/Container';
import { useLoaderData } from 'react-router-dom';
import './index.css';

// MuiTableCell-root MuiTableCell-head MuiTableCell-sizeMedium css-ysjy7b-MuiTableCell-root
// margin: 0 auto;
function createData(
  type: string,
  display_name: string,
  journal_path?: string,
  path?: string,
  minWidth?: number
) {
  return { type, display_name, journal_path, path };
}

const styles = {
  table: { minWidth: 650 },
  tableContainer: { backgroundColor: 'white' },
};

export default function ClientsTable() {
  const { clients } = useLoaderData();
  const createRows = (clients) =>
    clients.map((client) =>
      createData(
        client.type,
        client.display_name,
        client.journal_path,
        client.path
      )
    );
  const rows = clients && clients.length ? createRows(clients) : [];

  return (
    <>
      <Container maxWidth="md" sx={styles.tableContainer}>
        <h2>Clients</h2>
        <TableContainer component={Paper}>
          <Table sx={styles.table} aria-label="simple table" stickyHeader>
            <TableHead>
              <TableRow className="tableHeader">
                <TableCell>Display Name</TableCell>
                <TableCell align="right">Type</TableCell>
                <TableCell align="right">Journal Path</TableCell>
                <TableCell align="right">Path</TableCell>
                <TableCell align="right">Workspace IDs</TableCell>
                <TableCell align="right">Active?</TableCell>
                <TableCell align="right" sx={{ maxWidth: '115px' }}>
                  Last Mycelial Activity
                </TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {rows.map((row) => (
                <TableRow
                  key={row.display_name}
                  sx={{ '&:last-child td, &:last-child th': { border: 0 } }}
                >
                  <TableCell
                    component="th"
                    sx={{ fontWeight: 700 }}
                    scope="row"
                  >
                    {row.display_name}
                  </TableCell>
                  <TableCell align="right">{row.type}</TableCell>
                  <TableCell align="right">{row.journal_path}</TableCell>
                  <TableCell align="right">{row.path}</TableCell>
                  <TableCell align="right"></TableCell>
                  <TableCell align="right" sx={{ color: 'green' }}>
                    ●
                  </TableCell>
                  <TableCell align="right"></TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      </Container>
    </>
  );
}
