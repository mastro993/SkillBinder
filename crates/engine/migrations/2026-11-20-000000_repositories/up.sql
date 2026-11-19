-- Your repositories: Git repositories added to Explore, kept on this device only and never
-- synced. `normalized` is the URL as every comparison reads it, so one repository is added once;
-- `listing` holds the skills last listed, as JSON, with `listed_at`, so a repository offline or
-- restored by Undo keeps its skills without the network.
CREATE TABLE repositories (
    id TEXT PRIMARY KEY NOT NULL,
    url TEXT NOT NULL,
    normalized TEXT NOT NULL UNIQUE,
    added_at BIGINT NOT NULL,
    listing TEXT NOT NULL,
    listed_at BIGINT NOT NULL
);
