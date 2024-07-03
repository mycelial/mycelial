use sea_query::Iden;

#[derive(Iden)]
pub enum Clients {
    Table,
    Id,
    DisplayName,
    UserId,
    Sources,
    Destinations,
    UniqueClientId,
    ClientSecretHash,
}

#[derive(Iden)]
pub enum Messages {
    Table,
    Id,
    Topic,
    Origin,
    StreamType,
    CreatedAt,
}

#[derive(Iden)]
pub enum MessageChunks {
    Table,
    MessageId,
    ChunkId,
    Data,
}

#[derive(Iden)]
pub enum Workspaces {
    Table,
    Id,
    UserId,
    Name,
    CreatedAt,
}

#[derive(Iden)]
pub enum UserDaemonTokens {
    Table,
    UserId,
    DaemonToken,
}

#[derive(Iden)]
pub enum Pipes {
    Table,
    Id,
    UserId,
    WorkspaceId,
    RawConfig,
    CreatedAt,
}


// Graph entities
#[derive(Iden)]
pub enum Nodes {
    Table,
    Id,
    DisplayName,
    UserId,
    WorkspaceId,
    DaemonId,
}

#[derive(Iden)]
pub enum Edges {
    Table,
    FromId,
    ToId,    
}
