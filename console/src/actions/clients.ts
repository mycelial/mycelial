import axios, { AxiosResponse } from 'axios';
import { CLIENT_URL, DAEMON_TOKEN_URL, headers, mycelialServer } from '../utils/constants';

async function getClients(token: string) {
  try {
    const response = await axios.get(CLIENT_URL, { headers: { 'x-auth0-token': token, ...headers }});
    return formatClients(response);
  } catch (error) {
    console.error(error);
  }
}

async function createDaemonToken(token: string) {
  try {
    const response = await axios.post(DAEMON_TOKEN_URL, {}, { headers: { 'x-auth0-token': token, ...headers }});
    return response;
  } catch (error) {
    console.error(error);
  }
}
type clientFormatType = {
  id: string;
  displayName: string;
  sections?: any[];
};

const formatSections = (client: clientFormatType, sections: any[], sources = true) => {
  if (sections.length === 0) return [];

  return sections.map((section) => {
    let formatted = { ...section };
    formatted.clientId = client.id;
    formatted.name = `${section.type}_${sources ? 'source' : 'destination'}`;
    formatted.clientName = client.displayName;
    if (sources) {
      formatted.source = true;
    } else {
      formatted.destination = true;
    }
    // FIXME: this section is called a source on the backend, so to have it appear in the UI correctly, we
    // have to tag it as both a source and a destination here
    if (section.type.endsWith("transformer")) {
      formatted.source = true;
      formatted.destination = true;
      formatted.name = section.type;
    }
    return formatted;
  });
};

function formatClients(response: AxiosResponse) {
  const clientResponse = response?.data?.clients;

  if (!response || !response.data || !clientResponse) {
    return [];
  }
  const clients = [mycelialServer];

  for (const client of clientResponse) {
    if (client.id === 'ui') continue;

    let formattedClient = {
      id: client.id,
      displayName: client.display_name,
      sections: [] as any[],
    };

    const formattedClientSections = [
      formatSections(formattedClient, client.sources),
      formatSections(formattedClient, client.destinations, false),
    ].flat();

    formattedClient.sections = formattedClientSections;
    clients.push(formattedClient);
  }
  return clients;
}

export { getClients, formatClients, createDaemonToken as createDaemonToken };
