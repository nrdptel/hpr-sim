"""The guide's Python page, run: its code blocks in order, each printing what the page says.

`docs/python.md` shows each ```python block's output in the ```text block after it. These tests
run the blocks in one namespace, from the repository's root as the page's paths assume, and hold
each output to the page's text, so the page can't go stale.
"""

import contextlib
import io
import re

BLOCK = re.compile(r"```(python|text)\n(.*?)```", re.DOTALL)


def blocks(repo):
    """The page's python blocks, each with the text block that follows it, if one does."""
    page = (repo / "docs/python.md").read_text(encoding="utf-8")
    found = BLOCK.findall(page)
    pairs = []
    for index, (kind, code) in enumerate(found):
        if kind == "python":
            after = found[index + 1] if index + 1 < len(found) else None
            pairs.append((code, after[1] if after and after[0] == "text" else None))
    return pairs


def test_the_python_page_prints_what_it_says(repo, monkeypatch):
    pairs = blocks(repo)
    assert len(pairs) == 5, "the page's python blocks"
    monkeypatch.chdir(repo)
    namespace = {}
    for code, expected in pairs:
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            exec(compile(code, "docs/python.md", "exec"), namespace)
        assert expected is not None, f"a python block with no output shown:\n{code}"
        assert out.getvalue() == expected


def test_the_package_readme_runs(repo):
    readme = (repo / "crates/hpr-py/README.md").read_text(encoding="utf-8")
    (code,) = [code for kind, code in BLOCK.findall(readme) if kind == "python"]
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        exec(compile(code, "crates/hpr-py/README.md", "exec"), {})
    apogee_m, highest_m = (float(value) for value in out.getvalue().split())
    assert 0.0 < highest_m <= apogee_m

