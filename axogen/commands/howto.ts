import { cmd, group } from "@axonotes/axogen";
import { metadata } from "../metadata.ts";
import { logger } from "../console/logger.ts";

const howtos = {
    overview: () => {
        logger.header(`xpatch Development Reference v${metadata.version}`);
        console.log();
        logger.info("Common Commands:");
        logger.bullet("axogen run setup              Set up development environment", 1);
        logger.bullet("axogen run build [target]     Build components (rust|python|node|all)", 1);
        logger.bullet("axogen run test [target]      Run tests", 1);
        logger.bullet("axogen run example <name>     Run an example", 1);
        logger.bullet("axogen run clean              Clean build artifacts", 1);
        console.log();
        logger.info("For detailed help:");
        logger.bullet("axogen run howto build", 1);
        logger.bullet("axogen run howto test", 1);
        logger.bullet("axogen run howto release", 1);
        logger.bullet("axogen run howto example", 1);
        logger.bullet("axogen run howto bench", 1);
        logger.bullet("axogen run howto local", 1);
        console.log();
    },

    build: () => {
        logger.header("Building xpatch");
        console.log();
        logger.info("Core Rust Library:");
        logger.bullet("axogen run build rust              Debug build", 1);
        logger.bullet("axogen run build rust --release    Release build", 1);
        logger.bullet("axogen run build rust --cli        With CLI features", 1);
        console.log();
        logger.info("Python Bindings:");
        logger.bullet("axogen run build python            Uses maturin develop", 1);
        logger.bullet("cd crates/xpatch-python && maturin develop", 1);
        console.log();
        logger.info("Node.js Bindings:");
        logger.bullet("axogen run build node              Uses bun or npm", 1);
        logger.bullet("cd crates/xpatch-node && bun install && bun run build", 1);
        console.log();
        logger.info("Build Everything:");
        logger.bullet("axogen run build all               Builds in correct order", 1);
        console.log();
    },

    test: () => {
        logger.header("Testing xpatch");
        console.log();
        logger.info("Run All Tests:");
        logger.bullet("axogen run test                    All tests across all languages", 1);
        console.log();
        logger.info("Individual Components:");
        logger.bullet("axogen run test rust               Rust unit tests", 1);
        logger.bullet("axogen run test python             Python binding tests", 1);
        logger.bullet("axogen run test node               Node.js binding tests", 1);
        console.log();
        logger.info("Manual Testing:");
        logger.bullet("cargo test -p xpatch               Direct cargo test", 1);
        logger.bullet("cd crates/xpatch-python && python3 tests/test_xpatch.py", 1);
        logger.bullet("cd crates/xpatch-node && bun test.js", 1);
        console.log();
    },

    release: () => {
        logger.header("Release Process for xpatch");
        console.log();
        logger.info("1. Update Version:");
        logger.bullet("Edit workspace Cargo.toml: version = \"x.y.z\"", 1);
        logger.bullet("Run: axogen generate", 1);
        logger.bullet("This updates pyproject.toml and package.json automatically", 1);
        console.log();
        logger.info("2. Update Changelog:");
        logger.bullet("Edit CHANGELOG.md with changes since last release", 1);
        logger.bullet("Follow Keep a Changelog format", 1);
        console.log();
        logger.info("3. Commit Changes:");
        logger.bullet("git add .", 1);
        logger.bullet("git commit -m \"Release vx.y.z\"", 1);
        console.log();
        logger.info("4. Tag Release:");
        logger.bullet("git tag -a vx.y.z -m \"Release vx.y.z\"", 1);
        logger.bullet("git push origin master --tags", 1);
        console.log();
        logger.info("5. Publish Packages:");
        logger.bullet("Rust: cargo publish -p xpatch", 1);
        logger.bullet("Python: cd crates/xpatch-python && maturin publish", 1);
        logger.bullet("Node: cd crates/xpatch-node && npm publish", 1);
        console.log();
        logger.info("6. Create GitHub Release:");
        logger.bullet(`Go to ${metadata.repository}/releases/new`, 1);
        logger.bullet("Select the tag", 1);
        logger.bullet("Copy changelog entries", 1);
        logger.bullet("Publish release", 1);
        console.log();
    },

    example: () => {
        logger.header("Running Examples");
        console.log();
        logger.info("List Examples:");
        logger.bullet("axogen run example list", 1);
        console.log();
        logger.info("Run Specific Example:");
        logger.bullet("axogen run example basic           Basic usage", 1);
        logger.bullet("axogen run example tags            Tag optimization demo", 1);
        console.log();
        logger.info("Manual:");
        logger.bullet("cargo run --example basic", 1);
        logger.bullet("cargo run --example tags", 1);
        console.log();
    },

    bench: () => {
        logger.header("Running Benchmarks");
        console.log();
        logger.info("Available Benchmarks:");
        logger.bullet("cargo bench --bench stress         Stress test patterns", 1);
        logger.bullet("cargo bench --bench git_real_world Real git histories", 1);
        console.log();
        logger.warn("Recommended: Build cache first, then run benchmarks");
        logger.bullet("Step 1: Build cache from git repository", 1);
        logger.bullet("  XPATCH_PRESET=tokio XPATCH_BUILD_CACHE=true cargo bench --bench git_real_world", 2);
        logger.bullet("Step 2: Run benchmark using cached data", 1);
        logger.bullet("  XPATCH_PRESET=tokio XPATCH_USE_CACHE=true cargo bench --bench git_real_world", 2);
        console.log();
        logger.info("Git Real-World Benchmark Configuration:");
        logger.bullet("Required (one of):", 1);
        logger.bullet("  XPATCH_REPO=<url>     Repository URL", 2);
        logger.bullet("  XPATCH_PRESET=<name>  Use preset (rust, neovim, tokio, git)", 2);
        console.log();
        logger.bullet("Options:", 1);
        logger.bullet("  XPATCH_MAX_COMMITS=<n>     Max commits per file (default: 50, 0=all)", 2);
        logger.bullet("  XPATCH_MAX_TAG_DEPTH=<n>   Tag search depth (default: 16)", 2);
        logger.bullet("  XPATCH_OUTPUT=<path>       Output directory (default: ./benchmark_results)", 2);
        logger.bullet("  XPATCH_CACHE_DIR=<path>    Cache directory", 2);
        logger.bullet("  XPATCH_BUILD_CACHE=<bool>  Build cache only (default: false)", 2);
        logger.bullet("  XPATCH_USE_CACHE=<bool>    Use existing cache (default: false)", 2);
        logger.bullet("  XPATCH_ALL_FILES=<bool>    Test all files (SLOW, default: false)", 2);
        logger.bullet("  XPATCH_ALL_FILES_HEAD=<bool> All files at HEAD (default: false)", 2);
        logger.bullet("  XPATCH_MAX_FILES=<n>       Max files to test (default: 0=all)", 2);
        logger.bullet("  XPATCH_PARALLEL_FILES=<bool> Process files in parallel (default: false)", 2);
        logger.bullet("  XPATCH_MIN_FILE_SIZE=<n>   Min average file size bytes (default: 100)", 2);
        console.log();
        logger.info("More Examples:");
        logger.bullet("XPATCH_PRESET=tokio XPATCH_MAX_COMMITS=1000 XPATCH_MAX_TAG_DEPTH=32 \\", 1);
        logger.bullet("  cargo bench --bench git_real_world", 1);
        console.log();
        logger.info("Results Location:");
        logger.bullet("crates/xpatch/benchmark_results/  Timestamped JSON and Markdown reports", 1);
        logger.bullet("target/criterion/                 Criterion HTML reports", 1);
        console.log();
    },

    local: () => {
        logger.header("Testing Packages Locally");
        console.log();
        logger.info("Use the local commands to prepare packages for local testing:");
        console.log();
        logger.bullet("axogen run local rust     Prepare Rust library", 1);
        logger.bullet("axogen run local python   Prepare Python package", 1);
        logger.bullet("axogen run local node     Prepare Node.js package", 1);
        console.log();
        logger.info("These commands will:");
        logger.bullet("Build the package", 1);
        logger.bullet("Set up local linking (for Python/Node)", 1);
        logger.bullet("Show exact commands to run in your test project", 1);
        console.log();
        logger.info("For detailed documentation, see:");
        logger.bullet("DEVELOPMENT.md - Testing Packages Locally section", 1);
        console.log();
    },
};

export const howtoCommands = group({
    help: "Quick reference documentation",
    commands: {
        build: cmd({
            help: "How to build xpatch",
            exec: () => howtos.build(),
        }),

        test: cmd({
            help: "How to test xpatch",
            exec: () => howtos.test(),
        }),

        release: cmd({
            help: "How to release a new version",
            exec: () => howtos.release(),
        }),

        example: cmd({
            help: "How to run examples",
            exec: () => howtos.example(),
        }),

        bench: cmd({
            help: "How to run benchmarks",
            exec: () => howtos.bench(),
        }),

        local: cmd({
            help: "How to test packages locally",
            exec: () => howtos.local(),
        }),
    },
});

export const howtoCommand = cmd({
    help: "Show quick reference documentation",
    exec: () => howtos.overview(),
});
