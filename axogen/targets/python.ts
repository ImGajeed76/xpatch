import { template, unsafe } from "@axonotes/axogen";
import { metadata } from "../metadata.ts";

export const pythonTarget = template({
    path: "crates/xpatch-python/pyproject.toml",
    template: "axogen/templates/pyproject.toml.njk",
    engine: "nunjucks",
    variables: {
        pythonPackageName: metadata.pythonPackageName,
        version: metadata.version,
        description: metadata.description,
        author: unsafe(metadata.author!, "Author name is not a secret"),
        license: metadata.license,
        homepage: metadata.homepage,
        repository: metadata.repository,
        keywords: metadata.keywords,
        pythonClassifiers: metadata.pythonClassifiers,
    },
    generate_meta: true
});
