import axios from 'axios';
import { WORKSPACE_URL, DAEMON_TOKEN_URL, headers } from '../utils/constants';
import { create } from '@mui/material/styles/createTransitions';



async function createDaemonToken(token: string) {
  try {
    const response = await axios({ url: DAEMON_TOKEN_URL,  method: 'post', headers: {'x-auth0-token': token, ...headers} });
    return response.data;
  } catch (error) {
    console.error(error);
  }

}
async function getDaemonToken(token: string) {
  try {
    const response = await axios({ url: DAEMON_TOKEN_URL, headers: {'x-auth0-token': token, ...headers} });
    return response.data;
  } catch (error) {
    const response = await createDaemonToken(token);
    return response.data;
  }
}

async function getWorkspaces(token: string) {
  try {
    const response = await axios({ url: WORKSPACE_URL, headers: {'x-auth0-token': token, ...headers} });
    return response.data;
  } catch (error) {
    console.error(error);
  }
}

async function createWorkspace(name: string, token: string) {
  try {
    const response = await axios({ url: WORKSPACE_URL, method: 'post', headers: {'x-auth0-token': token, ...headers}, data: name });
    return response.data;
  } catch (error) {
    console.error(error);
  }
}

async function deleteWorkspace(id: string, token: string) {
  try {
    const response = await axios({ url: `${WORKSPACE_URL}/${id}`, method: 'delete', headers: {'x-auth0-token': token, ...headers} });
    return response.data;
  } catch (error) {
    console.error(error);
  }
}

async function getWorkspace(id: string, token: string) {
  try {
    const response = await axios({
      url: `${WORKSPACE_URL}/${id}`,
      method: 'get',
      headers: {'x-auth0-token': token, ...headers},
    });
    return response.data;
  } catch (error) {
    console.error(error);
  }
}

export { getWorkspaces, createWorkspace, deleteWorkspace, getWorkspace, createDaemonToken, getDaemonToken };
