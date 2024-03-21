import { LoaderFunctionArgs } from 'react-router-dom';
import { WorkspacesData } from '../types';
import { getWorkspaces, createDaemonToken, getDaemonToken } from './workspaces';

const loadWorkspaces = async (token: string): Promise<WorkspacesData> => {
  const workspaces = await getWorkspaces(token);
  return workspaces;
};

const loadDaemonToken = async(token: string): Promise<string> => {
  return await getDaemonToken(token);
}

export { loadWorkspaces, loadDaemonToken} ;
