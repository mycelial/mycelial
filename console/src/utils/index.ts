const getRandomString = () => {
  return String(Date.now().toString(32) + Math.random().toString(16)).replace(/\./g, '');
};

const getId = (kind = "node") => `${kind}_${getRandomString()}`;

const toTitleCase = (s: string) =>
  s
    .replace(/^[-_]*(.)/, (_, c) => c.toUpperCase()) // Initial char (after -/_)
    .replace(/[-_]+(.)/g, (_, c) => ' ' + c.toUpperCase()); // First char after each -/_

const capitalize = (word: string) => word.charAt(0).toUpperCase() + word.slice(1);

export { getId, getRandomString, toTitleCase, capitalize };
