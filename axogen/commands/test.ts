import { cmd, group, liveExec } from "@axonotes/axogen";
import { detectTool } from "../utils/tool-detection.ts";
import { logger } from "../console/logger.ts";
import { getPreference } from "../preferences.ts";

async function testRust(): Promise<void> {
    const cargo = await detectTool("Cargo", "cargo");
    if (!cargo.installed) {
        logger.error("Cargo not found");
        logger.info("Install from: https://rustup.rs/");
        process.exit(1);
    }

    logger.start("Running Rust tests");
    await liveExec("cargo test -p xpatch");
    logger.success("Rust tests passed");
}

async function testPython(): Promise<void> {
    const python = await detectTool("Python", "python3");
    if (!python.installed) {
        logger.error("Python not found");
        logger.info("Install from: https://www.python.org/downloads/");
        process.exit(1);
    }

    const maturin = await detectTool("Maturin", "maturin");
    if (!maturin.installed) {
        logger.error("Maturin not found");
        logger.info("Install with: pip install maturin");
        process.exit(1);
    }

    // Create temporary venv for testing (use .venv so maturin auto-detects it)
    const venvPath = ".venv";

    logger.start("Setting up test environment");
    await liveExec(`cd crates/xpatch-python && python3 -m venv ${venvPath}`);

    logger.start("Installing package in test environment");
    await liveExec(`cd crates/xpatch-python && ${venvPath}/bin/pip install -q maturin`);
    await liveExec(`cd crates/xpatch-python && ${venvPath}/bin/maturin develop`);

    logger.start("Running Python tests");
    await liveExec(`cd crates/xpatch-python && ${venvPath}/bin/python tests/test_xpatch.py`);

    // Cleanup
    await liveExec(`rm -rf crates/xpatch-python/${venvPath}`);

    logger.success("Python tests passed");
}

async function testNode(): Promise<void> {
    const pm = await getPreference("nodePackageManager");
    const runtime = pm === "bun" ? "bun" : "node";

    const tool = await detectTool(runtime, runtime);
    if (!tool.installed) {
        logger.error(`${runtime} not found`);
        process.exit(1);
    }

    // Build the package first (needed if clean was run or first time)
    logger.start(`Building Node.js package with ${pm || runtime}`);
    await liveExec(`cd crates/xpatch-node && ${pm || runtime} install`);
    await liveExec(`cd crates/xpatch-node && ${pm || runtime} run build:debug`);

    logger.start(`Running Node.js tests with ${runtime}`);
    await liveExec(`cd crates/xpatch-node && ${runtime} test.js`);
    logger.success("Node.js tests passed");
}

async function testC(): Promise<void> {
    const cargo = await detectTool("Cargo", "cargo");
    if (!cargo.installed) {
        logger.error("Cargo not found");
        logger.info("Install from: https://rustup.rs/");
        process.exit(1);
    }

    logger.start("Running C bindings tests");
    await liveExec("cargo test -p xpatch-c");
    logger.success("C bindings tests passed");
}

export const testCommands = group({
    help: "Test commands for all components",
    commands: {
        rust: cmd({
            help: "Run Rust tests",
            exec: async () => {
                await testRust();
            },
        }),

        python: cmd({
            help: "Run Python tests",
            exec: async () => {
                await testPython();
            },
        }),

        node: cmd({
            help: "Run Node.js tests",
            exec: async () => {
                await testNode();
            },
        }),

        c: cmd({
            help: "Run C bindings tests",
            exec: async () => {
                await testC();
            },
        }),

        all: cmd({
            help: "Run all tests",
            exec: async () => {
                logger.header("Running All Tests");

                logger.divider("Rust");
                await testRust();

                console.log();
                logger.divider("C Bindings");
                await testC();

                console.log();
                logger.divider("Python");
                await testPython();

                console.log();
                logger.divider("Node.js");
                await testNode();

                console.log();
                logger.success("All tests passed");
            },
        }),
    },
});
