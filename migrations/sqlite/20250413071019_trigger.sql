-- Function for SQLite is implemented directly in the trigger
CREATE TRIGGER IF NOT EXISTS update_metrics_summary
    AFTER INSERT ON metrics
    FOR EACH ROW
BEGIN
    UPDATE metrics_summary 
    SET 
        total_keypresses = total_keypresses + NEW.keypresses,
        total_mouse_clicks = total_mouse_clicks + NEW.mouse_clicks,
        total_mouse_travel_in = total_mouse_travel_in + NEW.mouse_distance_in,
        total_mouse_travel_mi = total_mouse_travel_mi + NEW.mouse_distance_mi,
        total_scroll_steps = total_scroll_steps + NEW.scroll_steps,
        last_updated = CURRENT_TIMESTAMP
    WHERE id = 1;
END;