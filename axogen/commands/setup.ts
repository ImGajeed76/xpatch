import { cmd, liveExec } from "@axonotes/axogen";
import { detectAllTools, printToolStatus } from "../utils/tool-detection.ts";
import { logger } from "../console/logger.ts";
import { setPreference } from "../preferences.ts";
import { askYesNo } from "../utils/prompts.ts";

export const setupCommand = cmd({
    help: "Set up the development environment",
    exec: async () => {
        logger.header("Development Environment Setup");

        logger.divider("Detecting Tools");
        const tools = await detectAllTools();

        printToolStatus(tools.cargo);
        printToolStatus(tools.rustc);
        printToolStatus(tools.python);
        printToolStatus(tools.bun);
        printToolStatus(tools.npm);
        printToolStatus(tools.maturin);

        console.log();

        const missingCritical = [];
        if (!tools.cargo.installed) missingCritical.push("Cargo");
        if (!tools.python.installed) missingCritical.push("Python");

        if (missingCritical.length > 0) {
            logger.error(`Missing critical tools: ${missingCritical.join(", ")}`);
            logger.info("Please install them before continuing");
            process.exit(1);
        }

        if (!tools.bun.installed && !tools.npm.installed) {
            logger.warn("No Node.js package manager found");
            logger.info("Install Bun: https://bun.sh/");
            logger.info("Or install Node.js: https://nodejs.org/");
        } else if (tools.bun.installed && tools.npm.installed) {
            const useBun = await askYesNo("Use Bun for Node.js?", true);
            await setPreference("nodePackageManager", useBun ? "bun" : "npm");
        } else if (tools.bun.installed) {
            await setPreference("nodePackageManager", "bun");
        } else {
            await setPreference("nodePackageManager", "npm");
        }

        if (!tools.maturin.installed) {
            logger.warn("Maturin not found");
            const install = await askYesNo("Install maturin now?", true);
            if (install) {
                logger.start("Installing maturin");
                await liveExec("pip install maturin");
            }
        }

        console.log();
        logger.divider("Installing Dependencies");

        logger.info("Fetching Rust dependencies");
        await liveExec("cargo fetch");

        const pm = tools.bun.installed ? "bun" : "npm";
        if (tools.bun.installed || tools.npm.installed) {
            logger.info(`Installing Node.js dependencies with ${pm}`);
            await liveExec(`cd crates/xpatch-node && ${pm} install`);
        }

        console.log();
        logger.divider("Building All Components");
        await liveExec("axogen run build all");

        console.log();
        logger.success("Setup complete");
        console.log();
        logger.info("Try the following commands:");
        logger.bullet("axogen run test         Run all tests", 1);
        logger.bullet("axogen run example basic", 1);
        logger.bullet("axogen run howto        Quick reference", 1);
    },
});
