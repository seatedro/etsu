-- Metrics Table (Individual Records)
CREATE TABLE IF NOT EXISTS metrics (
    id SERIAL PRIMARY KEY,
    keypresses BIGINT NOT NULL,
    mouse_clicks BIGINT NOT NULL,
    mouse_distance_in DOUBLE PRECISION NOT NULL,
    mouse_distance_mi DOUBLE PRECISION NOT NULL,
    scroll_steps BIGINT NOT NULL,
    timestamp TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Summary Table (Single Row)
CREATE TABLE IF NOT EXISTS metrics_summary (
    id INTEGER PRIMARY KEY NOT NULL,
    last_updated TIMESTAMP WITH TIME ZONE,
    total_keypresses BIGINT NOT NULL DEFAULT 0,
    total_mouse_clicks BIGINT NOT NULL DEFAULT 0,
    total_mouse_travel_in DOUBLE PRECISION NOT NULL DEFAULT 0,
    total_mouse_travel_mi DOUBLE PRECISION NOT NULL DEFAULT 0,
    total_scroll_steps BIGINT NOT NULL DEFAULT 0
);

-- Ensure the single summary row exists (ID = 1)
INSERT INTO metrics_summary (id, last_updated, total_keypresses, total_mouse_clicks, total_scroll_steps, total_mouse_travel_in, total_mouse_travel_mi)
VALUES (1, CURRENT_TIMESTAMP, 0, 0, 0, 0.0, 0.0)
ON CONFLICT (id) DO NOTHING;

