import { template, unsafe } from "@axonotes/axogen";
import { metadata } from "../metadata.ts";

export const nodeTarget = template({
    path: "crates/xpatch-node/package.json",
    template: "axogen/templates/package.json.njk",
    engine: "nunjucks",
    variables: {
        nodePackageName: metadata.nodePackageName,
        version: metadata.version,
        description: metadata.description,
        author: unsafe(metadata.author!, "Author name is not a secret"),
        license: metadata.license,
        homepage: metadata.homepage,
        repository: metadata.repository,
        keywords: metadata.keywords,
    }
});
