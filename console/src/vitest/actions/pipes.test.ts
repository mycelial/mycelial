import { configurePipes, getPipes, createPipes, createPipe } from './pipes';
import { mockPostData, samplePipeRequestBody, samplePipeResponse } from '../testData/pipes';
import axios from 'axios';

vi.mock('axios');
describe('getPipes()', () => {
  test('calls axios', async () => {
    await getPipes();
    expect(axios.get).toHaveBeenCalled();
  });
});

describe('configurePipes', () => {
  it('should return empty nodes and edges if no response is provided', () => {
    const result = configurePipes(undefined);
    expect(result).toEqual({ data: { nodes: [], edges: [] } });
  });

  it('should configure pipes and return the correct nodes and edges', () => {
    const result = configurePipes(samplePipeResponse);

    // Add more specific expectations based on your logic
    expect(result.nodes.length).toBeGreaterThan(0);
    expect(result.edges.length).toBeGreaterThan(0);
  });
});

describe.skip('createPipe', () => {
  test('calls axios', async () => {
    await createPipe(mockPostData);

    expect(axios.post).toHaveBeenCalled();
  });
});
