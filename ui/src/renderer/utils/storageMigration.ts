/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * Storage key migration: nomifun:* / nomifun.* → sparkfox:* / sparkfox.*
 *
 * One-time, idempotent migration of persisted localStorage keys from the
 * upstream NomiFun brand namespace to the SparkFox brand namespace. Call
 * `migrateStorageKeys()` once at app startup (before React.render) and
 * `migrateThemeKeys()` before the theme is applied (the theme keys are
 * read synchronously by index.html inline script — see note below).
 *
 * Migration strategy per key:
 *   - If the new key already exists → skip (do NOT overwrite the user's
 *     new value; the old key is also left in place so we don't delete
 *     data the user may still rely on).
 *   - If the old key exists and the new key does not → copy the value
 *     to the new key, then delete the old key.
 *   - If the old key does not exist → skip.
 *
 * Idempotency: a `MIGRATION_DONE_KEY` flag is written after the first
 * successful run. Subsequent calls short-circuit and return zero counts,
 * so the migration is safe to call on every startup.
 */

/**
 * Migration table: each entry maps an old key to its new key.
 *
 * Exported for test/sanity-check purposes (the canary test asserts that
 * the table covers every documented key).
 */
export const STORAGE_KEY_MIGRATIONS: ReadonlyArray<Readonly<{ from: string; to: string }>> = [
  { from: 'nomifun.emoji.recent', to: 'sparkfox.emoji.recent' },
  { from: 'nomifun:recent-workspaces', to: 'sparkfox:recent-workspaces' },
  { from: 'nomifun:recent-terminal-commands', to: 'sparkfox:recent-terminal-commands' },
  { from: 'nomifun:rail-width', to: 'sparkfox:rail-width' },
  { from: 'nomifun:qr-login-resume', to: 'sparkfox:qr-login-resume' },
  { from: 'nomifun:session-sider-collapsed', to: 'sparkfox:session-sider-collapsed' },
  { from: 'nomifun:session-sider-width', to: 'sparkfox:session-sider-width' },
  { from: 'nomifun:execution-canvas-width', to: 'sparkfox:execution-canvas-width' },
  { from: 'nomifun:e2e-message-stream-conversation-id', to: 'sparkfox:e2e-message-stream-conversation-id' },
  { from: 'nomifun:session-sidebar-display-preferences', to: 'sparkfox:session-sidebar-display-preferences' },
  { from: 'nomifun:workpath-pinned', to: 'sparkfox:workpath-pinned' },
  { from: 'nomifun:workpath-expansion', to: 'sparkfox:workpath-expansion' },
  { from: 'nomifun:workpath-subgroup-expansion', to: 'sparkfox:workpath-subgroup-expansion' },
  { from: 'nomifun:companion-group-expanded', to: 'sparkfox:companion-group-expanded' },
  { from: 'nomifun:session-list-project-workpaths', to: 'sparkfox:session-list-project-workpaths' },
  { from: 'nomifun:modelhub-sider-width', to: 'sparkfox:modelhub-sider-width' },
  { from: 'nomifun:requirements-sider-width', to: 'sparkfox:requirements-sider-width' },
  { from: 'nomifun.skillMarket.rankings.v3', to: 'sparkfox.skillMarket.rankings.v3' },
  { from: 'nomifun.skillMarket.autoSynced.v3', to: 'sparkfox.skillMarket.autoSynced.v3' },
  { from: 'nomifun.openclaw.monitorUrl', to: 'sparkfox.openclaw.monitorUrl' },
  { from: 'nomifun.starOffice.url', to: 'sparkfox.starOffice.url' },
];

/** Flag key written after the first successful migration run. */
const MIGRATION_DONE_KEY = 'sparkfox:storage-migration-v1-done';

/** Old → new theme keys used by index.html inline script. */
const THEME_KEY_MIGRATIONS: ReadonlyArray<Readonly<{ from: string; to: string }>> = [
  { from: '__nomifun_theme', to: '__sparkfox_theme' },
  { from: '__nomifun_colorScheme', to: '__sparkfox_colorScheme' },
];

/**
 * Migrate localStorage keys from the `nomifun:*` / `nomifun.*` namespace
 * to the `sparkfox:*` / `sparkfox.*` namespace.
 *
 * Idempotent: uses `MIGRATION_DONE_KEY` to short-circuit on repeat calls.
 *
 * @returns `{ migrated, skipped }` counts for observability.
 */
export function migrateStorageKeys(): { migrated: number; skipped: number } {
  // Idempotency guard: once the v1 migration has run, never run it again.
  // This prevents re-migrating stale `nomifun:*` keys that may reappear
  // (e.g. from a downgraded build) and clobbering fresh `sparkfox:*` data.
  if (localStorage.getItem(MIGRATION_DONE_KEY) === 'true') {
    return { migrated: 0, skipped: 0 };
  }

  let migrated = 0;
  let skipped = 0;

  for (const { from, to } of STORAGE_KEY_MIGRATIONS) {
    // New key already exists — never overwrite the user's fresh value.
    // Also leave the old key in place (don't delete data the user may
    // still rely on if they downgrade again).
    if (localStorage.getItem(to) !== null) {
      skipped++;
      continue;
    }
    const oldValue = localStorage.getItem(from);
    if (oldValue === null) {
      skipped++;
      continue;
    }
    localStorage.setItem(to, oldValue);
    localStorage.removeItem(from);
    migrated++;
  }

  // Mark migration complete (must run after the loop so a partial run
  // interrupted by an exception is retried on next startup).
  localStorage.setItem(MIGRATION_DONE_KEY, 'true');

  console.info(`[storageMigration] migrated ${migrated} key(s), skipped ${skipped}`);
  return { migrated, skipped };
}

/**
 * Migrate the theme keys read by index.html inline script.
 *
 * These keys are read synchronously during HTML parsing (before main.tsx
 * loads), so the very first app launch on the new build would otherwise
 * flash the default theme until `migrateStorageKeys()` runs. To avoid
 * that, index.html has been updated to read `__sparkfox_*` keys, and
 * this function copies any existing `__nomifun_*` values over before
 * React renders.
 *
 * Idempotent: safe to call on every startup. Does not overwrite existing
 * `__sparkfox_*` keys.
 */
export function migrateThemeKeys(): void {
  for (const { from, to } of THEME_KEY_MIGRATIONS) {
    if (localStorage.getItem(to) !== null) {
      // New key already exists — preserve user's fresh value.
      continue;
    }
    const oldValue = localStorage.getItem(from);
    if (oldValue === null) {
      continue;
    }
    localStorage.setItem(to, oldValue);
    localStorage.removeItem(from);
  }
}
