import axios from 'axios';
import { WORKSPACE_URL, headers } from '../utils/constants';

async function getWorkspaces() {
  try {
    const response = await axios({ url: WORKSPACE_URL, headers });
    return response.data;
  } catch (error) {
    console.error(error);
  }
}

async function createWorkspace(name: string) {
  try {
    const response = await axios({ url: WORKSPACE_URL, method: 'post', headers, data: name });
    return response.data;
  } catch (error) {
    console.error(error);
  }
}

async function deleteWorkspace(id: string) {
  try {
    const response = await axios({ url: `${WORKSPACE_URL}/${id}`, method: 'delete', headers });
    return response.data;
  } catch (error) {
    console.error(error);
  }
}

async function getWorkspace(id: string) {
  try {
    const response = await axios({
      url: `${WORKSPACE_URL}/${id}`,
      method: 'get',
      headers,
    });
    return response.data;
  } catch (error) {
    console.error(error);
  }
}

export { getWorkspaces, createWorkspace, deleteWorkspace, getWorkspace };
