/**
 * UI JSON Schema generation command.
 *
 * Loads the user's `src/data/ui.ts` via jiti, finds the exported
 * `GameUiSchema` Zod schema, and writes the JSON Schema representation
 * to `ui.schema.json` in the project root.
 */

import { join } from 'node:path';
import { defineCommand } from 'citty';
import { generateJsonSchema } from '../utils/generate-json-schema.js';
import { requireProjectRoot } from '../utils/project.js';

const UI_RELATIVE_PATH = 'src/data/ui.ts';
const OUTPUT_FILENAME = 'ui.schema.json';
const EXPORT_NAME = 'GameUiSchema';

export default defineCommand({
  meta: {
    name: 'ui-schema',
    description: 'Generate JSON Schema from Zod UI definitions',
  },
  args: {
    input: {
      type: 'string',
      description: `Path to the UI schema file (default: ${UI_RELATIVE_PATH})`,
    },
    output: {
      type: 'string',
      description: `Output filename (default: ${OUTPUT_FILENAME})`,
    },
  },
  run: async ({ args }) => {
    const projectRoot = requireProjectRoot();
    const inputPath = join(projectRoot, args.input ?? UI_RELATIVE_PATH);
    const outputPath = join(projectRoot, args.output ?? OUTPUT_FILENAME);

    await generateJsonSchema({ inputPath, outputPath, exportName: EXPORT_NAME });
  },
});
