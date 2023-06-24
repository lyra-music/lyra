CREATE TABLE IF NOT EXISTS guild_configs (
    id bigint primary key NOT NULL,
    now_playing boolean NOT NULL DEFAULT true,
    usr_access boolean,
    rol_access boolean,
    xch_access boolean,
    tch_access boolean,
    vch_access boolean,
    cch_access boolean
);
CREATE TABLE IF NOT EXISTS usr_access (
    guild bigint references guild_configs(id),
    id bigint NOT NULL
);
CREATE TABLE IF NOT EXISTS rol_access (
    guild bigint references guild_configs(id),
    id bigint NOT NULL
);
CREATE TABLE IF NOT EXISTS xch_access (
    guild bigint references guild_configs(id),
    id bigint NOT NULL
);
CREATE TABLE IF NOT EXISTS tch_access (
    guild bigint references guild_configs(id),
    id bigint NOT NULL
);
CREATE TABLE IF NOT EXISTS vch_access (
    guild bigint references guild_configs(id),
    id bigint NOT NULL
);
CREATE TABLE IF NOT EXISTS cch_access (
    guild bigint references guild_configs(id),
    id bigint NOT NULL
);