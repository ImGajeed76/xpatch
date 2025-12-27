import { cmd, group, liveExec } from "@axonotes/axogen";
import { logger } from "../console/logger.ts";
import { resolve } from "path";
import { getPreference, setPreference } from "../preferences.ts";
import { askYesNo } from "../utils/prompts.ts";

export const localCommands = group({
    help: "Prepare packages for local testing in other projects",
    commands: {
        c: cmd({
            help: "Prepare C/C++ bindings for local use",
            exec: async () => {
                const cPath = resolve(process.cwd(), "crates/xpatch-c/dist");

                logger.header("Preparing C/C++ Bindings for Local Use");
                console.log();

                logger.start("Building C/C++ bindings");
                await liveExec("cd crates/xpatch-c && cargo build --release");

                // Create dist directory
                await liveExec("mkdir -p crates/xpatch-c/dist");

                // Copy library
                const platform = process.platform;
                let libExt = "so";
                if (platform === "darwin") libExt = "dylib";
                else if (platform === "win32") libExt = "dll";

                await liveExec(`cp -f target/release/libxpatch_c.${libExt} crates/xpatch-c/dist/`);
                await liveExec(`cp -f crates/xpatch-c/xpatch.h crates/xpatch-c/dist/`);
                await liveExec(`cp -f crates/xpatch-c/README.md crates/xpatch-c/dist/`);

                logger.success("C/C++ bindings prepared");
                console.log();

                logger.header("Ready for Local Use");
                console.log();
                logger.info("Package contents:");
                console.log();
                logger.logF(`<primary>${cPath}/</primary>`);
                logger.logF(`<primary>  ├── libxpatch_c.${libExt}</primary>`);
                logger.logF(`<primary>  ├── xpatch.h</primary>`);
                logger.logF(`<primary>  └── README.md</primary>`);
                console.log();
                logger.info("To use in your C/C++ project:");
                console.log();
                logger.logF(`<primary>gcc -o myapp myapp.c -I${cPath} -L${cPath} -lxpatch_c</primary>`);
                console.log();
            },
        }),

        rust: cmd({
            help: "Prepare Rust library for local use",
            exec: async () => {
                const xpatchPath = resolve(process.cwd(), "crates/xpatch");

                logger.header("Preparing Rust Library for Local Use");
                console.log();

                logger.start("Building Rust library");
                await liveExec("cargo build -p xpatch");
                logger.success("Rust library built");
                console.log();

                logger.header("Ready for Local Use");
                console.log();
                logger.info("Add this to your test project's Cargo.toml:");
                console.log();
                logger.logF(`<primary>[dependencies]</primary>`);
                logger.logF(`<primary>xpatch = { path = "${xpatchPath}" }</primary>`);
                console.log();
                logger.info("Or run this command in your test project:");
                console.log();
                logger.logF(`<primary>cargo add --path ${xpatchPath}</primary>`);
                console.log();
            },
        }),

        python: cmd({
            help: "Prepare Python package for local use",
            exec: async () => {
                const pythonPath = resolve(process.cwd(), "crates/xpatch-python");

                logger.header("Preparing Python Package for Local Use");
                console.log();

                // Check if in virtualenv
                const inVenv = process.env.VIRTUAL_ENV || process.env.CONDA_PREFIX;

                if (inVenv) {
                    logger.start("Building and installing with maturin develop");
                    await liveExec("cd crates/xpatch-python && maturin develop");
                    logger.success("Python package installed in development mode");
                    console.log();

                    logger.header("Ready for Local Use");
                    console.log();
                    logger.info("The package is now available in this environment. Test it:");
                    console.log();
                    logger.logF(`<primary>python -c "import xpatch; print(xpatch.__version__)"</primary>`);
                    console.log();
                } else {
                    logger.start("Building Python wheel");
                    await liveExec("cd crates/xpatch-python && maturin build --release");
                    logger.success("Python wheel built");
                    console.log();

                    logger.header("Ready for Local Use");
                    console.log();
                    logger.info("Install the wheel in your project:");
                    console.log();
                    logger.logF(`<primary>pip install ${pythonPath}/target/wheels/xpatch_rs-*.whl</primary>`);
                    console.log();
                    logger.info("Or use editable install:");
                    console.log();
                    logger.logF(`<primary>pip install -e ${pythonPath}</primary>`);
                    console.log();
                }
            },
        }),

        node: cmd({
            help: "Prepare Node.js package for local use",
            exec: async () => {
                const nodePath = resolve(process.cwd(), "crates/xpatch-node");

                logger.header("Preparing Node.js Package for Local Use");
                console.log();

                let pm = await getPreference("nodePackageManager");

                if (!pm) {
                    const useBun = await askYesNo("Use Bun for building and linking?", true);
                    pm = useBun ? "bun" : "npm";
                    await setPreference("nodePackageManager", pm);
                }

                logger.start(`Building Node.js package with ${pm}`);
                await liveExec(`cd crates/xpatch-node && ${pm} run build`);
                logger.success("Node.js package built");
                console.log();

                logger.start(`Creating global link with ${pm} link`);
                await liveExec(`cd crates/xpatch-node && ${pm} link`);
                logger.success("Package linked globally");
                console.log();

                logger.header("Ready for Local Use");
                console.log();
                logger.info("Run this command in your test project:");
                console.log();
                logger.logF(`<primary>${pm} link xpatch-rs</primary>`);
                console.log();
                logger.info("To unlink later:");
                console.log();
                logger.logF(`<primary>${pm} unlink xpatch-rs</primary>`);
                console.log();
                logger.info("Alternative: Direct path install");
                console.log();
                logger.logF(`<primary>${pm} ${pm === "npm" ? "install" : "add"} ${nodePath}</primary>`);
                console.log();
            },
        }),
    },
});
