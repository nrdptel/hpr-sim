/** A document the reader refused: not JSON, not an hpr design, another version, or not valid. */
export class DesignFormatError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "DesignFormatError";
    // Keeps `instanceof DesignFormatError` true when compiled for ES5.
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Reads a design document (`.hpr` text) and checks it against the format's schema, so what comes
 * back has the types above.
 *
 * It checks what the schema says: every required key present, no unknown key, each value of its
 * type, each tagged union one of its forms. hpr's own reader checks a few things more that no
 * schema can say, such as that two source files don't share a name, so hpr can still refuse a
 * document this takes. Like hpr, it refuses a number too large for a 64-bit float, a lone UTF-16
 * surrogate (`"\ud800"`), and nesting 128 levels deep. `JSON.parse` keeps the last of two equal
 * keys, where hpr refuses them, and can't tell `2.0` from `2`, which hpr refuses where it wants a
 * whole number; and a whole number of 2^53 or more, which it would round, is refused.
 *
 * @throws {DesignFormatError} The document is not one of this version.
 */
export function readDesign(text: string): DesignFile {
  let value: unknown;
  try {
    // A byte-order mark, which some Windows editors write at the start of UTF-8, is not JSON.
    value = JSON.parse(text.startsWith("\uFEFF") ? text.slice(1) : text);
  } catch (error) {
    throw new DesignFormatError(`not JSON: ${error instanceof Error ? error.message : String(error)}`);
  }
  const unread = scan(value);
  if (unread !== null) {
    throw new DesignFormatError(`not JSON: ${unread}`);
  }
  if (!isObject(value) || value.format !== FORMAT) {
    throw new DesignFormatError(`not an hpr design: its "format" is not "${FORMAT}"`);
  }
  if (value.version !== VERSION) {
    throw new DesignFormatError(versionMessage(value.version));
  }
  const problem = check(value, SCHEMA, "$");
  if (problem !== null) {
    throw new DesignFormatError(`${problem.path}: ${problem.message}`);
  }
  return value as unknown as DesignFile;
}

/** Why a document of `version`, which isn't this one, is refused, and what to do. */
function versionMessage(version: unknown): string {
  const match = typeof version === "string" ? /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/.exec(version) : null;
  if (match === null) {
    return `its "version" is ${shown(version)}, not a version such as "${VERSION}"`;
  }
  const [major, minor] = VERSION.split(".").map(Number);
  const [theirMajor, theirMinor] = [Number(match[1]), Number(match[2])];
  if (theirMajor > major || (theirMajor === major && theirMinor > minor)) {
    return `written in version ${version}, newer than these types, which read ${VERSION}: take the types from a newer hpr`;
  }
  return `written in version ${version}; these types read ${VERSION} only (\`hpr convert\` rewrites an older document at ${VERSION})`;
}

/** A JSON Schema node, as far as the reader uses one. */
interface SchemaNode {
  $defs?: { [name: string]: SchemaNode };
  $ref?: string;
  type?: string | string[];
  const?: string;
  oneOf?: SchemaNode[];
  anyOf?: SchemaNode[];
  properties?: { [name: string]: SchemaNode };
  required?: string[];
  additionalProperties?: boolean | SchemaNode;
  items?: SchemaNode;
  prefixItems?: SchemaNode[];
  minItems?: number;
  maxItems?: number;
  minimum?: number;
  pattern?: string;
  format?: string;
}

/**
 * Where in the document a check failed, and why; for a value that isn't one of the constants a
 * union allows there, the value and those constants, so the message can list them all.
 */
interface Problem {
  path: string;
  message: string;
  found?: string;
  expected?: string[];
}

/** The deepest nesting hpr reads: serde_json refuses a 128th level of arrays and objects. */
const MOST_LEVELS = 127;

/** A lone UTF-16 surrogate, which JSON can escape (`"\ud800"`) but hpr refuses. */
const LONE_SURROGATE = /[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?:^|[^\uD800-\uDBFF])[\uDC00-\uDFFF]/;

/** Why hpr couldn't read `value` as JSON although `JSON.parse` did, or `null`. */
function scan(value: unknown): string | null {
  const stack: Array<[unknown, number]> = [[value, 0]];
  while (stack.length > 0) {
    const [item, level] = stack.pop() as [unknown, number];
    if (typeof item === "number" && !Number.isFinite(item)) {
      return "a number too large for a 64-bit float";
    }
    if (typeof item === "string" && LONE_SURROGATE.test(item)) {
      return `a lone UTF-16 surrogate in ${shown(item)}`;
    }
    if (typeof item === "object" && item !== null) {
      if (level + 1 > MOST_LEVELS) {
        return `nested more than ${MOST_LEVELS} levels deep`;
      }
      for (const [key, child] of Object.entries(item)) {
        if (LONE_SURROGATE.test(key)) {
          return `a lone UTF-16 surrogate in the key ${shown(key)}`;
        }
        stack.push([child, level + 1]);
      }
    }
  }
  return null;
}

/** `value` as JSON, cut to its first 40 characters. */
function shown(value: unknown): string {
  const text = JSON.stringify(value) ?? String(value);
  return text.length > 40 ? `${text.slice(0, 40)}…` : text;
}

/** Whether `object` has `key` of its own, not from its prototype, as `constructor` would. */
function has(object: object, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(object, key);
}

