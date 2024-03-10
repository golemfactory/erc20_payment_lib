
ALTER TABLE token_transfer RENAME COLUMN allocation_id TO deposit_id;
ALTER TABLE token_transfer DELETE COLUMN use_internal;
