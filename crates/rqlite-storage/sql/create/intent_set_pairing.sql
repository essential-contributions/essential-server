CREATE TABLE IF NOT EXISTS intent_set_pairing (
    id INTEGER PRIMARY KEY,
    set_id INTEGER NOT NULL,
    intent_id INTEGER NOT NULL,
    FOREIGN KEY (set_id) REFERENCES intent_sets (id),
    FOREIGN KEY (intent_id) REFERENCES intents (id),
    UNIQUE(set_id, intent_id)
);