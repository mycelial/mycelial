import { WorkspacesData } from '../types';
import { getWorkspaces } from './workspaces';

const workspacesLoader = async (): Promise<WorkspacesData> => {
  const response = await getWorkspaces();
  return response;
};

export default workspacesLoader;
