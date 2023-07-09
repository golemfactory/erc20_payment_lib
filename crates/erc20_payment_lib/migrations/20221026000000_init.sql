CREATE TABLE "tx"
(
    id                  INTEGER     NOT NULL     PRIMARY KEY AUTOINCREMENT,
    method              TEXT        NOT NULL,
    from_addr           TEXT        NOT NULL,
    to_addr             TEXT        NOT NULL,
    chain_id            INTEGER     NOT NULL,
    gas_limit           INTEGER     NULL,
    max_fee_per_gas     TEXT        NOT NULL,
    priority_fee        TEXT        NOT NULL,
    val                 TEXT        NOT NULL,
    nonce               INTEGER     NULL,
    processing          INTEGER     NOT NULL,
    call_data           TEXT        NULL,
    created_date        DATETIME    NOT NULL,
    first_processed     DATETIME    NULL,
    tx_hash             TEXT        NULL,
    signed_raw_data     TEXT        NULL,
    signed_date         DATETIME    NULL,
    broadcast_date      DATETIME    NULL,
    broadcast_count     INTEGER     NOT NULL,
    confirm_date        DATETIME    NULL,
    block_number        INTEGER     NULL,
    chain_status        INTEGER     NULL,
    fee_paid            TEXT        NULL,
    error               TEXT        NULL
);

CREATE INDEX "idx_tx_created_date" ON "tx" (created_date);
CREATE INDEX "idx_tx_first_processed" ON "tx" (first_processed);
CREATE INDEX "idx_tx_processing" ON "tx" (processing);

CREATE TABLE "chain_tx"
(
    id                  INTEGER     NOT NULL     PRIMARY KEY AUTOINCREMENT,
    tx_hash             TEXT        NOT NULL,
    method              TEXT        NOT NULL,
    from_addr           TEXT        NOT NULL,
    to_addr             TEXT        NOT NULL,
    chain_id            INTEGER     NOT NULL,
    gas_limit           INTEGER     NULL,
    max_fee_per_gas     TEXT        NULL,
    priority_fee        TEXT        NULL,
    val                 TEXT        NOT NULL,
    nonce               INTEGER     NOT NULL,
    checked_date        DATETIME    NOT NULL,
    blockchain_date     DATETIME    NOT NULL,
    block_number        INTEGER     NOT NULL,
    chain_status        INTEGER     NOT NULL,
    fee_paid            TEXT        NULL,
    error               TEXT        NULL
);


CREATE TABLE "token_transfer"
(
    id                  INTEGER     NOT NULL     PRIMARY KEY AUTOINCREMENT,
    payment_id          TEXT        NULL,
    from_addr           TEXT        NOT NULL,
    receiver_addr       TEXT        NOT NULL,
    chain_id            INTEGER     NOT NULL,
    token_addr          TEXT        NULL,
    token_amount        TEXT        NOT NULL,
    create_date         DATETIME    NOT NULL,
    tx_id               INTEGER     NULL,
    fee_paid            TEXT        NULL,
    error               TEXT        NULL,
    CONSTRAINT "fk_token_transfer_tx" FOREIGN KEY ("tx_id") REFERENCES "tx" ("id")
);

CREATE TABLE "transfer_in"
(
    id                  INTEGER     NOT NULL     PRIMARY KEY AUTOINCREMENT,
    payment_id          TEXT        NOT NULL,
    from_addr           TEXT        NOT NULL,
    receiver_addr       TEXT        NOT NULL,
    chain_id            INTEGER     NOT NULL,
    token_addr          TEXT        NULL,
    token_amount        TEXT        NOT NULL,
    tx_hash             TEXT        NULL,
    requested_date      DATETIME    NOT NULL,
    received_date       DATETIME    NULL
);

CREATE TABLE "chain_transfer"
(
    id                  INTEGER     NOT NULL     PRIMARY KEY AUTOINCREMENT,
    from_addr           TEXT        NOT NULL,
    receiver_addr       TEXT        NOT NULL,
    chain_id            INTEGER     NOT NULL,
    token_addr          TEXT        NULL,
    token_amount        TEXT        NOT NULL,
    chain_tx_id         INTEGER     NOT NULL,
    CONSTRAINT "fk_chain_transfer_tx" FOREIGN KEY ("chain_tx_id") REFERENCES "chain_tx" ("id")
);

CREATE TABLE "allowance"
(
    id                  INTEGER     NOT NULL     PRIMARY KEY AUTOINCREMENT,
    owner               TEXT        NOT NULL,
    token_addr          TEXT        NULL,
    spender             TEXT        NOT NULL,
    allowance           TEXT        NOT NULL,
    chain_id            INTEGER     NOT NULL,
    tx_id               INTEGER     NULL,
    fee_paid            TEXT        NULL,
    confirm_date        DATETIME    NULL,
    error               TEXT        NULL,
    CONSTRAINT "fk_allowance_tx" FOREIGN KEY ("tx_id") REFERENCES "tx" ("id")
);



