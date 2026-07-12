//! Database schema definitions.

/// SQL statements that create the Fokus database schema.
/// Uses `IF NOT EXISTS` for safe startup execution.
pub const SCHEMA_SQL: &str = r#"
-- Sessions — Merged time blocks
CREATE TABLE IF NOT EXISTS sessions (
    id                  TEXT PRIMARY KEY,    -- UUID v4
    start_time          TEXT NOT NULL,       -- ISO 8601 datetime
    end_time            TEXT,                -- NULL if session is still active
    app_name            TEXT NOT NULL,
    window_title        TEXT NOT NULL,       -- Last observed title
    category            TEXT NOT NULL,       -- JSON-encoded Category enum
    url                 TEXT,
    activity_count      INTEGER NOT NULL DEFAULT 1,
    idle_seconds_total  INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_sessions_time_category
    ON sessions(start_time, category);

-- Rules — Classification rules
CREATE TABLE IF NOT EXISTS rules (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    pattern     TEXT NOT NULL,       -- Substring to match (case-insensitive)
    target      TEXT NOT NULL,       -- "app_name", "window_title", or "url"
    category    TEXT NOT NULL,       -- JSON-encoded Category
    priority    INTEGER NOT NULL DEFAULT 100,
    enabled     INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_rules_priority
    ON rules(priority);

-- Daily Rollups — Pre-aggregated daily summaries
CREATE TABLE IF NOT EXISTS daily_rollups (
    date            TEXT NOT NULL,       -- "YYYY-MM-DD" format
    category        TEXT NOT NULL,       -- JSON-encoded Category
    total_seconds   INTEGER NOT NULL DEFAULT 0,
    session_count   INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (date, category)         -- One row per date + category combo
);

CREATE INDEX IF NOT EXISTS idx_daily_rollups_date
    ON daily_rollups(date);

-- Settings — Application settings
CREATE TABLE IF NOT EXISTS settings (
    key     TEXT PRIMARY KEY,
    value   TEXT NOT NULL            -- JSON-encoded value
);
"#;

/// Default classification rules for a fresh installation.
pub const DEFAULT_RULES_SQL: &str = r#"
INSERT OR IGNORE INTO rules (id, name, pattern, target, category, priority, enabled) VALUES
    -- =================================================================
    -- IDEs and code editors → Coding (app_name based)
    -- =================================================================
    ('default-001', 'VS Code → Coding',           'Code',            'app_name',     '"coding"',       10, 1),
    ('default-002', 'IntelliJ → Coding',           'idea',            'app_name',     '"coding"',       10, 1),
    ('default-003', 'WebStorm → Coding',           'webstorm',        'app_name',     '"coding"',       10, 1),
    ('default-004', 'Terminal → Coding',            'WindowsTerminal', 'app_name',     '"coding"',       20, 1),
    ('default-005', 'PowerShell → Coding',          'powershell',      'app_name',     '"coding"',       20, 1),
    ('default-006', 'cmd → Coding',                 'cmd',             'app_name',     '"coding"',       20, 1),
    ('default-007', 'Git in title → Coding',        'git',             'window_title', '"coding"',       20, 1),
    ('default-008', 'Sublime Text → Coding',        'sublime_text',    'app_name',     '"coding"',       10, 1),
    ('default-009', 'Vim/Neovim → Coding',          'nvim',            'app_name',     '"coding"',       10, 1),
    ('default-00a', 'PyCharm → Coding',             'pycharm',         'app_name',     '"coding"',       10, 1),
    ('default-00b', 'Cursor → Coding',              'Cursor',          'app_name',     '"coding"',       10, 1),

    -- =================================================================
    -- Learning platforms → Study
    -- Window title rules work WITHOUT the browser extension because
    -- Chrome's title bar shows: "Coursera | ML Course - Google Chrome"
    -- =================================================================
    ('default-010', 'Coursera (url) → Study',       'coursera',        'url',          '"study"',        10, 1),
    ('default-011', 'Udemy (url) → Study',           'udemy',           'url',          '"study"',        10, 1),
    ('default-012', 'Khan Academy (url) → Study',    'khanacademy',     'url',          '"study"',        10, 1),
    ('default-013', 'edX (url) → Study',             'edx.org',         'url',          '"study"',        10, 1),
    ('default-014', 'LeetCode (url) → Study',        'leetcode',        'url',          '"study"',        15, 1),
    -- Title-based fallbacks (work without extension)
    ('default-015', 'Coursera (title) → Study',      'coursera',        'window_title', '"study"',        12, 1),
    ('default-016', 'Udemy (title) → Study',          'udemy',           'window_title', '"study"',        12, 1),
    ('default-017', 'Khan Academy (title) → Study',   'khan academy',    'window_title', '"study"',        12, 1),
    ('default-018', 'LeetCode (title) → Study',       'leetcode',        'window_title', '"study"',        17, 1),
    ('default-019', 'edX (title) → Study',            'edx',             'window_title', '"study"',        12, 1),

    -- =================================================================
    -- Note-taking → NoteTaking
    -- =================================================================
    ('default-020', 'Notion (url) → Note-taking',     'notion',          'url',          '"note_taking"',   10, 1),
    ('default-021', 'Obsidian → Note-taking',          'Obsidian',        'app_name',     '"note_taking"',   10, 1),
    ('default-022', 'OneNote → Note-taking',           'OneNote',         'app_name',     '"note_taking"',   10, 1),
    ('default-023', 'Notion (title) → Note-taking',    'Notion',          'window_title', '"note_taking"',   12, 1),

    -- =================================================================
    -- Developer tools → Coding (title-based for browsers)
    -- =================================================================
    ('default-030', 'GitHub (url) → Coding',           'github.com',      'url',          '"coding"',       15, 1),
    ('default-031', 'GitLab (url) → Coding',           'gitlab.com',      'url',          '"coding"',       15, 1),
    ('default-032', 'Stack Overflow (url) → Coding',   'stackoverflow',   'url',          '"coding"',       15, 1),
    ('default-033', 'GitHub (title) → Coding',         'GitHub',          'window_title', '"coding"',       17, 1),
    ('default-034', 'Stack Overflow (title) → Coding', 'Stack Overflow',  'window_title', '"coding"',       17, 1),
    ('default-035', 'MDN (title) → Coding',            'MDN',             'window_title', '"coding"',       17, 1),
    ('default-036', 'Rust docs (title) → Coding',      'docs.rs',         'window_title', '"coding"',       17, 1),
    ('default-037', 'npm (title) → Coding',            'npm',             'window_title', '"coding"',       25, 1),

    -- =================================================================
    -- Communication → Communication
    -- =================================================================
    ('default-040', 'Slack → Communication',            'Slack',           'app_name',     '"communication"', 20, 1),
    ('default-041', 'Discord → Communication',          'Discord',         'app_name',     '"communication"', 20, 1),
    ('default-042', 'Teams → Communication',            'Teams',           'app_name',     '"communication"', 20, 1),
    ('default-043', 'WhatsApp → Communication',         'WhatsApp',        'app_name',     '"communication"', 20, 1),
    ('default-044', 'Telegram → Communication',         'Telegram',        'app_name',     '"communication"', 20, 1),
    ('default-045', 'Zoom → Communication',             'Zoom',            'app_name',     '"communication"', 20, 1),
    ('default-046', 'WhatsApp (title) → Communication', 'WhatsApp',        'window_title', '"communication"', 22, 1),

    -- =================================================================
    -- Productivity → Productive
    -- =================================================================
    ('default-050', 'Word → Productive',                'WINWORD',         'app_name',     '"productive"',   30, 1),
    ('default-051', 'Excel → Productive',               'EXCEL',           'app_name',     '"productive"',   30, 1),
    ('default-052', 'PowerPoint → Productive',          'POWERPNT',        'app_name',     '"productive"',   30, 1),
    ('default-053', 'Google Docs (url) → Productive',   'docs.google',     'url',          '"productive"',   30, 1),
    ('default-054', 'Google Docs (title) → Productive', 'Google Docs',     'window_title', '"productive"',   32, 1),
    ('default-055', 'Google Sheets (title) → Productive','Google Sheets',  'window_title', '"productive"',   32, 1),
    ('default-056', 'Acrobat → Productive',             'Acrobat',         'app_name',     '"productive"',   30, 1),
    ('default-057', 'File Explorer → Productive',       'explorer',        'app_name',     '"productive"',   40, 1),

    -- =================================================================
    -- Browsers → Productive (low priority, overridden by specific title rules)
    -- Without the extension, we at least know the user is browsing.
    -- Specific window_title rules above will override this for known sites.
    -- =================================================================
    ('default-070', 'Chrome → Productive',              'chrome',          'app_name',     '"productive"',   90, 1),
    ('default-071', 'Firefox → Productive',             'firefox',         'app_name',     '"productive"',   90, 1),
    ('default-072', 'Edge → Productive',                'msedge',          'app_name',     '"productive"',   90, 1),
    ('default-073', 'Brave → Productive',               'brave',           'app_name',     '"productive"',   90, 1),
    ('default-074', 'Opera → Productive',               'opera',           'app_name',     '"productive"',   90, 1),
    ('default-075', 'Safari → Productive',              'safari',          'app_name',     '"productive"',   90, 1),

    -- =================================================================
    -- Entertainment → Entertainment (title-based for browser content)
    -- These must have LOWER priority (higher number) than Study rules
    -- so that "YouTube - Rust tutorial" can be overridden to Study.
    -- =================================================================
    ('default-060', 'YouTube (url) → Entertainment',    'youtube.com',     'url',          '"entertainment"', 50, 1),
    ('default-061', 'Netflix (url) → Entertainment',    'netflix',         'url',          '"entertainment"', 50, 1),
    ('default-062', 'Twitter/X (url) → Entertainment',  'twitter.com',     'url',          '"entertainment"', 50, 1),
    ('default-063', 'Reddit (url) → Entertainment',     'reddit.com',      'url',          '"entertainment"', 50, 1),
    ('default-064', 'Twitch (url) → Entertainment',     'twitch.tv',       'url',          '"entertainment"', 50, 1),
    -- Title-based fallbacks
    ('default-065', 'YouTube (title) → Entertainment',  'YouTube',         'window_title', '"entertainment"', 52, 1),
    ('default-066', 'Netflix (title) → Entertainment',  'Netflix',         'window_title', '"entertainment"', 52, 1),
    ('default-067', 'Twitter/X (title) → Entertainment','Twitter',         'window_title', '"entertainment"', 52, 1),
    ('default-068', 'Reddit (title) → Entertainment',   'Reddit',          'window_title', '"entertainment"', 52, 1),
    ('default-069', 'Twitch (title) → Entertainment',   'Twitch',          'window_title', '"entertainment"', 52, 1),
    ('default-06a', 'Instagram (title) → Entertainment','Instagram',       'window_title', '"entertainment"', 52, 1),
    ('default-06b', 'TikTok (title) → Entertainment',   'TikTok',          'window_title', '"entertainment"', 52, 1),
    ('default-06c', 'Spotify → Entertainment',           'Spotify',         'app_name',     '"entertainment"', 50, 1),

    -- =================================================================
    -- Desktop apps → Productive / Communication (catch common apps)
    -- =================================================================
    -- =================================================================
    -- macOS built-in apps (also applied to existing DBs via RULES_V2_SQL)
    -- =================================================================
    ('default-090', 'Mail → Productive',                 'mail',            'app_name',     '"productive"',   30, 1),
    ('default-091', 'Calendar → Productive',             'calendar',        'app_name',     '"productive"',   30, 1),
    ('default-092', 'Notes → Note-taking',               'notes',           'app_name',     '"note_taking"',  20, 1),
    ('default-093', 'Preview → Productive',              'preview',         'app_name',     '"productive"',   40, 1),
    ('default-094', 'Finder → Productive',               'finder',          'app_name',     '"productive"',   40, 1),
    ('default-095', 'Messages → Communication',          'messages',        'app_name',     '"communication"',20, 1),
    ('default-096', 'FaceTime → Communication',          'facetime',        'app_name',     '"communication"',20, 1),
    ('default-097', 'Music → Entertainment',             'music',           'app_name',     '"entertainment"',50, 1),
    ('default-098', 'Podcasts → Entertainment',          'podcasts',        'app_name',     '"entertainment"',50, 1),
    ('default-099', 'Xcode → Coding',                    'xcode',           'app_name',     '"coding"',       10, 1),

    ('default-080', 'Claude → Productive',               'claude',          'app_name',     '"productive"',   30, 1),
    ('default-081', 'ChatGPT (title) → Productive',      'ChatGPT',         'window_title', '"productive"',   32, 1),
    ('default-082', 'Figma → Productive',                 'Figma',           'app_name',     '"productive"',   30, 1),
    ('default-083', 'Postman → Coding',                   'Postman',         'app_name',     '"coding"',       20, 1),
    ('default-084', 'Docker → Coding',                    'Docker',          'app_name',     '"coding"',       20, 1);
"#;

/// Rules added after the initial release, applied to EXISTING databases via
/// a versioned migration (see `rules_seed_version` handling in database.rs).
/// INSERT OR IGNORE on fixed ids keeps this idempotent; because the batch
/// only runs once per version bump, rules the user deletes stay deleted.
pub const RULES_V2_SQL: &str = r#"
INSERT OR IGNORE INTO rules (id, name, pattern, target, category, priority, enabled) VALUES
    ('default-075', 'Safari → Productive',       'safari',   'app_name', '"productive"',    90, 1),
    ('default-090', 'Mail → Productive',         'mail',     'app_name', '"productive"',    30, 1),
    ('default-091', 'Calendar → Productive',     'calendar', 'app_name', '"productive"',    30, 1),
    ('default-092', 'Notes → Note-taking',       'notes',    'app_name', '"note_taking"',   20, 1),
    ('default-093', 'Preview → Productive',      'preview',  'app_name', '"productive"',    40, 1),
    ('default-094', 'Finder → Productive',       'finder',   'app_name', '"productive"',    40, 1),
    ('default-095', 'Messages → Communication',  'messages', 'app_name', '"communication"', 20, 1),
    ('default-096', 'FaceTime → Communication',  'facetime', 'app_name', '"communication"', 20, 1),
    ('default-097', 'Music → Entertainment',     'music',    'app_name', '"entertainment"', 50, 1),
    ('default-098', 'Podcasts → Entertainment',  'podcasts', 'app_name', '"entertainment"', 50, 1),
    ('default-099', 'Xcode → Coding',            'xcode',    'app_name', '"coding"',        10, 1);
"#;
