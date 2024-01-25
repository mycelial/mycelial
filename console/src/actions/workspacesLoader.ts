import { LoaderFunctionArgs } from 'react-router-dom';
import { WorkspacesData } from '../types';
import { getWorkspaces } from './workspaces';

const workspacesLoader = async (args: LoaderFunctionArgs): Promise<WorkspacesData> => {
  return [];
};

export default workspacesLoader;
