/**
 * Project initialization command.
 *
 * Interactive wizard for creating a new Moyu project. Currently the
 * scaffolding logic is not yet implemented – only the prompt framework
 * is wired up. The actual file generation will be added later.
 */

import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { defineCommand } from 'citty';
import consola from 'consola';
import pc from 'picocolors';

export default defineCommand({
  meta: {
    name: 'init',
    description: 'Create a new Moyu project',
  },
  args: {
    name: {
      type: 'positional',
      description: 'Project name',
      required: false,
    },
  },
  run: async ({ args }) => {
    consola.log('');
    consola.log(pc.bold('Moyu Project Setup'));
    consola.log(pc.dim('Create a new visual novel project\n'));

    // Step 1: Project name
    let projectName = args.name;
    if (!projectName) {
      projectName = (await consola.prompt('Project name:', {
        type: 'text',
        placeholder: 'my-visual-novel',
        default: 'my-visual-novel',
        cancel: 'symbol',
      })) as string;

      if (typeof projectName === 'symbol') {
        // User cancelled (Ctrl+C)
        consola.info('Setup cancelled.');
        return;
      }
    }

    // Validate project name
    if (!/^[a-z0-9][a-z0-9._-]*$/.test(projectName)) {
      consola.error('Invalid project name. Use lowercase letters, numbers, hyphens, and dots.');
      process.exit(1);
    }

    const targetDir = resolve(process.cwd(), projectName);
    if (existsSync(targetDir)) {
      consola.error(`Directory "${projectName}" already exists.`);
      process.exit(1);
    }

    // Step 2: Template selection
    const template = (await consola.prompt('Select a template:', {
      type: 'select',
      cancel: 'symbol',
      options: [
        { label: 'Basic', value: 'basic', hint: 'Minimal starter template' },
        { label: 'Advanced', value: 'advanced', hint: 'With save/load, settings, and gallery' },
      ],
    })) as string;

    if (typeof template === 'symbol') {
      consola.info('Setup cancelled.');
      return;
    }

    // Step 3: Confirm
    consola.log('');
    consola.log(pc.bold('Project configuration:'));
    consola.log(`  Name:     ${pc.cyan(projectName)}`);
    consola.log(`  Template: ${pc.cyan(template)}`);
    consola.log(`  Path:     ${pc.dim(targetDir)}`);
    consola.log('');

    const confirmed = await consola.prompt('Create project?', {
      type: 'confirm',
      initial: true,
      cancel: 'symbol',
    });

    if (!confirmed || typeof confirmed === 'symbol') {
      consola.info('Setup cancelled.');
      return;
    }

    // Placeholder for actual scaffolding
    await createProject({ name: projectName, template, targetDir });
  },
});

// ---------------------------------------------------------------------------
// Project scaffolding (to be implemented)
// ---------------------------------------------------------------------------

interface ProjectOptions {
  name: string;
  template: string;
  targetDir: string;
}

// eslint-disable-next-line @typescript-eslint/no-unused-vars
async function createProject(_options: ProjectOptions): Promise<void> {
  consola.log('');
  consola.warn('Project scaffolding is not yet implemented.');
  consola.info('This feature will be available in a future release.');
}