function isObject(value: unknown): value is { [key: string]: unknown } {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isType(value: unknown, type: string): boolean {
  switch (type) {
    case "null":
      return value === null;
    case "boolean":
      return typeof value === "boolean";
    case "string":
      return typeof value === "string";
    case "number":
      return typeof value === "number" && Number.isFinite(value);
    case "integer":
      return typeof value === "number" && Number.isInteger(value);
    case "array":
      return Array.isArray(value);
    case "object":
      return isObject(value);
    default:
      return false;
  }
}

/** `path` followed by the key `key`: `.name` where it can be, else `["na-me"]`. */
function member(path: string, key: string): string {
  return /^[A-Za-z_][A-Za-z0-9_]*$/.test(key) ? `${path}.${key}` : `${path}[${JSON.stringify(key)}]`;
}

/**
 * The problem with `value` against the union `forms`, or `null` if a form holds (exactly one,
 * for `oneOf`): where the forms fail deepest, every constant they wanted there, or else the
 * first deepest problem, or else that the value is none of them.
 */
function union(value: unknown, forms: SchemaNode[], path: string, exactlyOne: boolean): Problem | null {
  let matched = 0;
  const problems: Problem[] = [];
  for (const form of forms) {
    const problem = check(value, form, path);
    if (problem === null) {
      matched += 1;
    } else {
      problems.push(problem);
    }
  }
  if (exactlyOne && matched > 1) {
    return { path, message: `matches ${matched} of its forms, not one` };
  }
  if (matched > 0) {
    return null;
  }
  const depth = Math.max(...problems.map((p) => p.path.length));
  const deepest = problems.filter((p) => p.path.length === depth);
  const constants = deepest.filter((p) => p.expected !== undefined);
  if (constants.length > 0 && deepest.every((p) => p.path === deepest[0].path)) {
    const choices = [...new Set(constants.flatMap((p) => p.expected ?? []))];
    const listed =
      choices.length === 1 ? choices[0] : `${choices.slice(0, -1).join(", ")} or ${choices[choices.length - 1]}`;
    const found = constants[0].found;
    return { path: deepest[0].path, message: `is ${found}, not ${listed}`, found, expected: choices };
  }
  if (depth > path.length) {
    return deepest[0];
  }
  return { path, message: `${shown(value)} is none of the ${forms.length} forms allowed here` };
}

/** The first problem with `value` against `node`, or `null` when it holds. */
function check(value: unknown, node: SchemaNode, path: string): Problem | null {
  if (node.$ref !== undefined) {
    const name = node.$ref.replace("#/$defs/", "");
    const defs = SCHEMA.$defs ?? {};
    if (!has(defs, name)) {
      return { path, message: `the schema has no ${node.$ref}` };
    }
    const problem = check(value, defs[name], path);
    if (problem !== null) {
      return problem;
    }
  }
  if (node.type !== undefined) {
    const types = typeof node.type === "string" ? [node.type] : node.type;
    if (!types.some((type) => isType(value, type))) {
      return { path, message: `is ${shown(value)}, not ${types.join(" or ")}` };
    }
  }
  if (node.const !== undefined && value !== node.const) {
    const [found, expected] = [shown(value), JSON.stringify(node.const)];
    return { path, message: `is ${found}, not ${expected}`, found, expected: [expected] };
  }
  if (node.oneOf !== undefined) {
    const problem = union(value, node.oneOf, path, true);
    if (problem !== null) {
      return problem;
    }
  }
  if (node.anyOf !== undefined) {
    const problem = union(value, node.anyOf, path, false);
    if (problem !== null) {
      return problem;
    }
  }
  if (typeof value === "number") {
    if (node.minimum !== undefined && value < node.minimum) {
      return { path, message: `is ${value}, less than ${node.minimum}` };
    }
    // A `uint` is 64 bits in hpr, but JavaScript holds whole numbers exactly only below 2^53.
    const most = node.format === "uint32" ? 4294967295 : node.format === "uint" ? Number.MAX_SAFE_INTEGER : null;
    if (most !== null && value > most) {
      return { path, message: `is ${value}, more than ${node.format} holds exactly here (${most})` };
    }
  }
  if (typeof value === "string" && node.pattern !== undefined && !new RegExp(node.pattern, "u").test(value)) {
    return { path, message: `${shown(value)} doesn't match ${node.pattern}` };
  }
  if (Array.isArray(value)) {
    if (node.minItems !== undefined && value.length < node.minItems) {
      return { path, message: `has ${value.length} items, fewer than ${node.minItems}` };
    }
    if (node.maxItems !== undefined && value.length > node.maxItems) {
      return { path, message: `has ${value.length} items, more than ${node.maxItems}` };
    }
    const prefix = node.prefixItems ?? [];
    for (let i = 0; i < value.length; i++) {
      const itemNode = i < prefix.length ? prefix[i] : node.items;
      const problem = itemNode === undefined ? null : check(value[i], itemNode, `${path}[${i}]`);
      if (problem !== null) {
        return problem;
      }
    }
  }
  if (isObject(value)) {
    for (const key of node.required ?? []) {
      if (!has(value, key)) {
        return { path, message: `has no ${JSON.stringify(key)}, which it needs` };
      }
    }
    for (const [key, item] of Object.entries(value)) {
      const itemNode = node.properties !== undefined && has(node.properties, key) ? node.properties[key] : undefined;
      if (itemNode !== undefined) {
        const problem = check(item, itemNode, member(path, key));
        if (problem !== null) {
          return problem;
        }
      } else if (node.additionalProperties === false) {
        return { path, message: `has the unknown key ${JSON.stringify(key)}` };
      } else if (typeof node.additionalProperties === "object") {
        const problem = check(item, node.additionalProperties, member(path, key));
        if (problem !== null) {
          return problem;
        }
      }
    }
  }
  return null;
}
