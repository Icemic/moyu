/**
 * JSON Schema generation command.
 *
 * Loads the user's `src/commands/commands.ts` via jiti, finds the
 * exported `ScenarioCommandSchema` Zod schema, and writes the JSON
 * Schema representation to `commands.schema.json` in the project root.
 */

import { existsSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { defineCommand } from 'citty';
import consola from 'consola';
import { createJiti } from 'jiti';
import { requireProjectRoot } from '../utils/project.js';

const COMMANDS_RELATIVE_PATH = 'src/commands/commands.ts';
const OUTPUT_FILENAME = 'commands.schema.json';
const EXPORT_NAME = 'ScenarioCommandSchema';

export default defineCommand({
  meta: {
    name: 'schema',
    description: 'Generate JSON Schema from Zod command definitions',
  },
  args: {
    input: {
      type: 'string',
      description: `Path to the commands file (default: ${COMMANDS_RELATIVE_PATH})`,
    },
    output: {
      type: 'string',
      description: `Output filename (default: ${OUTPUT_FILENAME})`,
    },
  },
  run: async ({ args }) => {
    const projectRoot = requireProjectRoot();
    const inputPath = join(projectRoot, args.input ?? COMMANDS_RELATIVE_PATH);
    const outputPath = join(projectRoot, args.output ?? OUTPUT_FILENAME);

    if (!existsSync(inputPath)) {
      consola.error(
        `Commands file not found: ${inputPath}\n` +
          'Make sure your project has a commands definition file at the expected path.',
      );
      process.exit(1);
    }

    consola.start(`Loading ${inputPath}...`);

    // Use jiti to import TypeScript files at runtime
    const jiti = createJiti(import.meta.url, {
      // Enable TypeScript transformation
      interopDefault: true,
    });

    let mod: Record<string, unknown>;
    try {
      mod = (await jiti.import(inputPath)) as Record<string, unknown>;
    } catch (err) {
      consola.error('Failed to load commands file:', err);
      process.exit(1);
    }

    const schema = mod[EXPORT_NAME];
    if (!schema || typeof schema !== 'object' || !('toJSONSchema' in schema)) {
      consola.error(
        `Export "${EXPORT_NAME}" not found or does not have a toJSONSchema() method.\n` +
          `Make sure your commands file exports a Zod schema named "${EXPORT_NAME}".`,
      );
      process.exit(1);
    }

    // biome-ignore lint: Zod schema type is dynamic
    const jsonSchema = (schema as any).toJSONSchema();
    writeFileSync(outputPath, JSON.stringify(jsonSchema, null, 2) + '\n');

    consola.success(`Schema written to ${outputPath}`);
  },
});
