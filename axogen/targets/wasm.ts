import { template, unsafe } from "@axonotes/axogen";
import { metadata } from "../metadata.ts";

export const wasmTarget = template({
    path: "crates/xpatch-wasm/package.json",
    template: "axogen/templates/wasm-package.json.njk",
    engine: "nunjucks",
    variables: {
        wasmPackageName: "xpatch-wasm",
        version: metadata.version,
        description: "WebAssembly bindings for xpatch - high-performance delta compression library",
        author: unsafe(metadata.author!, "Author name is not a secret"),
        license: metadata.license,
        homepage: metadata.homepage,
        repository: metadata.repository,
        keywords: [...metadata.keywords, "wasm", "webassembly"],
    }
});
