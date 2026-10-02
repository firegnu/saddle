CREATE TRIGGER no_closed_trace_update BEFORE UPDATE OF capture_enabled,generation,closed_at ON traces
WHEN OLD.closed_at IS NOT NULL
BEGIN SELECT RAISE(ABORT,'trace closed'); END;
CREATE TRIGGER no_closed_trace_event BEFORE INSERT ON events
WHEN EXISTS (SELECT 1 FROM traces WHERE trace_id=NEW.trace_id AND closed_at IS NOT NULL)
BEGIN SELECT RAISE(ABORT,'trace closed'); END;
CREATE TRIGGER no_closed_trace_dispatch BEFORE INSERT ON dispatches
WHEN EXISTS (SELECT 1 FROM traces WHERE trace_id=NEW.trace_id AND closed_at IS NOT NULL)
BEGIN SELECT RAISE(ABORT,'trace closed'); END;
