/**
 * Shared helper for loading a Zod schema from a TypeScript source file
 * and writing its JSON Schema representation to a file.
 */

import { existsSync, writeFileSync } from 'node:fs';
import consola from 'consola';
import { createJiti } from 'jiti';

export interface GenerateJsonSchemaOptions {
  inputPath: string;
  outputPath: string;
  exportName: string;
}

export async function generateJsonSchema({ inputPath, outputPath, exportName }: GenerateJsonSchemaOptions): Promise<void> {
  if (!existsSync(inputPath)) {
    consola.error(
      `Schema source file not found: ${inputPath}\n` +
        'Make sure the file exists at the expected path.',
    );
    process.exit(1);
  }

  consola.start(`Loading ${inputPath}...`);

  const jiti = createJiti(import.meta.url, {
    interopDefault: true,
  });

  let mod: Record<string, unknown>;
  try {
    mod = (await jiti.import(inputPath)) as Record<string, unknown>;
  } catch (err) {
    consola.error('Failed to load schema source file:', err);
    process.exit(1);
  }

  const schema = mod[exportName];
  if (!schema || typeof schema !== 'object' || !('toJSONSchema' in schema)) {
    consola.error(
      `Export "${exportName}" not found or does not have a toJSONSchema() method.\n` +
        `Make sure the file exports a Zod schema named "${exportName}".`,
    );
    process.exit(1);
  }

  // Use input-side JSON Schema so defaulted fields stay optional for authors.
  // biome-ignore lint: Zod schema type is dynamic
  const jsonSchema = (schema as any).toJSONSchema({ io: 'input' });
  writeFileSync(outputPath, JSON.stringify(jsonSchema, null, 2) + '\n');

  consola.success(`JSON Schema written to ${outputPath}`);
}
