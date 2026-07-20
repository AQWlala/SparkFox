/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * Tests for the one-time storage key migration (nomifun:* → sparkfox:*).
 * Run with: `bun test src/renderer/utils/storageMigration.test.ts`
 */

import { describe, test, expect } from 'bun:test';
import { migrateStorageKeys, migrateThemeKeys, STORAGE_KEY_MIGRATIONS } from './storageMigration';

/**
 * Install a fresh in-memory localStorage mock on globalThis.
 *
 * Bun's test environment does not provide a DOM, so `localStorage` is
 * undefined by default. This helper mirrors the pattern used by
 * `projectWorkpaths.test.ts` (with a `clear` method added so each test
 * starts from a clean slate). Returns a `restore` function that removes
 * the mock.
 */
const installStorage = () => {
  const originalLocalStorage = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
  const store = new Map<string, string>();
  const localStorageMock = {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => store.set(key, String(value)),
    removeItem: (key: string) => store.delete(key),
    clear: () => store.clear(),
  };
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    writable: true,
    value: localStorageMock,
  });

  return () => {
    if (originalLocalStorage) Object.defineProperty(globalThis, 'localStorage', originalLocalStorage);
    else Reflect.deleteProperty(globalThis, 'localStorage');
  };
};

describe('storageMigration', () => {
  test('STORAGE_KEY_MIGRATIONS covers all 21 documented keys', () => {
    // Sanity check: the migration table must cover every key documented
    // in the rename task. If a key is added/removed, this test forces
    // an explicit update.
    expect(STORAGE_KEY_MIGRATIONS).toHaveLength(21);
    expect(STORAGE_KEY_MIGRATIONS.every((m) => m.from.startsWith('nomifun'))).toBe(true);
    expect(STORAGE_KEY_MIGRATIONS.every((m) => m.to.startsWith('sparkfox'))).toBe(true);
  });

  test('migrates nomifun: keys to sparkfox:', () => {
    const restore = installStorage();
    try {
      localStorage.setItem('nomifun:rail-width', '300');
      localStorage.setItem('nomifun:workpath-pinned', '["a","b"]');
      const result = migrateStorageKeys();
      expect(result.migrated).toBe(2);
      expect(localStorage.getItem('sparkfox:rail-width')).toBe('300');
      expect(localStorage.getItem('sparkfox:workpath-pinned')).toBe('["a","b"]');
      expect(localStorage.getItem('nomifun:rail-width')).toBeNull();
      expect(localStorage.getItem('nomifun:workpath-pinned')).toBeNull();
    } finally {
      restore();
    }
  });

  test('migrates nomifun. (dot) keys to sparkfox.', () => {
    const restore = installStorage();
    try {
      localStorage.setItem('nomifun.emoji.recent', '["🎉"]');
      localStorage.setItem('nomifun.skillMarket.rankings.v3', '{"data":1}');
      const result = migrateStorageKeys();
      expect(result.migrated).toBe(2);
      expect(localStorage.getItem('sparkfox.emoji.recent')).toBe('["🎉"]');
      expect(localStorage.getItem('sparkfox.skillMarket.rankings.v3')).toBe('{"data":1}');
      expect(localStorage.getItem('nomifun.emoji.recent')).toBeNull();
      expect(localStorage.getItem('nomifun.skillMarket.rankings.v3')).toBeNull();
    } finally {
      restore();
    }
  });

  test('skips when new key already exists (does not overwrite)', () => {
    const restore = installStorage();
    try {
      localStorage.setItem('nomifun:rail-width', '300');
      localStorage.setItem('sparkfox:rail-width', '500'); // new value wins
      const result = migrateStorageKeys();
      expect(result.migrated).toBe(0);
      expect(result.skipped).toBeGreaterThanOrEqual(1);
      expect(localStorage.getItem('sparkfox:rail-width')).toBe('500');
      // Old key is preserved when skipped (not deleted)
      expect(localStorage.getItem('nomifun:rail-width')).toBe('300');
    } finally {
      restore();
    }
  });

  test('skips when old key does not exist', () => {
    const restore = installStorage();
    try {
      const result = migrateStorageKeys();
      expect(result.migrated).toBe(0);
    } finally {
      restore();
    }
  });

  test('idempotent: second call is a no-op (MIGRATION_DONE_KEY set)', () => {
    const restore = installStorage();
    try {
      localStorage.setItem('nomifun:rail-width', '300');
      const result1 = migrateStorageKeys();
      expect(result1.migrated).toBe(1);

      // Second call must be a no-op even if old key were re-added
      localStorage.setItem('nomifun:rail-width', '999');
      const result2 = migrateStorageKeys();
      expect(result2.migrated).toBe(0);
      expect(result2.skipped).toBe(0);
      // The re-added old key is NOT migrated (idempotent guard short-circuits)
      expect(localStorage.getItem('sparkfox:rail-width')).toBe('300');
    } finally {
      restore();
    }
  });

  test('marks migration done via MIGRATION_DONE_KEY', () => {
    const restore = installStorage();
    try {
      expect(localStorage.getItem('sparkfox:storage-migration-v1-done')).toBeNull();
      migrateStorageKeys();
      expect(localStorage.getItem('sparkfox:storage-migration-v1-done')).toBe('true');
    } finally {
      restore();
    }
  });

  test('migrateThemeKeys migrates __nomifun_theme and __nomifun_colorScheme', () => {
    const restore = installStorage();
    try {
      localStorage.setItem('__nomifun_theme', 'dark');
      localStorage.setItem('__nomifun_colorScheme', 'blue');
      migrateThemeKeys();
      expect(localStorage.getItem('__sparkfox_theme')).toBe('dark');
      expect(localStorage.getItem('__sparkfox_colorScheme')).toBe('blue');
      expect(localStorage.getItem('__nomifun_theme')).toBeNull();
      expect(localStorage.getItem('__nomifun_colorScheme')).toBeNull();
    } finally {
      restore();
    }
  });

  test('migrateThemeKeys does not overwrite existing new theme keys', () => {
    const restore = installStorage();
    try {
      localStorage.setItem('__nomifun_theme', 'dark');
      localStorage.setItem('__sparkfox_theme', 'light'); // existing wins
      migrateThemeKeys();
      expect(localStorage.getItem('__sparkfox_theme')).toBe('light');
      // Old key is preserved (not deleted) when skipped
      expect(localStorage.getItem('__nomifun_theme')).toBe('dark');
    } finally {
      restore();
    }
  });

  test('migrateThemeKeys is a no-op when old keys absent', () => {
    const restore = installStorage();
    try {
      migrateThemeKeys();
      expect(localStorage.getItem('__sparkfox_theme')).toBeNull();
      expect(localStorage.getItem('__sparkfox_colorScheme')).toBeNull();
    } finally {
      restore();
    }
  });
});
