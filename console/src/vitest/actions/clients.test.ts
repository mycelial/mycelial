import { formatClients } from '../../actions/clients';
import { sampleClientResponse, sampleClientData } from '../../vitest/testData/clients';

test('returns the expected result', () => {
  expect(formatClients(sampleClientResponse)).toStrictEqual(sampleClientData);
});

test('returns the expected number of clients', () => {
  expect(formatClients(sampleClientResponse)).toHaveLength(4);
});

test('empty response', () => {
  expect(formatClients({ data: { clients: [] } })).toStrictEqual([]);
});
