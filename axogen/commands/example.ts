import { cmd, liveExec } from "@axonotes/axogen";
import * as z from "zod";
import { logger } from "../console/logger.ts";
import { detectTool } from "../utils/tool-detection.ts";

const EXAMPLES = ["basic", "tags"] as const;

export const exampleCommand = cmd({
    help: "Run examples",
    args: {
        name: z.enum([...EXAMPLES, "list"]).describe("Example name or 'list'"),
    },
    exec: async (ctx) => {
        const { name } = ctx.args;

        if (name === "list") {
            logger.header("Available Examples");
            console.log();
            logger.bullet("basic  Basic delta encoding and decoding", 1);
            logger.bullet("tags   Tag optimization demonstration", 1);
            console.log();
            logger.info("Run with: axogen run example <name>");
            return;
        }

        const cargo = await detectTool("Cargo", "cargo");
        if (!cargo.installed) {
            logger.error("Cargo not found");
            logger.info("Install from: https://rustup.rs/");
            process.exit(1);
        }

        logger.start(`Running example: ${name}`);
        await liveExec(`cargo run --example ${name}`);
    },
});
