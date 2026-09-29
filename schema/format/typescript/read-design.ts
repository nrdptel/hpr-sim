// Reads each design document named on the command line with the generated types, and prints a
// line for each: what it holds, or why the reader refused it. Exits 1 if it refused any.
//
//     node schema/format/typescript/read-design.ts design.hpr [more.hpr ...]
//
// Node.js 22.18 or later runs TypeScript as it is; an older one needs --experimental-strip-types.
import { readFileSync } from "node:fs";
import { DesignFormatError, readDesign, type Component } from "./hpr-design.ts";

/** How many parts `components` hold, counting the parts inside parts. */
function count(components: Component[]): number {
  return components.reduce((sum, c) => sum + 1 + count(c.children ?? []), 0);
}

let refused = 0;
for (const path of process.argv.slice(2)) {
  try {
    const design = readDesign(readFileSync(path, "utf8"));
    const stages = design.rocket.stages;
    const parts = stages.reduce((sum, stage) => sum + count(stage.components), 0);
    const configurations = design.motors.configurations.length;
    console.log(
      `read ${path}: stages ${stages.length}, parts ${parts}, motor configurations ${configurations}`,
    );
  } catch (error) {
    if (!(error instanceof DesignFormatError)) {
      throw error;
    }
    refused += 1;
    console.log(`refused ${path}: ${error.message}`);
  }
}
process.exitCode = refused === 0 ? 0 : 1;
