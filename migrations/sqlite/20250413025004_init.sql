-- Metrics Table (Individual Records)
CREATE TABLE IF NOT EXISTS metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT, -- SQLite equivalent of sequence/serial
    keypresses BIGINT NOT NULL,
    mouse_clicks BIGINT NOT NULL,
    mouse_distance_in REAL NOT NULL, -- Using REAL for double precision
    mouse_distance_mi REAL NOT NULL,
    scroll_steps BIGINT NOT NULL,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Summary Table (Single Row)
CREATE TABLE IF NOT EXISTS metrics_summary (
    id INTEGER PRIMARY KEY NOT NULL, -- Should always be 1
    last_updated DATETIME, -- SQLite uses DATETIME, can store timezone if needed
    total_keypresses BIGINT NOT NULL DEFAULT 0,
    total_mouse_clicks BIGINT NOT NULL DEFAULT 0,
    total_mouse_travel_in REAL NOT NULL DEFAULT 0,
    total_mouse_travel_mi REAL NOT NULL DEFAULT 0,
    total_scroll_steps BIGINT NOT NULL DEFAULT 0
);

-- Ensure the single summary row exists (ID = 1)
INSERT OR IGNORE INTO metrics_summary (id, last_updated, total_keypresses, total_mouse_clicks, total_scroll_steps, total_mouse_travel_in, total_mouse_travel_mi)
VALUES (1, CURRENT_TIMESTAMP, 0, 0, 0, 0.0, 0.0);
