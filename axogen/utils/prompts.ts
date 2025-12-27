import { createInterface } from "readline";

export function askYesNo(question: string, defaultYes = true): Promise<boolean> {
    return new Promise((resolve) => {
        const rl = createInterface({
            input: process.stdin,
            output: process.stdout,
        });

        const suffix = defaultYes ? " (Y/n)" : " (y/N)";
        rl.question(`${question}${suffix}: `, (answer) => {
            rl.close();
            const normalized = answer.trim().toLowerCase();

            if (normalized === "") {
                resolve(defaultYes);
            } else {
                resolve(normalized === "y" || normalized === "yes");
            }
        });
    });
}

export function askChoice<T extends string>(
    question: string,
    choices: readonly T[],
    defaultChoice?: T
): Promise<T> {
    return new Promise((resolve) => {
        const rl = createInterface({
            input: process.stdin,
            output: process.stdout,
        });

        const choicesList = choices.map((c, i) => `${i + 1}) ${c}`).join(" ");
        const defaultSuffix = defaultChoice ? ` [default: ${defaultChoice}]` : "";

        rl.question(`${question} (${choicesList})${defaultSuffix}: `, (answer) => {
            rl.close();
            const normalized = answer.trim();

            if (normalized === "" && defaultChoice) {
                resolve(defaultChoice);
                return;
            }

            const index = parseInt(normalized, 10) - 1;
            if (index >= 0 && index < choices.length) {
                resolve(choices[index]);
            } else {
                const found = choices.find(c => c.toLowerCase() === normalized.toLowerCase());
                if (found) {
                    resolve(found);
                } else if (defaultChoice) {
                    resolve(defaultChoice);
                } else {
                    resolve(choices[0]);
                }
            }
        });
    });
}
