import axios, { AxiosResponse } from 'axios';
import { CLIENT_URL, headers, mycelialServer } from '../utils/constants';

async function getClients() {
  try {
    const response = await axios.get(CLIENT_URL, { headers });
    return formatClients(response);
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
      if (section.type === 'hello_world') {
        formatted.interval_milis = section.interval_milis || 5000;
        formatted.message = 'Hello!';
      }
      if (section.type === 'sqlite_connector') {
        formatted.strict ??= true;
        formatted.tables ??= '*';
      }

      if (section.type === 'excel_connector') {
        formatted.sheets = section.sheets || '*';
      }
    } else {
      formatted.destination = true;
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

export { getClients, formatClients };
