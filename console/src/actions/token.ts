import axios from 'axios';
import { headers } from '../utils/constants';

async function getToken() {
  try {
    const response = await axios.post('http://localhost:8080/api/tokens', {
      method: 'POST',
      headers,
      body: JSON.stringify({ client_id: 'ui' }),
    });
    const result = response.data;
    return result;
  } catch (error) {
    console.error(error);
  }
}

// async function registerClient() {
//   try {
//     const response = await axios.post('http://localhost:8080/api/client', {
//       method: 'POST',
//       headers,
//       body: JSON.stringify({
//         client_config: {
//           node: {
//             unique_id: 'ui',
//             display_name: 'UI',
//             storage_path: '',
//           },
//           server: {
//             endpoint: 'localhost',
//             token: '',
//           },
//           sources: [],
//           destinations: [],
//         },
//       }),
//     });
//     const result = await response.json();
//     return result;
//   } catch (error) {
//     console.error(error);
//   }
// }

// async function registerClientAndGetToken() {
//   return registerClient()
//     .then((result) => {
//       getToken().then((result) => {
//         // return result.id || 'hi';
//         return 'id';
//       });
//     })
//     .catch((error) => {
//       console.error(error);
//       return 'error';
//     });
// }

export { getToken };
