class DesignFormatError(ValueError):
    """A document the reader refused: not JSON, not an hpr design, another version, or not valid."""


def read_design(text: str) -> DesignFile:
    """Reads a design document (`.hpr` text) and checks it against the format's schema, so what
    comes back has the types above.

    It checks what the schema says: every required key present, no unknown key, each value of
    its type, each tagged union one of its forms. hpr's own reader checks a few things more that
    no schema can say, such as that two source files don't share a name, so hpr can still refuse
    a document this takes. Like hpr, it refuses two equal keys in one object, `NaN`, `Infinity`
    and any number too large for a 64-bit float, a lone UTF-16 surrogate (`"\\ud800"`), nesting
    128 levels deep, and `2.0` where a whole number belongs.

    Raises `DesignFormatError` when the document is not one of this version.
    """
    try:
        # A byte-order mark, which some Windows editors write at the start of UTF-8, is not JSON.
        value = json.loads(
            text.removeprefix("﻿"),
            object_pairs_hook=_no_repeated_keys,
            parse_constant=_no_constants,
        )
    except RecursionError:
        raise DesignFormatError(f"not JSON: nested more than {_MOST_LEVELS} levels deep") from None
    except ValueError as error:
        raise DesignFormatError(f"not JSON: {error}") from None
    unread = _scan(value)
    if unread is not None:
        raise DesignFormatError(f"not JSON: {unread}")
    if not isinstance(value, dict) or value.get("format") != FORMAT:
        raise DesignFormatError(f'not an hpr design: its "format" is not "{FORMAT}"')
    if value.get("version") != VERSION:
        raise DesignFormatError(_version_message(value.get("version")))
    problem = _check(value, _SCHEMA, "$")
    if problem is not None:
        raise DesignFormatError(f"{problem[0]}: {problem[1]}")
    return cast(DesignFile, value)


def _version_message(version: Any) -> str:
    """Why a document of `version`, which isn't this one, is refused, and what to do."""
    match = re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", version, re.ASCII) if isinstance(
        version, str
    ) else None
    if match is None:
        return f'its "version" is {_shown(version)}, not a version such as "{VERSION}"'
    theirs = (int(match[1]), int(match[2]))
    ours = tuple(int(part) for part in VERSION.split("."))
    if theirs > ours:
        return (
            f"written in version {version}, newer than these types, which read {VERSION}:"
            " take the types from a newer hpr"
        )
    return (
        f"written in version {version}; these types read {VERSION} only"
        f" (`hpr convert` rewrites an older document at {VERSION})"
    )


# The deepest nesting hpr reads: serde_json refuses a 128th level of arrays and objects.
_MOST_LEVELS = 127

# The largest finite 64-bit float.
_LARGEST_FLOAT = 1.7976931348623157e308


def _no_repeated_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, item in pairs:
        if key in result:
            raise ValueError(f"the key {json.dumps(key)} appears twice in one object")
        result[key] = item
    return result


def _no_constants(name: str) -> Any:
    raise ValueError(f"{name} is not a JSON number")


def _is_text(text: str) -> bool:
    """Whether `text` is Unicode, with no lone UTF-16 surrogate, which JSON can escape."""
    try:
        text.encode("utf-8")
    except UnicodeEncodeError:
        return False
    return True


def _scan(value: Any) -> str | None:
    """Why hpr couldn't read `value` as JSON although Python's reader did, or `None`."""
    stack: list[tuple[Any, int]] = [(value, 0)]
    while stack:
        item, level = stack.pop()
        if isinstance(item, float) and not math.isfinite(item):
            return "a number too large for a 64-bit float"
        if isinstance(item, int) and not isinstance(item, bool) and abs(item) > _LARGEST_FLOAT:
            return "a number too large for a 64-bit float"
        if isinstance(item, str) and not _is_text(item):
            return f"a lone UTF-16 surrogate in {_shown(item)}"
        if isinstance(item, (dict, list)):
            if level + 1 > _MOST_LEVELS:
                return f"nested more than {_MOST_LEVELS} levels deep"
            for key in item if isinstance(item, dict) else ():
                if not _is_text(key):
                    return f"a lone UTF-16 surrogate in the key {_shown(key)}"
            children = item.values() if isinstance(item, dict) else item
            stack.extend((child, level + 1) for child in children)
    return None


def _shown(value: Any) -> str:
    """`value` as JSON, cut to its first 40 characters."""
    text = json.dumps(value, separators=(",", ":"))
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

_PATTERNS: dict[str, re.Pattern[str]] = {}


