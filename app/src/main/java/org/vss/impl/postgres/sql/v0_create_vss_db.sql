CREATE TABLE vss_db (
    user_token character varying(120) NOT NULL CHECK (user_token <> ''),
    store_id character varying(120) NOT NULL CHECK (store_id <> ''),
    key character varying(600) NOT NULL,
    value bytea NULL,
    version bigint NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE,
    last_updated_at TIMESTAMP WITH TIME ZONE,
    PRIMARY KEY (user_token, store_id, key)
);
