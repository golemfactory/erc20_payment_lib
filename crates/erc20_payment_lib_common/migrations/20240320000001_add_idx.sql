DROP INDEX "idx_chain_tx_tx_hash";
CREATE UNIQUE INDEX "idx_chain_tx_tx_hash" ON "chain_tx" ("tx_hash");