def _pattern(pattern: str) -> re.Pattern[str]:
    """`pattern` as JSON Schema reads it: ASCII classes, and `$` at the very end of the text only,
    where Python's `$` would also match before a final newline."""
    if pattern not in _PATTERNS:
        translated = pattern[:-1] + r"\Z" if pattern.endswith("$") else pattern
        _PATTERNS[pattern] = re.compile(translated, re.ASCII)
    return _PATTERNS[pattern]


# A problem: where in the document a check failed, and why; for a value that isn't a union's
# constant, also the value and the constant, so a union can list every constant it allows.
_Problem = tuple[str, str, Union[str, None], Union[str, None]]


def _member(path: str, key: str) -> str:
    """`path` followed by the key `key`: `.name` where it can be, else `["na-me"]`."""
    return f"{path}.{key}" if _IDENTIFIER.fullmatch(key) else f"{path}[{json.dumps(key)}]"


def _union(value: Any, forms: list[Any], path: str, exactly_one: bool) -> _Problem | None:
    """The problem with `value` against the union `forms`, or `None` if a form holds (exactly
    one, for `oneOf`): where the forms fail deepest, every constant they wanted there, or else the
    first deepest problem, or else that the value is none of them."""
    matched = 0
    problems: list[_Problem] = []
    for form in forms:
        problem = _check(value, form, path)
        if problem is None:
            matched += 1
        else:
            problems.append(problem)
    if exactly_one and matched > 1:
        return (path, f"matches {matched} of its forms, not one", None, None)
    if matched > 0:
        return None
    depth = max((len(p[0]) for p in problems), default=-1)
    deepest = [p for p in problems if len(p[0]) == depth]
    wanted = [p[3] for p in deepest]
    if len(deepest) > 1 and all(p[0] == deepest[0][0] for p in deepest) and None not in wanted:
        choices = list(dict.fromkeys(w for w in wanted if w is not None))
        listed = f"{', '.join(choices[:-1])} or {choices[-1]}"
        return (deepest[0][0], f"is {deepest[0][2]}, not {listed}", None, None)
    if depth > len(path):
        return deepest[0]
    return (path, f"{_shown(value)} is none of the {len(forms)} forms allowed here", None, None)


def _check(value: Any, node: Any, path: str) -> _Problem | None:
    """The first problem with `value` against `node`, or `None` when it holds."""
    if "$ref" in node:
        target = _SCHEMA["$defs"].get(node["$ref"].removeprefix("#/$defs/"))
        if target is None:
            return (path, f"the schema has no {node['$ref']}", None, None)
        problem = _check(value, target, path)
        if problem is not None:
            return problem
    if "type" in node:
        kinds = [node["type"]] if isinstance(node["type"], str) else node["type"]
        if not any(_is_type(value, kind) for kind in kinds):
            return (path, f"is {_shown(value)}, not {' or '.join(kinds)}", None, None)
    if "const" in node and value != node["const"]:
        found, expected = _shown(value), json.dumps(node["const"])
        return (path, f"is {found}, not {expected}", found, expected)
    for keyword, exactly_one in (("oneOf", True), ("anyOf", False)):
        if keyword in node:
            problem = _union(value, node[keyword], path, exactly_one)
            if problem is not None:
                return problem
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if "minimum" in node and value < node["minimum"]:
            return (path, f"is {value}, less than {node['minimum']}", None, None)
        most = _MOST.get(node.get("format", ""))
        if most is not None and value > most:
            return (path, f"is {value}, more than {node['format']} holds ({most})", None, None)
    if isinstance(value, str) and "pattern" in node and not _pattern(node["pattern"]).search(value):
        return (path, f"{_shown(value)} doesn't match {node['pattern']}", None, None)
    if isinstance(value, list):
        if "minItems" in node and len(value) < node["minItems"]:
            return (path, f"has {len(value)} items, fewer than {node['minItems']}", None, None)
        if "maxItems" in node and len(value) > node["maxItems"]:
            return (path, f"has {len(value)} items, more than {node['maxItems']}", None, None)
        prefix = node.get("prefixItems", [])
        for i, item in enumerate(value):
            item_node = prefix[i] if i < len(prefix) else node.get("items")
            problem = None if item_node is None else _check(item, item_node, f"{path}[{i}]")
            if problem is not None:
                return problem
    if isinstance(value, dict):
        for key in node.get("required", []):
            if key not in value:
                return (path, f"has no {json.dumps(key)}, which it needs", None, None)
        properties = node.get("properties", {})
        extra = node.get("additionalProperties", True)
        for key, item in value.items():
            if key in properties:
                problem = _check(item, properties[key], _member(path, key))
            elif extra is False:
                return (path, f"has the unknown key {json.dumps(key)}", None, None)
            elif isinstance(extra, dict):
                problem = _check(item, extra, _member(path, key))
            else:
                problem = None
            if problem is not None:
                return problem
    return None
