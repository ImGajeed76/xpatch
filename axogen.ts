import { defineConfig } from "@axonotes/axogen";
import { pythonTarget, nodeTarget, docsTarget } from "./axogen/targets/index.ts";
import {
    buildCommands,
    testCommands,
    setupCommand,
    howtoCommand,
    howtoCommands,
    exampleCommand,
    cleanCommand,
    localCommands,
} from "./axogen/commands/index.ts";

export default defineConfig({
    targets: {
        python: pythonTarget,
        node: nodeTarget,
        docs: docsTarget,
    },

    commands: {
        build: buildCommands,
        test: testCommands,
        setup: setupCommand,
        howto: howtoCommands,
        example: exampleCommand,
        clean: cleanCommand,
        local: localCommands,

        fmt: "cargo fmt --all",
        lint: "cargo clippy --all --all-features -- -D warnings",
    },
});
