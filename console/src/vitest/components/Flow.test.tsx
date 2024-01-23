import FlowWithProvider from '.';
import { cleanup, getByTestId, render, screen } from '@testing-library/react';
import React from 'react';
import { samplePipeData } from '../testData/pipes';
import { sampleClientData } from '../testData/clients';
import { useNodesState, useEdgesState } from 'reactflow';
import {
  BrowserRouter,
  RouterProvider,
  createBrowserRouter,
  useLoaderData,
} from 'react-router-dom';
import userEvent from '@testing-library/user-event';

const mockData = {
  clients: sampleClientData,
  data: {
    nodes: samplePipeData.nodes,
    edges: samplePipeData.edges,
  },
};
const renderWithRouter = (ui, { route = '/' } = {}) => {
  window.history.pushState({}, 'Test page', route);

  return {
    user: userEvent.setup(),
    ...render(ui, { wrapper: BrowserRouter }),
  };
};

test('Flow renders', async () => {
  vi.mock('react-router-dom');
  vi.mock('reactflow');
  vi.mocked(useNodesState).mockReturnValue([mockData.data.nodes, () => {}, () => {}]);
  vi.mocked(useEdgesState).mockReturnValue([mockData.data.edges, () => {}, () => {}]);

  vi.mocked(useLoaderData).mockReturnValue(mockData);
  // test('full app rendering/navigating', async () => {
  const { user } = await renderWithRouter(<FlowWithProvider />);
  console.log(screen.debug());
  cleanup();
});
