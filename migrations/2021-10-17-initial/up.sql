-- Your SQL goes here
CREATE EXTENSION if not exists pgcrypto;
create table NFT
(
    id               uuid primary key DEFAULT gen_random_uuid (),
    name             varchar(255)  unique not null ,
    meta_data        json,
    svg              json,
    ipfs_image       char(46)      null,
    ipfs_meta        char(46)      null,
    image_data       text          null,
    external_url     varchar(2000) null,
    description      text          null,
    background_color char(6)       null,
    animation_url    varchar(2000) null,
    youtube_url      varchar(2000) null,
    assigned         boolean default false
);
create table NFT_Reservation
(
    id             uuid primary key DEFAULT gen_random_uuid (),
    wallet_address char(44) not null,
    signed_packet  json     null,
    reserved_until timestamp,
    nft_reserved  uuid references NFT (id),
    completed      boolean default false,
    has_error      boolean default false,
    has_expired    boolean default false

)