CREATE TABLE "scan_info"
(
    id                  INTEGER     NOT NULL     PRIMARY KEY AUTOINCREMENT,
    chain_id            INTEGER     NOT NULL,
    filter              TEXT        NOT NULL,
    start_block         INTEGER     NOT NULL,
    last_block          INTEGER     NOT NULL
);

CREATE UNIQUE INDEX "idx_scan_info_chain_id" ON "scan_info" ("chain_id", "filter");