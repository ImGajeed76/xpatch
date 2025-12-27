import { template, unsafe } from "@axonotes/axogen";
import { metadata } from "../metadata.ts";

export const docsTarget = template({
    path: "DEVELOPMENT.md",
    template: "axogen/templates/DEVELOPMENT.md.njk",
    engine: "nunjucks",
    variables: {
        version: metadata.version,
        repository: metadata.repository,
        commands: {
            setup: unsafe("axogen run setup", "Command string"),
            howto: unsafe("axogen run howto", "Command string"),
            buildAll: unsafe("axogen run build all", "Command string"),
            buildRust: unsafe("axogen run build rust", "Command string"),
            buildPython: unsafe("axogen run build python", "Command string"),
            buildNode: unsafe("axogen run build node", "Command string"),
            test: unsafe("axogen run test", "Command string"),
            testRust: unsafe("axogen run test rust", "Command string"),
            testPython: unsafe("axogen run test python", "Command string"),
            testNode: unsafe("axogen run test node", "Command string"),
            exampleList: unsafe("axogen run example list", "Command string"),
            exampleBasic: unsafe("axogen run example basic", "Command string"),
            exampleTags: unsafe("axogen run example tags", "Command string"),
            fmt: unsafe("axogen run fmt", "Command string"),
            lint: unsafe("axogen run lint", "Command string"),
            clean: unsafe("axogen run clean", "Command string"),
        },
    },
});
