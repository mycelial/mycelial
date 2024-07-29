#![allow(unused)]
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
    Config,
    X,
    Y,
}

#[derive(Iden)]
pub enum Edges {
    Table,
    FromId,
    ToId,
}

#[derive(Iden)]
pub enum Certs {
    Table,
    Key,
    Value,
    CreatedAt,
}

impl Certs {
    #[inline(always)]
    pub fn ca_key() -> &'static str {
        "ca_key"
    }

    #[inline(always)]
    pub fn ca_cert() -> &'static str {
        "ca_cert"
    }

    #[inline(always)]
    pub fn key() -> &'static str {
        "key"
    }

    #[inline(always)]
    pub fn cert() -> &'static str {
        "cert"
    }
}

#[derive(Iden)]
pub enum Daemons {
    Table,
    Id,
    DisplayName,
    LastOnline,
}

#[derive(Iden)]
pub enum DaemonTokens {
    Table,
    Id,
    Secret,
    IssuedAt,
    UsedAt,
}