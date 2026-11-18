#!/usr/bin/env node
// The native declaration is a generated artifact, but the WASI declaration is
// produced by `just build-rolldown-wasi` in the dedicated WASI lane, which does
// not check for a dirty tree. The OHOS fork adds the `ets-static` member to the
// `lang` unions, and that delta silently drifted out of
// `rolldown-binding.wasi.d.cts` once. Guard the small part both files must
// agree on in the normal lanes without building the wasm artifact.

import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const nativePath = resolve(repoRoot, 'packages/rolldown/src/binding.d.cts');
const wasiPath = resolve(repoRoot, 'packages/rolldown/src/rolldown-binding.wasi.d.cts');

/**
 * Collect every `lang` declaration together with the interface that owns it.
 * @param {string} source
 * @returns {{ interfaceName: string, union: string }[]}
 */
function collectLangFields(source) {
  const fields = [];
  let interfaceName = '(top-level)';
  for (const line of source.split('\n')) {
    const interfaceMatch = line.match(/^export interface (\w+)/);
    if (interfaceMatch) {
      interfaceName = interfaceMatch[1];
    }
    const langMatch = line.match(/^\s*lang\?:\s*(.+?)\s*$/);
    if (langMatch) {
      fields.push({ interfaceName, union: langMatch[1] });
    }
  }
  return fields;
}

const format = (fields) =>
  fields.map((field) => `  ${field.interfaceName}: lang?: ${field.union}`).join('\n');

const native = collectLangFields(readFileSync(nativePath, 'utf8'));
const wasi = collectLangFields(readFileSync(wasiPath, 'utf8'));

const same =
  native.length === wasi.length &&
  native.every(
    (field, index) =>
      field.interfaceName === wasi[index].interfaceName && field.union === wasi[index].union,
  );

if (!same) {
  console.error('WASI binding declarations drifted from the native ones for `lang`.');
  console.error(`\nNative (${native.length}):\n${format(native)}`);
  console.error(`\nWASI (${wasi.length}):\n${format(wasi)}`);
  console.error(
    '\nRegenerate the WASI declaration with `just build-rolldown-wasi`, or sync the fields manually, so both declarations expose the OHOS `ets-static` unions.',
  );
  process.exit(1);
}

console.log(
  `WASI binding declarations expose the same ${native.length} \`lang\` unions as native.`,
);
