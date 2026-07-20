/**
 * @license
 * Copyright 2026 SparkFox Contributors — AGPL-3.0-only
 *
 * Naming consistency canary: ensures renderer-internal identifiers and
 * user-visible brand strings have been migrated from the upstream NomiFun
 * brand to the SparkFox brand.
 *
 * Scope: only catches identifiers that are safe to rename (no API contract,
 * no persisted user data). External contracts (copyright headers,
 * `agent_type === 'nomi'`, MCP tool names, Cookie names, i18n keys) are out
 * of scope. CSS class names are in scope — the coordinated TSX↔CSS rename
 * is enforced by dedicated `no_nomifun_css_class_in_*` test cases below.
 *
 * Test files (*.test.ts / *.test.tsx) are excluded from the scan because
 * they may legitimately reference old identifiers as part of regression
 * assertions.
 */

import { describe, test, expect } from 'bun:test';
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { join, basename } from 'node:path';

const ROOT = join(__dirname, '..'); // → src/renderer
const INDEX_HTML = join(ROOT, 'index.html');

function listSourceFiles(dir: string, exts: string[] = ['.ts', '.tsx']): string[] {
  const result: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory() && entry.name !== 'node_modules' && entry.name !== '__tests__') {
      result.push(...listSourceFiles(full, exts));
    } else if (exts.some((ext) => entry.name.endsWith(ext))) {
      // Skip test files — they may legitimately reference old identifiers
      // as part of regression assertions (e.g. "source must not contain X").
      if (entry.name.endsWith('.test.ts') || entry.name.endsWith('.test.tsx')) continue;
      result.push(full);
    }
  }
  return result;
}

/**
 * Match single/double/backtick-quoted string literals whose payload starts
 * with `prefix`. Returns the full literal including surrounding quotes.
 */
function findQuotedStringsStartingWith(content: string, prefix: string): string[] {
  const escaped = prefix.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const regex = new RegExp(`(['"\`])${escaped}[^'"\\\n]*?\\1`, 'g');
  return content.match(regex) ?? [];
}

/**
 * Quoted string literals that must be preserved as-is. Renaming would either:
 *   (a) break backward compatibility with already-exported user files
 *       (migration archive filename prefixes),
 *   (b) mis-reference an upstream source path or backend service identifier
 *       in JSDoc.
 *
 * Note: localStorage/sessionStorage keys and their paired CustomEvent names
 * were previously preserved here, but have since been migrated to the
 * `sparkfox:*` / `sparkfox.*` namespace with a one-time backward-compat
 * migration script (`utils/storageMigration.ts`). Their absence of
 * `nomifun:*` / `nomifun.*` prefixes is now enforced by dedicated tests:
 *   - `no nomifun: or nomifun. storage keys in source`
 *   - `no nomifun: CustomEvent names in source`
 *
 * CSS class names are NOT preserved here — the coordinated TSX↔CSS rename
 * is enforced by the `no_nomifun_css_class_in_tsx` and
 * `no_nomifun_css_class_in_css` test cases below.
 *
 * Each entry is the full quoted literal (with surrounding quotes) so it can
 * be matched exactly against the regex captures. Backtick-quoted JSDoc
 * references to the same keys are listed separately with backticks.
 */
const PRESERVED_QUOTED_STRINGS = [
  // ---- (a) Migration archive filename prefixes (preserve for backward compat) ----
  // Existing user-exported migration archives use these prefixes; renaming
  // would orphan previously-exported files.
  '`nomifun-memory-${today()}.zip`', // MigrateTab.tsx — memory export filename
  '`nomifun-companion-${safeName(companion.name)}-${today()}.zip`', // MigrateTab.tsx — companion export filename

  // ---- (b) JSDoc backtick references (documentation, not active code) ----
  // Backtick-quoted references in JSDoc comments pointing at:
  //   - backend service identifiers (workshop/api.ts, types.ts)
  //   - upstream Rust source path (detectFamily.ts)
  '`nomifun-workshop`', // workshop/api.ts + types.ts — backend service name in JSDoc
  '`nomifun-creation`', // workshop/api.ts + types.ts — backend service name in JSDoc
  '`nomifun-terminal/src/enhance.rs`', // detectFamily.ts — upstream Rust source path in JSDoc
];

