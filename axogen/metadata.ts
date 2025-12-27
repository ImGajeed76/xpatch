import { loadFile } from "@axonotes/axogen";
import * as z from "zod";

const workspaceCargoSchema = z.object({
    workspace: z.object({
        package: z.object({
            version: z.string(),
            edition: z.string(),
            license: z.string(),
            repository: z.string(),
            homepage: z.string(),
            authors: z.array(z.string()),
        }),
    }),
});

const workspaceCargo = loadFile(
    "Cargo.toml",
    "toml",
    workspaceCargoSchema
);

export const metadata = {
    version: workspaceCargo.workspace.package.version,
    edition: workspaceCargo.workspace.package.edition,
    license: workspaceCargo.workspace.package.license,
    repository: workspaceCargo.workspace.package.repository,
    homepage: workspaceCargo.workspace.package.homepage,
    author: workspaceCargo.workspace.package.authors[0],

    pythonPackageName: "xpatch-rs",
    nodePackageName: "xpatch-rs",
    description: "High-performance delta compression library with automatic algorithm selection",
    keywords: ["delta", "compression", "diff", "patch", "version-control"],

    pythonClassifiers: [
        "Programming Language :: Rust",
        "Programming Language :: Python :: Implementation :: CPython",
        "Programming Language :: Python :: Implementation :: PyPy",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Programming Language :: Python :: 3.13",
        "License :: OSI Approved :: GNU Affero General Public License v3 or later (AGPLv3+)",
        "Topic :: System :: Archiving :: Compression",
    ],
};
