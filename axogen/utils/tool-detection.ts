import { exec } from "@axonotes/axogen";
import { logger } from "../console/logger.ts";

export interface ToolInfo {
    name: string;
    command: string;
    installed: boolean;
    version?: string;
    installUrl?: string;
}

export async function detectTool(
    name: string,
    command: string,
    versionFlag = "--version",
    installUrl?: string
): Promise<ToolInfo> {
    try {
        const result = await exec(`${command} ${versionFlag}`);
        return {
            name,
            command,
            installed: true,
            version: result.stdout.trim().split("\n")[0],
            installUrl,
        };
    } catch {
        return {
            name,
            command,
            installed: false,
            installUrl,
        };
    }
}

export async function detectAllTools() {
    const [cargo, rustc, python, bun, npm, maturin] = await Promise.all([
        detectTool("Cargo", "cargo", "--version", "https://rustup.rs/"),
        detectTool("Rust", "rustc", "--version", "https://rustup.rs/"),
        detectTool("Python", "python3", "--version", "https://www.python.org/downloads/"),
        detectTool("Bun", "bun", "--version", "https://bun.sh/"),
        detectTool("npm", "npm", "--version", "https://nodejs.org/"),
        detectTool("Maturin", "maturin", "--version", "pip install maturin"),
    ]);

    return { cargo, rustc, python, bun, npm, maturin };
}

export function printToolStatus(tool: ToolInfo): void {
    if (tool.installed) {
        logger.logF(`  <success>✓</success> ${tool.name}: <subtle>${tool.version}</subtle>`);
    } else {
        logger.logF(`  <danger>✗</danger> ${tool.name}: <subtle>Not installed</subtle>`);
        if (tool.installUrl) {
            logger.logF(`    <muted>Install:</muted> ${tool.installUrl}`);
        }
    }
}
