CREATE TABLE recording_policy (
    singleton INTEGER PRIMARY KEY CHECK(singleton=1),
    enabled INTEGER NOT NULL CHECK(enabled IN (0,1)),
    generation INTEGER NOT NULL CHECK(generation>=0),
    updated_at TEXT NOT NULL
) STRICT;
CREATE TABLE traces (
    trace_id TEXT PRIMARY KEY,
    origin TEXT NOT NULL CHECK(origin IN ('task','ad_hoc')),
    created_at TEXT NOT NULL,
    capture_enabled INTEGER NOT NULL CHECK(capture_enabled IN (0,1)),
    generation INTEGER NOT NULL CHECK(generation>=0),
    binding_kind TEXT, binding_scope TEXT, binding_key TEXT, binding_run TEXT,
    record_json TEXT NOT NULL,
    CHECK ((binding_kind IS NULL AND binding_scope IS NULL AND binding_key IS NULL AND binding_run IS NULL)
        OR (binding_kind IS NOT NULL AND binding_scope IS NOT NULL AND binding_key IS NOT NULL AND binding_run IS NOT NULL)),
    CHECK (origin!='task' OR binding_kind IS NOT NULL),
    UNIQUE(binding_kind,binding_scope,binding_key,binding_run)
) STRICT;
CREATE TABLE dispatches (
    dispatch_id TEXT PRIMARY KEY,
    trace_id TEXT NOT NULL REFERENCES traces(trace_id),
    parent_dispatch_id TEXT,
    kind TEXT NOT NULL CHECK(kind IN ('implementation','review','redispatch','controller_handoff')),
    created_at TEXT NOT NULL,
    record_json TEXT NOT NULL,
    UNIQUE(dispatch_id,trace_id),
    CHECK(parent_dispatch_id IS NULL OR parent_dispatch_id!=dispatch_id),
    FOREIGN KEY(parent_dispatch_id,trace_id) REFERENCES dispatches(dispatch_id,trace_id)
) STRICT;
CREATE INDEX dispatch_trace ON dispatches(trace_id,parent_dispatch_id);
CREATE TABLE operations (
    operation_id TEXT PRIMARY KEY,
    trace_id TEXT NOT NULL,
    dispatch_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('route','agent.start','agent.send','agent.reply')),
    producer TEXT NOT NULL,
    global_generation INTEGER NOT NULL,
    trace_generation INTEGER NOT NULL,
    UNIQUE(operation_id,dispatch_id,trace_id),
    FOREIGN KEY(dispatch_id,trace_id) REFERENCES dispatches(dispatch_id,trace_id)
) STRICT;
CREATE TABLE events (
    seq INTEGER PRIMARY KEY,
    event_id TEXT NOT NULL UNIQUE,
    trace_id TEXT REFERENCES traces(trace_id),
    dispatch_id TEXT,
    operation_id TEXT,
    kind TEXT NOT NULL CHECK(kind IN ('requirement.recorded','proposal.recorded','authorization.recorded',
        'evidence.reused','controller.summary','brief.snapshot','route.begin','route.end','controller.decision',
        'agent.start.begin','agent.start.end','agent.send.begin','agent.send.end','agent.reply.begin','agent.reply.end',
        'review.recorded','controller.note','task.transition','recording.changed')),
    phase TEXT CHECK(phase IN ('begin','end')),
    recorded_at TEXT NOT NULL,
    observed_at TEXT,
    fingerprint TEXT NOT NULL,
    record_json TEXT NOT NULL,
    CHECK (trace_id IS NOT NULL OR (kind='recording.changed' AND dispatch_id IS NULL AND operation_id IS NULL)),
    CHECK (dispatch_id IS NULL OR trace_id IS NOT NULL),
    CHECK (operation_id IS NULL OR (dispatch_id IS NOT NULL AND trace_id IS NOT NULL)),
    CHECK (kind NOT IN ('requirement.recorded','proposal.recorded','authorization.recorded','evidence.reused','task.transition','recording.changed') OR dispatch_id IS NULL),
    CHECK (kind NOT IN ('controller.summary','controller.decision','review.recorded') OR dispatch_id IS NOT NULL),
    CHECK ((kind IN ('brief.snapshot','route.begin','route.end','agent.start.begin','agent.start.end','agent.send.begin',
        'agent.send.end','agent.reply.begin','agent.reply.end') AND operation_id IS NOT NULL)
        OR (kind NOT IN ('brief.snapshot','route.begin','route.end','agent.start.begin','agent.start.end','agent.send.begin',
        'agent.send.end','agent.reply.begin','agent.reply.end') AND operation_id IS NULL)),
    FOREIGN KEY(dispatch_id,trace_id) REFERENCES dispatches(dispatch_id,trace_id),
    FOREIGN KEY(operation_id,dispatch_id,trace_id) REFERENCES operations(operation_id,dispatch_id,trace_id)
) STRICT;
CREATE UNIQUE INDEX operation_phase ON events(operation_id,phase) WHERE phase IS NOT NULL;
CREATE INDEX events_trace ON events(trace_id,seq);
CREATE INDEX events_dispatch ON events(dispatch_id,seq);
CREATE INDEX events_kind ON events(kind,observed_at);
CREATE TABLE blobs (
    sha256 TEXT PRIMARY KEY CHECK(length(sha256)=64),
    bytes INTEGER NOT NULL CHECK(bytes>=0)
) STRICT;
CREATE TABLE event_blobs (
    event_id TEXT NOT NULL REFERENCES events(event_id),
    role TEXT NOT NULL,
    sha256 TEXT NOT NULL REFERENCES blobs(sha256),
    PRIMARY KEY(event_id,role)
) STRICT;
CREATE TABLE event_links (
    event_id TEXT NOT NULL REFERENCES events(event_id),
    relation TEXT NOT NULL CHECK(relation IN ('based_on','responds_to','supersedes','uses_brief','uses_decision','carried_from')),
    target_event_id TEXT NOT NULL REFERENCES events(event_id),
    PRIMARY KEY(event_id,relation,target_event_id),
    CHECK(event_id!=target_event_id)
) STRICT;
CREATE TRIGGER operation_event_kind BEFORE INSERT ON events
WHEN NEW.operation_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM operations WHERE operation_id=NEW.operation_id AND trace_id=NEW.trace_id AND dispatch_id=NEW.dispatch_id
    AND ((NEW.phase IS NOT NULL AND NEW.kind=kind || '.' || NEW.phase)
        OR (NEW.kind='brief.snapshot' AND NEW.phase IS NULL AND kind IN ('route','agent.start','agent.send')))
) BEGIN SELECT RAISE(ABORT,'operation kind mismatch'); END;
CREATE TRIGGER body_in_original_record BEFORE INSERT ON event_blobs
WHEN NOT EXISTS (
    SELECT 1 FROM events,json_each(events.record_json,'$.bodies') AS body,blobs
    WHERE events.event_id=NEW.event_id AND json_extract(body.value,'$.role')=NEW.role
    AND json_extract(body.value,'$.sha256')=NEW.sha256 AND blobs.sha256=NEW.sha256
    AND json_extract(body.value,'$.bytes')=blobs.bytes
) BEGIN SELECT RAISE(ABORT,'body reference was not in original event'); END;
CREATE TRIGGER link_in_original_record BEFORE INSERT ON event_links
WHEN NOT EXISTS (
    SELECT 1 FROM events AS source,events AS target,json_each(source.record_json,'$.links') AS link
    WHERE source.event_id=NEW.event_id AND target.event_id=NEW.target_event_id AND target.seq<source.seq
    AND json_extract(link.value,'$.relation')=NEW.relation
    AND json_extract(link.value,'$.target_event_id')=NEW.target_event_id
    AND (source.trace_id=target.trace_id OR (source.kind='evidence.reused' AND NEW.relation='carried_from'
        AND target.kind IN ('requirement.recorded','proposal.recorded','authorization.recorded','evidence.reused')))
) BEGIN SELECT RAISE(ABORT,'link was not in original event or target is invalid'); END;
CREATE TRIGGER no_event_update BEFORE UPDATE ON events BEGIN SELECT RAISE(ABORT,'immutable event'); END;
CREATE TRIGGER no_event_delete BEFORE DELETE ON events BEGIN SELECT RAISE(ABORT,'immutable event'); END;
CREATE TRIGGER no_blob_update BEFORE UPDATE ON blobs BEGIN SELECT RAISE(ABORT,'immutable blob'); END;
CREATE TRIGGER no_blob_delete BEFORE DELETE ON blobs BEGIN SELECT RAISE(ABORT,'immutable blob'); END;
CREATE TRIGGER no_ref_update BEFORE UPDATE ON event_blobs BEGIN SELECT RAISE(ABORT,'immutable body reference'); END;
CREATE TRIGGER no_ref_delete BEFORE DELETE ON event_blobs BEGIN SELECT RAISE(ABORT,'immutable body reference'); END;
CREATE TRIGGER no_link_update BEFORE UPDATE ON event_links BEGIN SELECT RAISE(ABORT,'immutable link'); END;
CREATE TRIGGER no_link_delete BEFORE DELETE ON event_links BEGIN SELECT RAISE(ABORT,'immutable link'); END;
CREATE TRIGGER no_dispatch_update BEFORE UPDATE ON dispatches BEGIN SELECT RAISE(ABORT,'immutable dispatch'); END;
CREATE TRIGGER no_dispatch_delete BEFORE DELETE ON dispatches BEGIN SELECT RAISE(ABORT,'immutable dispatch'); END;
CREATE TRIGGER no_operation_update BEFORE UPDATE ON operations BEGIN SELECT RAISE(ABORT,'immutable operation'); END;
CREATE TRIGGER no_operation_delete BEFORE DELETE ON operations BEGIN SELECT RAISE(ABORT,'immutable operation'); END;
CREATE TRIGGER no_trace_identity_update BEFORE UPDATE OF trace_id,origin,created_at,binding_kind,binding_scope,binding_key,binding_run,record_json ON traces
    BEGIN SELECT RAISE(ABORT,'immutable trace identity'); END;
CREATE TRIGGER no_trace_delete BEFORE DELETE ON traces BEGIN SELECT RAISE(ABORT,'immutable trace'); END;
PRAGMA user_version=1;
