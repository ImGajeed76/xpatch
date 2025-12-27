import { cmd, liveExec } from "@axonotes/axogen";
import * as z from "zod";
import { logger } from "../console/logger.ts";
import { detectTool } from "../utils/tool-detection.ts";
import * as net from "net";

const RUST_EXAMPLES = ["basic", "tags"] as const;
const C_EXAMPLES = ["basic"] as const;
const WASM_EXAMPLES = ["browser", "node"] as const;

// Find an available port starting from the given port
async function findAvailablePort(startPort: number, maxAttempts: number = 10): Promise<number> {
    for (let port = startPort; port < startPort + maxAttempts; port++) {
        try {
            await new Promise<void>((resolve, reject) => {
                const server = net.createServer();
                server.once('error', (err: NodeJS.ErrnoException) => {
                    if (err.code === 'EADDRINUSE') {
                        reject(err);
                    } else {
                        reject(err);
                    }
                });
                server.once('listening', () => {
                    server.close(() => resolve());
                });
                server.listen(port);
            });
            return port;
        } catch (err) {
            // Port is in use, try next one
            continue;
        }
    }
    throw new Error(`No available ports found in range ${startPort}-${startPort + maxAttempts - 1}`);
}

export const exampleCommand = cmd({
    help: "Run examples",
    options: {
        lang: z.enum(["rust", "c", "wasm"]).default("rust").describe("Language (rust, c, or wasm)"),
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
            logger.divider("WASM Examples");
            logger.bullet("browser  Interactive browser demo with UI (builds & serves)", 1);
            logger.bullet("node     Node.js example with 5 test scenarios (builds & runs)", 1);
            console.log();
            logger.info("Run with: axogen run example <name> [--lang=rust|c|wasm]");
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
        } else if (lang === "wasm") {
            if (!WASM_EXAMPLES.includes(name as any)) {
                logger.error(`Unknown WASM example: ${name}`);
                logger.info("Run 'axogen run example list' to see available examples");
                process.exit(1);
            }

            const wasmPack = await detectTool("wasm-pack", "wasm-pack");
            if (!wasmPack.installed) {
                logger.error("wasm-pack not found");
                logger.info("Install with: cargo install wasm-pack");
                process.exit(1);
            }

            if (name === "browser") {
                // Build WASM for web target
                logger.start("Building WASM for web target");
                await liveExec("cd crates/xpatch-wasm && wasm-pack build --release --target web");

                // Check for Python
                const python = await detectTool("Python", "python3");
                if (!python.installed) {
                    logger.error("Python 3 not found");
                    logger.info("Install from: https://www.python.org/downloads/");
                    process.exit(1);
                }

                // Find available port
                logger.start("Finding available port");
                const port = await findAvailablePort(8080);

                if (port !== 8080) {
                    logger.info(`Port 8080 in use, using port ${port} instead`);
                }

                logger.success("Build complete!");
                console.log();
                logger.header("Starting Browser Demo");
                logger.info(`Server starting at: http://localhost:${port}/examples/browser/`);
                logger.info("Press Ctrl+C to stop the server");
                console.log();

                // Start HTTP server (this will block until Ctrl+C)
                await liveExec(`cd crates/xpatch-wasm && python3 -m http.server ${port}`);
            } else if (name === "node") {
                // Build WASM for Node.js target
                logger.start("Building WASM for Node.js target");
                await liveExec("cd crates/xpatch-wasm && wasm-pack build --release --target nodejs");

                const node = await detectTool("Node.js", "node");
                if (!node.installed) {
                    logger.error("Node.js not found");
                    logger.info("Install from: https://nodejs.org/");
                    process.exit(1);
                }

                // Run the example
                logger.start("Running Node.js example");
                console.log();
                await liveExec("cd crates/xpatch-wasm/examples/node && node example.js");
            }
        }
    },
});