describe('naming consistency: nomi → sparkfox', () => {
  test('no nomifun- prefix in renderer-internal event/cache/MIME strings', () => {
    const files = listSourceFiles(ROOT);
    const violations: string[] = [];
    for (const file of files) {
      const content = readFileSync(file, 'utf8');
      const matches = findQuotedStringsStartingWith(content, 'nomifun-');
      const problematic = matches.filter((m) => !PRESERVED_QUOTED_STRINGS.includes(m));
      if (problematic.length > 0) {
        violations.push(`${file}: ${problematic.join(', ')}`);
      }
    }
    expect(violations).toEqual([]);
  });

  test('no nomifun: prefix in renderer-internal CustomEvent names', () => {
    const files = listSourceFiles(ROOT);
    const violations: string[] = [];
    for (const file of files) {
      // Skip migration script — it legitimately references old key names
      if (file.includes('storageMigration')) continue;
      const content = readFileSync(file, 'utf8');
      const matches = findQuotedStringsStartingWith(content, 'nomifun:');
      const problematic = matches.filter((m) => !PRESERVED_QUOTED_STRINGS.includes(m));
      if (problematic.length > 0) {
        violations.push(`${file}: ${problematic.join(', ')}`);
      }
    }
    expect(violations).toEqual([]);
  });

  test('no nomifun:// prefix in renderer-internal Tauri event names', () => {
    const files = listSourceFiles(ROOT);
    const violations: string[] = [];
    for (const file of files) {
      const content = readFileSync(file, 'utf8');
      const matches = findQuotedStringsStartingWith(content, 'nomifun://');
      if (matches.length > 0) {
        violations.push(`${file}: ${matches.join(', ')}`);
      }
    }
    expect(violations).toEqual([]);
  });

  test('no __Nomi window globals (use __Spark prefix instead)', () => {
    const files = listSourceFiles(ROOT);
    const violations: string[] = [];
    for (const file of files) {
      const content = readFileSync(file, 'utf8');
      const matches = content.match(/__Nomi[A-Z][a-zA-Z]*__/g);
      if (matches) {
        violations.push(`${file}: ${matches.join(', ')}`);
      }
    }
    expect(violations).toEqual([]);
  });

  test('no application/x-nomifun MIME types in source', () => {
    const files = listSourceFiles(ROOT);
    const violations: string[] = [];
    for (const file of files) {
      const content = readFileSync(file, 'utf8');
      const matches = content.match(/application\/x-nomifun-[a-z-]+/g);
      if (matches) {
        violations.push(`${file}: ${matches.join(', ')}`);
      }
    }
    expect(violations).toEqual([]);
  });

  test('index.html uses SparkFox brand in title and meta', () => {
    expect(existsSync(INDEX_HTML)).toBe(true);
    const content = readFileSync(INDEX_HTML, 'utf8');
    expect(content.includes('<title>SparkFox</title>')).toBe(true);
    expect(content.includes('application-name" content="SparkFox"')).toBe(true);
    expect(content.includes('apple-mobile-web-app-title" content="SparkFox"')).toBe(true);
    expect(content.includes('<title>NomiFun</title>')).toBe(false);
    expect(content.includes('application-name" content="NomiFun"')).toBe(false);
    expect(content.includes('apple-mobile-web-app-title" content="NomiFun"')).toBe(false);
  });

  /**
   * Coordinated TSX↔CSS class rename: every `nomifun-*` CSS class reference
   * in .ts/.tsx source must be migrated to `sparkfox-*`. Whitelisted
   * non-CSS strings (localStorage keys, JSDoc references, migration
   * filenames) are still allowed via PRESERVED_QUOTED_STRINGS.
   */
  test('no_nomifun_css_class_in_tsx: no nomifun- CSS class references in .ts/.tsx', () => {
    const files = listSourceFiles(ROOT, ['.ts', '.tsx']);
    const violations: string[] = [];
    for (const file of files) {
      const content = readFileSync(file, 'utf8');
      const matches = findQuotedStringsStartingWith(content, 'nomifun-');
      const problematic = matches.filter((m) => !PRESERVED_QUOTED_STRINGS.includes(m));
      if (problematic.length > 0) {
        violations.push(`${file}: ${problematic.join(', ')}`);
      }
    }
    expect(violations).toEqual([]);
  });

  /**
   * Coordinated TSX↔CSS class rename: every `.nomifun-*` class selector in
   * .css files must be migrated to `.sparkfox-*`. CSS files don't carry
   * JSDoc or persisted-key references, so any `.nomifun-` selector is a
   * violation.
   */
  test('no_nomifun_css_class_in_css: no .nomifun- class selectors in .css files', () => {
    const cssFiles = listSourceFiles(ROOT, ['.css']);
    const violations: string[] = [];
    for (const file of cssFiles) {
      const content = readFileSync(file, 'utf8');
      const matches = content.match(/\.nomifun-[a-z][a-z0-9-]*/g);
      if (matches && matches.length > 0) {
        violations.push(`${file}: ${matches.join(', ')}`);
      }
    }
    expect(violations).toEqual([]);
  });

  /**
   * Brand-level file rename: generic UI primitives under `components/base/`
   * must use the `Spark*` filename (brand identifier). The `Nomi*` filename
   * is the upstream brand and must not appear in this directory.
   *
   * Note: `pages/conversation/platforms/nomi/Nomi*` is intentionally NOT
   * covered here — that directory is a platform adapter whose `nomi` segment
   * matches the `agent_type === 'nomi'` API contract and is out of scope.
   */
  test('no Nomi* component files in components/base/', () => {
    const baseDir = join(ROOT, 'components', 'base');
    const files = listSourceFiles(baseDir, ['.tsx']);
    const violations = files.filter((f) => /Nomi[A-Z]/.test(basename(f)));
    expect(violations).toEqual([]);
  });

  /**
   * Brand-level import-path rename: imports of the renamed Spark* primitives
   * must use the new `Spark*` path. lingering `from '.../NomiModal'` style
   * imports would break at resolver level after the file rename.
   *
   * Component identifier names (e.g. `NomiModal`) are intentionally NOT
   * flagged here — they are code identifiers, not brand identifiers, and
   * will be migrated in a follow-up PR to keep this change scoped.
   */
  test('no NomiModal/NomiSteps/NomiSelect/NomiScrollArea/NomiCollapse import paths', () => {
    const files = listSourceFiles(ROOT);
    const violations: string[] = [];
    const PATTERNS = [
      /from\s+['"][^'"]*\/NomiModal(\.tsx)?['"]/,
      /from\s+['"][^'"]*\/NomiSteps(\.tsx)?['"]/,
      /from\s+['"][^'"]*\/NomiSelect(\.tsx)?['"]/,
      /from\s+['"][^'"]*\/NomiScrollArea(\.tsx)?['"]/,
      /from\s+['"][^'"]*\/NomiCollapse(\.tsx)?['"]/,
    ];
    for (const file of files) {
      const content = readFileSync(file, 'utf8');
      for (const pattern of PATTERNS) {
        if (pattern.test(content)) {
          violations.push(`${file}: matches ${pattern}`);
        }
      }
    }
    expect(violations).toEqual([]);
  });

  /**
   * Storage key migration: all localStorage/sessionStorage keys must use the
   * `sparkfox:*` or `sparkfox.*` namespace. The old `nomifun:*` / `nomifun.*`
   * prefixes are migrated at app startup by `utils/storageMigration.ts`
   * (which legitimately references the old key names — skipped here).
   */
  test('no nomifun: or nomifun. storage keys in source', () => {
    const files = listSourceFiles(ROOT);
    const violations: string[] = [];
    for (const file of files) {
      // Skip canary itself and migration script (legitimately references old keys)
      if (file.includes('__tests__') || file.includes('storageMigration')) continue;
      const content = readFileSync(file, 'utf8');
      // Detect 'nomifun:' or 'nomifun.' as storage key prefix (quoted forms)
      const matches = content.match(/['"`]nomifun[:.][a-z]/g) || [];
      if (matches.length > 0) {
        violations.push(`${file}: ${matches.join(', ')}`);
      }
    }
    expect(violations).toEqual([]);
  });

  test('no nomifun: CustomEvent names in source', () => {
    const files = listSourceFiles(ROOT);
    const violations: string[] = [];
    const EVENT_PATTERNS = [
      'nomifun:session-sidebar-display-preferences-change',
      'nomifun:workpath-ui-changed',
      'nomifun:session-list-project-workpaths-changed',
    ];
    for (const file of files) {
      if (file.includes('__tests__') || file.includes('storageMigration')) continue;
      const content = readFileSync(file, 'utf8');
      for (const pattern of EVENT_PATTERNS) {
        if (content.includes(pattern)) {
          violations.push(`${file}: ${pattern}`);
        }
      }
    }
    expect(violations).toEqual([]);
  });
});
