import axios from "axios";
import { WORKSPACE_URL, DAEMON_TOKEN_URL } from "../utils/constants";
import { create } from "@mui/material/styles/createTransitions";

// FIXME: return error properly
async function createDaemonToken(token: string) {
  const response = await axios({
    url: DAEMON_TOKEN_URL,
    method: "post",
    headers: { "x-auth0-token": token },
  });
  return response.data;
}

// FIXME: check response properly
async function getDaemonToken(token: string) {
  try {
    const response = await axios({
      url: DAEMON_TOKEN_URL,
      headers: { "x-auth0-token": token },
    });
    return response.data;
  } catch (error) {
    return await createDaemonToken(token);
  }
}

async function getWorkspaces(token: string) {
  try {
    const response = await axios({
      url: WORKSPACE_URL,
      headers: { "x-auth0-token": token },
    });
    return response.data;
  } catch (error) {
    console.error(error);
  }
}

async function createWorkspace(name: string, token: string) {
  try {
    const response = await axios({
      url: WORKSPACE_URL,
      method: "post",
      headers: { "x-auth0-token": token },
      data: name,
    });
    return response.data;
  } catch (error) {
    console.error(error);
  }
}

async function deleteWorkspace(id: string, token: string) {
  try {
    const response = await axios({
      url: `${WORKSPACE_URL}/${id}`,
      method: "delete",
      headers: { "x-auth0-token": token },
    });
    return response.data;
  } catch (error) {
    console.error(error);
  }
}

async function getWorkspace(id: string, token: string) {
  try {
    const response = await axios({
      url: `${WORKSPACE_URL}/${id}`,
      method: "get",
      headers: { "x-auth0-token": token },
    });
    return response.data;
  } catch (error) {
    console.error(error);
  }
}

export {
  getWorkspaces,
  createWorkspace,
  deleteWorkspace,
  getWorkspace,
  createDaemonToken,
  getDaemonToken,
};
