import { cmd, group, liveExec } from "@axonotes/axogen";
import * as z from "zod";
import { detectTool } from "../utils/tool-detection.ts";
import { logger } from "../console/logger.ts";
import { getPreference, setPreference } from "../preferences.ts";
import { askYesNo } from "../utils/prompts.ts";

async function buildRust(release: boolean, cli: boolean): Promise<void> {
    const cargo = await detectTool("Cargo", "cargo");
    if (!cargo.installed) {
        logger.error("Cargo not found");
        logger.info("Install from: https://rustup.rs/");
        process.exit(1);
    }

    logger.start("Building Rust library");

    const flags = [];
    if (release) {
        flags.push("--release");
    }
    if (cli) {
        flags.push("-p xpatch --features cli");
    } else {
        flags.push("--all");
    }

    await liveExec(`cargo build ${flags.join(" ")}`);
    logger.success("Rust build complete");
}

async function buildPython(release: boolean): Promise<void> {
    const maturin = await detectTool("Maturin", "maturin");
    if (!maturin.installed) {
        logger.error("Maturin not found");
        logger.info("Install with: pip install maturin");
        process.exit(1);
    }

    logger.start("Building Python bindings");

    const flags = release ? "--release" : "";
    await liveExec(`cd crates/xpatch-python && maturin build ${flags}`);

    logger.success("Python build complete");
    logger.info("Wheel created in crates/xpatch-python/target/wheels/");
}

async function buildNode(release: boolean): Promise<void> {
    const bun = await detectTool("Bun", "bun");
    const npm = await detectTool("npm", "npm");

    let pm = await getPreference("nodePackageManager");

    if (!pm) {
        if (bun.installed && npm.installed) {
            const useBun = await askYesNo("Use Bun for Node.js builds?", true);
            pm = useBun ? "bun" : "npm";
            await setPreference("nodePackageManager", pm);
        } else if (bun.installed) {
            pm = "bun";
            await setPreference("nodePackageManager", pm);
        } else if (npm.installed) {
            pm = "npm";
            await setPreference("nodePackageManager", pm);
        } else {
            logger.error("Neither Bun nor npm found");
            logger.info("Install Bun: https://bun.sh/");
            logger.info("Or install Node.js: https://nodejs.org/");
            process.exit(1);
        }
    }

    const tool = pm === "bun" ? bun : npm;
    if (!tool.installed) {
        logger.error(`${pm} not found`);
        process.exit(1);
    }

    logger.start(`Building Node.js bindings with ${pm}`);

    await liveExec(`cd crates/xpatch-node && ${pm} install`);

    const buildCmd = release ? "build" : "build:debug";
    await liveExec(`cd crates/xpatch-node && ${pm} run ${buildCmd}`);

    logger.success("Node.js build complete");
}

export const buildCommands = group({
    help: "Build commands for all components",
    commands: {
        rust: cmd({
            help: "Build the core Rust library",
            options: {
                release: z.boolean().default(false).describe("Build in release mode"),
                cli: z.boolean().default(false).describe("Build CLI with features"),
            },
            exec: async (ctx) => {
                await buildRust(ctx.options.release, ctx.options.cli);
            },
        }),

        python: cmd({
            help: "Build Python bindings using maturin",
            options: {
                release: z.boolean().default(false).describe("Build in release mode"),
            },
            exec: async (ctx) => {
                await buildPython(ctx.options.release);
            },
        }),

        node: cmd({
            help: "Build Node.js bindings using napi-rs",
            options: {
                release: z.boolean().default(true).describe("Build in release mode"),
            },
            exec: async (ctx) => {
                await buildNode(ctx.options.release);
            },
        }),

        all: cmd({
            help: "Build all components (Rust, Python, Node.js)",
            options: {
                release: z.boolean().default(false).describe("Build in release mode"),
            },
            exec: async (ctx) => {
                logger.header("Building All Components");

                logger.divider("Rust");
                await buildRust(ctx.options.release, false);

                console.log();
                logger.divider("Python");
                await buildPython(ctx.options.release);

                console.log();
                logger.divider("Node.js");
                await buildNode(ctx.options.release);

                console.log();
                logger.success("All components built successfully");
            },
        }),
    },
});
