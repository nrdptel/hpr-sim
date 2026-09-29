class DesignFormatError(ValueError):
    """A document the reader refused: not JSON, not an hpr design, another version, or not valid."""


def read_design(text: str) -> DesignFile:
    """Reads a design document (`.hpr` text) and checks it against the format's schema, so what
    comes back has the types above.

    It checks what the schema says: every required key present, no unknown key, each value of
    its type, each tagged union one of its forms. hpr's own reader checks a few things more that
    no schema can say, such as that two source files don't share a name, so hpr can still refuse
    a document this takes. Like hpr, it refuses two equal keys in one object, `NaN` and
    `Infinity`, and `2.0` where a whole number belongs.

    Raises `DesignFormatError` when the document is not one of this version.
    """
    try:
        # A byte-order mark, which some Windows editors write at the start of UTF-8, is not JSON.
        value = json.loads(
            text.removeprefix("﻿"),
            object_pairs_hook=_no_repeated_keys,
            parse_constant=_no_constants,
        )
    except ValueError as error:
        raise DesignFormatError(f"not JSON: {error}") from None
    if not isinstance(value, dict) or value.get("format") != FORMAT:
        raise DesignFormatError(f'not an hpr design: its "format" is not "{FORMAT}"')
    if value.get("version") != VERSION:
        raise DesignFormatError(
            f"written in version {json.dumps(value.get('version'))}; these types read {VERSION}"
            f" only (`hpr convert` rewrites an older document at {VERSION})"
        )
    problem = _check(value, _SCHEMA, "$")
    if problem is not None:
        raise DesignFormatError(f"{problem[0]}: {problem[1]}")
    return cast(DesignFile, value)


def _no_repeated_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, item in pairs:
        if key in result:
            raise ValueError(f"the key {json.dumps(key)} appears twice in one object")
        result[key] = item
    return result


def _no_constants(name: str) -> Any:
    raise ValueError(f"{name} is not a JSON number")


def _shown(value: Any) -> str:
    """`value` as JSON, cut to its first 40 characters."""
    text = json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    return text if len(text) <= 40 else text[:40] + "…"


def _is_type(value: Any, kind: str) -> bool:
    if kind == "null":
        return value is None
    if kind == "boolean":
        return isinstance(value, bool)
    if kind == "string":
        return isinstance(value, str)
    if kind == "number":
        return isinstance(value, (int, float)) and not isinstance(value, bool)
    if kind == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if kind == "array":
        return isinstance(value, list)
    if kind == "object":
        return isinstance(value, dict)
    return False


_IDENTIFIER = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")

_MOST = {"uint32": 4294967295, "uint": 18446744073709551615}


def _member(path: str, key: str) -> str:
    """`path` followed by the key `key`: `.name` where it can be, else `["na-me"]`."""
    return f"{path}.{key}" if _IDENTIFIER.fullmatch(key) else f"{path}[{json.dumps(key)}]"


def _union(value: Any, forms: list[Any], path: str, exactly_one: bool) -> tuple[str, str] | None:
    """The problem in the first of `forms` that fails deepest, or `None` if any form holds."""
    matched = 0
    deepest: tuple[str, str] | None = None
    for form in forms:
        problem = _check(value, form, path)
        if problem is None:
            matched += 1
        elif deepest is None or len(problem[0]) > len(deepest[0]):
            deepest = problem
    if exactly_one and matched > 1:
        return (path, f"matches {matched} of its forms, not one")
    if matched > 0:
        return None
    if deepest is not None and len(deepest[0]) > len(path):
        return deepest
    return (path, f"{_shown(value)} is none of the {len(forms)} forms allowed here")


def _check(value: Any, node: Any, path: str) -> tuple[str, str] | None:
    """The first problem with `value` against `node`, as its path and why, or `None`."""
    if "$ref" in node:
        target = _SCHEMA["$defs"].get(node["$ref"].removeprefix("#/$defs/"))
        if target is None:
            return (path, f"the schema has no {node['$ref']}")
        problem = _check(value, target, path)
        if problem is not None:
            return problem
    if "type" in node:
        kinds = [node["type"]] if isinstance(node["type"], str) else node["type"]
        if not any(_is_type(value, kind) for kind in kinds):
            return (path, f"is {_shown(value)}, not {' or '.join(kinds)}")
    if "const" in node and value != node["const"]:
        return (path, f"is {_shown(value)}, not {json.dumps(node['const'])}")
    for keyword, exactly_one in (("oneOf", True), ("anyOf", False)):
        if keyword in node:
            problem = _union(value, node[keyword], path, exactly_one)
            if problem is not None:
                return problem
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if "minimum" in node and value < node["minimum"]:
            return (path, f"is {value}, less than {node['minimum']}")
        most = _MOST.get(node.get("format", ""))
        if most is not None and value > most:
            return (path, f"is {value}, more than {node['format']} holds ({most})")
    if isinstance(value, str) and "pattern" in node and not re.search(node["pattern"], value):
        return (path, f"{json.dumps(value)} doesn't match {node['pattern']}")
    if isinstance(value, list):
        if "minItems" in node and len(value) < node["minItems"]:
            return (path, f"has {len(value)} items, fewer than {node['minItems']}")
        if "maxItems" in node and len(value) > node["maxItems"]:
            return (path, f"has {len(value)} items, more than {node['maxItems']}")
        prefix = node.get("prefixItems", [])
        for i, item in enumerate(value):
            item_node = prefix[i] if i < len(prefix) else node.get("items")
            problem = None if item_node is None else _check(item, item_node, f"{path}[{i}]")
            if problem is not None:
                return problem
    if isinstance(value, dict):
        for key in node.get("required", []):
            if key not in value:
                return (path, f"has no {json.dumps(key)}, which it needs")
        properties = node.get("properties", {})
        extra = node.get("additionalProperties", True)
        for key, item in value.items():
            if key in properties:
                problem = _check(item, properties[key], _member(path, key))
            elif extra is False:
                return (path, f"has the unknown key {json.dumps(key)}")
            elif isinstance(extra, dict):
                problem = _check(item, extra, _member(path, key))
            else:
                problem = None
            if problem is not None:
                return problem
    return None
