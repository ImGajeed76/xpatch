import { cmd, liveExec } from "@axonotes/axogen";
import { logger } from "../console/logger.ts";
import { detectTool } from "../utils/tool-detection.ts";

export const cleanCommand = cmd({
    help: "Clean build artifacts",
    exec: async () => {
        logger.header("Cleaning Build Artifacts");

        logger.divider("Rust");
        const cargo = await detectTool("Cargo", "cargo");
        if (cargo.installed) {
            logger.info("Cleaning Rust target directory");
            await liveExec("cargo clean");
        } else {
            logger.warn("Cargo not found, skipping Rust clean");
        }

        console.log();
        logger.divider("Python");
        logger.info("Cleaning Python build artifacts");
        await liveExec("rm -rf crates/xpatch-python/target");
        await liveExec("rm -rf crates/xpatch-python/build");
        await liveExec("rm -rf crates/xpatch-python/*.egg-info");
        await liveExec("find crates/xpatch-python -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true");

        console.log();
        logger.divider("Node.js");
        logger.info("Cleaning Node.js build artifacts");
        await liveExec("rm -rf crates/xpatch-node/target");
        await liveExec("rm -rf crates/xpatch-node/*.node");
        await liveExec("rm -rf crates/xpatch-node/artifacts");

        console.log();
        logger.success("All build artifacts cleaned");
    },
});
