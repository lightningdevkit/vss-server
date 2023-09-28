CREATE TABLE vss_db (
    store_id character varying(120) NOT NULL CHECK (store_id <> ''),
    key character varying(600) NOT NULL,
    value bytea NULL,
    version bigint NOT NULL,
    PRIMARY KEY (store_id, key)
);
