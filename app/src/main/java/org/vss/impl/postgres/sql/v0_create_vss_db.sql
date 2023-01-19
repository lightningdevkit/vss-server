CREATE TABLE vss_db (
    store_id character varying(120)  NOT NULL,
    key character varying(120)  NOT NULL,
    value bytea  NULL,
    version bigint  NOT NULL,
    PRIMARY KEY (store_id, key)
);

