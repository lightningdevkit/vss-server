CREATE TABLE vss_db (
    store_id character varying(120) NOT NULL CHECK (store_id <> ''),
    key character varying(600) NOT NULL,
    value bytea NULL,
    version bigint NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE,
    last_updated_at TIMESTAMP WITH TIME ZONE,
    PRIMARY KEY (store_id, key)
);
