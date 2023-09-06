export interface IClient {
    id: string;
    display_name: string;
    sources: Array<ISource>;
    destinations: Array<IDestination>;
}

type ISource = ISqlite | IKafka | IPostgres | ISnowflake | IMyceliteSource;
type IDestination = ISqlite | ISnowflake | IMyceliteDestination;

type ISqlite = {
    type: string;
    display_name: string;
    path: string;
}

type IKafka = {
    type: string;
    display_name: string;
    brokers: string;
}

type IPostgres = {
    type: string;
    display_name: string;
    host: string;
    port: number;
    user: string;
    password: string;
    database: string;
}

type ISnowflake = {
    type: string;
    display_name: string;
    username: string;
    password: string;
    role: string;
    account_identifier: string;
    warehouse: string;
    database: string;
}

type IMyceliteSource = {
    type: string;
    display_name: string;
    journal_path: string;
}

type IMyceliteDestination = {
    type: string;
    display_name: string;
    journal_path: string;
    database_path: string;
}

export type ClientContextType = {
    clients: Array<IClient>;
    token: string;
};