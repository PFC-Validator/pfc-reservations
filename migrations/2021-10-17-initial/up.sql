-- Your SQL goes here
CREATE EXTENSION if not exists pgcrypto;
create table NFT
(
    id                         uuid primary key DEFAULT gen_random_uuid(),
    name                       varchar(255) unique      not null,
    meta_data                  json,
    svg                        json,
    ipfs_image                 char(46)                 null,
    ipfs_meta                  char(46)                 null,
    image_data                 text                     null,
    external_url               varchar(2000)            null,
    description                text                     null,
    background_color           char(6)                  null,
    animation_url              varchar(2000)            null,
    youtube_url                varchar(2000)            null,
    assigned                   boolean          default false,
    reserved                   boolean          default false,
    assigned_to_wallet_address char(44)                 null,
    reserved_to_wallet_address char(44)                 null,
    signed_packet              json                     null,
    has_submit_error           boolean          default false,
    reserved_until             timestamp with time zone null,
    assigned_on                timestamp with time zone null,
    in_process                 boolean          default false,
    txhash                     char(64)         default null,
    tx_error                   varchar(2000)    default null,
    token_id                   varchar(255)     default null,
    tx_retry_count             integer          default 0,
    in_mint_run                boolean          default false
);
create index txhash on nft(txhash);
create index token_id on nft(token_id);
create table NFT_Reservation
(
    id             uuid primary key DEFAULT gen_random_uuid(),
    wallet_address char(44) not null,
    nft_reserved   uuid references NFT (id),
    completed      boolean          default false,
    has_error      boolean          default false,
    has_expired    boolean          default false
);

create table stage_whitelist
(
    id              uuid primary key              DEFAULT gen_random_uuid(),
    code            char(20)                 not null,
    name            varchar(200)             not null,
    attribute_type  varchar(25)              null,
    attribute_value varchar(100)             null,
    is_default      bool                          default false,
    stage_open      timestamp with time zone      default now(),
    stage_close     timestamp with time zone null default null,
    stage_free      bool                          default false
);

--
-- wallets can get assigned to multiple stages
--
create table wallet_whitelist
(
    id               uuid primary key DEFAULT gen_random_uuid(),
    wallet_address   char(44) not null,
    stage            uuid references stage_whitelist (id),
    -- amount initially allocated
    allocation_count int              default 1,
    -- amount currently in reservation process
    reserved_count   int              default 0,
    -- amount currently assigned
    assigned_count   int              default 0
);
