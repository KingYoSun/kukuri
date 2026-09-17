-- #1060: a cached successful probe is valid only for the same model/extraction configuration.
ALTER TABLE cn_admin.readiness_probe_cache ADD COLUMN configuration_fingerprint TEXT NULL;
