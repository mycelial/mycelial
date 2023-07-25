export interface IClient {
    id: string;
}

export type ClientContextType = {
    clients: Array<IClient>;
    token: string;
};