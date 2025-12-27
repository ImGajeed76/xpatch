import { cmd, liveExec } from "@axonotes/axogen";
import * as z from "zod";
import { logger } from "../console/logger.ts";
import { detectTool } from "../utils/tool-detection.ts";

const RUST_EXAMPLES = ["basic", "tags"] as const;
const C_EXAMPLES = ["basic"] as const;

export const exampleCommand = cmd({
    help: "Run examples",
    options: {
        lang: z.enum(["rust", "c"]).default("rust").describe("Language (rust or c)"),
    },
    args: {
        name: z.string().describe("Example name or 'list'"),
    },
    exec: async (ctx) => {
        const { name } = ctx.args;
        const { lang } = ctx.options;

        if (name === "list") {
            logger.header("Available Examples");
            console.log();
            logger.divider("Rust Examples");
            logger.bullet("basic  Basic delta encoding and decoding", 1);
            logger.bullet("tags   Tag optimization demonstration", 1);
            console.log();
            logger.divider("C Examples");
            logger.bullet("basic  Basic delta encoding and decoding in C", 1);
            console.log();
            logger.info("Run with: axogen run example <name> [--lang=rust|c]");
            return;
        }

        if (lang === "rust") {
            if (!RUST_EXAMPLES.includes(name as any)) {
                logger.error(`Unknown Rust example: ${name}`);
                logger.info("Run 'axogen run example list' to see available examples");
                process.exit(1);
            }

            const cargo = await detectTool("Cargo", "cargo");
            if (!cargo.installed) {
                logger.error("Cargo not found");
                logger.info("Install from: https://rustup.rs/");
                process.exit(1);
            }

            logger.start(`Running Rust example: ${name}`);
            await liveExec(`cargo run --example ${name}`);
        } else if (lang === "c") {
            if (!C_EXAMPLES.includes(name as any)) {
                logger.error(`Unknown C example: ${name}`);
                logger.info("Run 'axogen run example list' to see available examples");
                process.exit(1);
            }

            const gcc = await detectTool("GCC", "gcc");
            const clang = await detectTool("Clang", "clang");

            if (!gcc.installed && !clang.installed) {
                logger.error("Neither GCC nor Clang found");
                logger.info("Install a C compiler to run C examples");
                process.exit(1);
            }

            // Build the C bindings first
            logger.start("Building C bindings");
            await liveExec("cd crates/xpatch-c && cargo build --release");

            // Build the example
            logger.start(`Building C example: ${name}`);
            await liveExec(`cd crates/xpatch-c/examples && make clean && make ${name}`);

            // Run the example
            logger.start(`Running C example: ${name}`);
            await liveExec(`cd crates/xpatch-c/examples && ./${name}`);
        }
    },
});
