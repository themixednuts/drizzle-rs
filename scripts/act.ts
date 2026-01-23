#!/usr/bin/env bun
/**
 * Run GitHub Actions locally using act
 *
 * Usage:
 *   bun scripts/act.ts              # Run CI workflow
 *   bun scripts/act.ts ci           # Run CI workflow
 *   bun scripts/act.ts ci lint      # Run only lint job
 *   bun scripts/act.ts release      # Test release workflow
 *   bun scripts/act.ts --list       # List all jobs
 *   bun scripts/act.ts --dry-run    # Dry run
 */

import { $ } from "bun";
import { existsSync } from "fs";
import { parseArgs } from "util";

const WORKFLOWS = [
  "ci",
  "release",
  "publish",
  "benchmarks",
  "turso-experimental",
  "update",
] as const;

type Workflow = (typeof WORKFLOWS)[number];

interface Args {
  workflow: Workflow;
  job?: string;
  list: boolean;
  dryRun: boolean;
  verbose: boolean;
  help: boolean;
}

function parseArguments(): Args {
  const { values, positionals } = parseArgs({
    args: Bun.argv.slice(2),
    options: {
      list: { type: "boolean", short: "l", default: false },
      "dry-run": { type: "boolean", short: "n", default: false },
      verbose: { type: "boolean", short: "v", default: false },
      help: { type: "boolean", short: "h", default: false },
    },
    allowPositionals: true,
  });

  let workflow: Workflow = "ci";
  let job: string | undefined;

  for (const arg of positionals) {
    if (WORKFLOWS.includes(arg as Workflow)) {
      workflow = arg as Workflow;
    } else {
      job = arg;
    }
  }

  return {
    workflow,
    job,
    list: values.list ?? false,
    dryRun: values["dry-run"] ?? false,
    verbose: values.verbose ?? false,
    help: values.help ?? false,
  };
}

function printHelp(): void {
  console.log(`
Usage: bun scripts/act.ts [OPTIONS] [WORKFLOW] [JOB]

Run GitHub Actions locally using act.

Workflows:
  ci                    CI workflow (default)
  release               Release workflow
  publish               Publish workflow
  benchmarks            Benchmarks workflow
  turso-experimental    Turso experimental workflow
  update                Update workflow

Options:
  -l, --list      List available jobs without running
  -n, --dry-run   Show what would run without executing
  -v, --verbose   Enable verbose output
  -h, --help      Show this help message

Examples:
  bun scripts/act.ts                  # Run CI workflow
  bun scripts/act.ts ci lint          # Run only lint job from CI
  bun scripts/act.ts release          # Test release workflow
  bun scripts/act.ts --list           # List all jobs
  bun scripts/act.ts -n ci            # Dry run CI workflow
`);
}

async function checkActInstalled(): Promise<boolean> {
  try {
    await $`act --version`.quiet();
    return true;
  } catch {
    return false;
  }
}

async function checkDockerRunning(): Promise<boolean> {
  try {
    await $`docker info`.quiet();
    return true;
  } catch {
    return false;
  }
}

function getEventForWorkflow(workflow: Workflow): string {
  switch (workflow) {
    case "ci":
    case "release":
    case "publish":
    case "turso-experimental":
      return "push";
    case "benchmarks":
    case "update":
      return "workflow_dispatch";
    default:
      return "push";
  }
}

async function main(): Promise<void> {
  const args = parseArguments();

  if (args.help) {
    printHelp();
    process.exit(0);
  }

  // Check prerequisites
  if (!(await checkActInstalled())) {
    console.error("\x1b[31mError: act is not installed.\x1b[0m\n");
    console.log("\x1b[33mInstall act using one of these methods:\x1b[0m");
    console.log("  Windows:     winget install nektos.act");
    console.log("  macOS/Linux: brew install act");
    console.log("  Manual:      https://github.com/nektos/act/releases\n");
    process.exit(1);
  }

  if (!(await checkDockerRunning())) {
    console.error("\x1b[31mError: Docker is not running.\x1b[0m");
    console.log("Please start Docker Desktop and try again.");
    process.exit(1);
  }

  // Check workflow file exists
  const workflowFile = `.github/workflows/${args.workflow}.yml`;
  if (!existsSync(workflowFile)) {
    console.error(
      `\x1b[31mError: Workflow file not found: ${workflowFile}\x1b[0m`
    );
    process.exit(1);
  }

  // Build act command arguments
  const actArgs: string[] = ["-W", workflowFile, getEventForWorkflow(args.workflow)];

  // Windows needs a specific Docker socket path
  if (process.platform === "win32") {
    actArgs.push("--container-daemon-socket", "//./pipe/dockerDesktopLinuxEngine");
  }

  if (args.job) {
    actArgs.push("-j", args.job);
  }

  if (existsSync(".env")) {
    actArgs.push("--secret-file", ".env");
  }

  if (args.list) {
    actArgs.push("-l");
  }

  if (args.dryRun) {
    actArgs.push("-n");
  }

  if (args.verbose) {
    actArgs.push("-v");
  }

  // For release workflow, simulate a tag
  if (args.workflow === "release") {
    actArgs.push("--env", "GITHUB_REF_NAME=v0.0.0-test");
  }

  console.log(`\x1b[36mRunning: act ${actArgs.join(" ")}\x1b[0m\n`);

  // Run act
  const proc = Bun.spawn(["act", ...actArgs], {
    stdio: ["inherit", "inherit", "inherit"],
    cwd: process.cwd(),
  });

  const exitCode = await proc.exited;
  process.exit(exitCode);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
