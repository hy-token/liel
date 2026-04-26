"""Cross-check the public ``.pyi`` stub against the live runtime extension.

Drift between ``python/liel/liel.pyi`` and the actual PyO3 module surface is
easy to introduce: someone adds a method on the Rust side without touching the
stub, or the stub gains a hand-edited helper that does not exist at runtime.
This test catches both directions by parsing the stub with ``ast`` and walking
the runtime with ``inspect.getmembers``.

The test intentionally restricts itself to *names* and (where the runtime
exposes them) *parameter names*. PyO3 does not always emit a real
``inspect.Signature`` for native methods, so we treat that as a soft warning,
not a hard failure.
"""

from __future__ import annotations

import ast
import inspect
from pathlib import Path

import liel

STUB_PATH = Path(liel.__file__).with_name("liel.pyi")

# Dunder methods that are intentionally part of the protocol surface but not
# part of the user-facing API. We do not require them to round-trip in either
# direction because their presence is implied by the corresponding behaviour
# (context manager, indexing, membership, ...).
PROTOCOL_DUNDERS = {
    "__enter__",
    "__exit__",
    "__getitem__",
    "__contains__",
    "__repr__",
    "__init__",
    "__new__",
    "__class__",
    "__doc__",
    "__module__",
    "__dict__",
    "__weakref__",
    "__hash__",
    "__eq__",
    "__ne__",
    "__lt__",
    "__le__",
    "__gt__",
    "__ge__",
    "__str__",
    "__sizeof__",
    "__reduce__",
    "__reduce_ex__",
    "__init_subclass__",
    "__subclasshook__",
    "__format__",
    "__getattribute__",
    "__setattr__",
    "__delattr__",
    "__dir__",
}

# Stub classes whose runtime equivalents we want to compare. The mapping value
# is the attribute name on the ``liel`` module.
PUBLIC_CLASSES = [
    "GraphDB",
    "Node",
    "Edge",
    "NodeQuery",
    "EdgeQuery",
    "MergeReport",
    "Transaction",
]


def _load_stub_module() -> ast.Module:
    """Parse ``liel.pyi`` into an AST module."""
    source = STUB_PATH.read_text(encoding="utf-8")
    return ast.parse(source, filename=str(STUB_PATH))


def _stub_class_members(module: ast.Module, class_name: str) -> set[str]:
    """Return the set of public member names declared on a stub class.

    A "member" is any function definition, async function definition, or
    annotated assignment whose name does not start with an underscore (with
    the exception of dunders we treat as part of the protocol surface and
    therefore exclude).
    """
    for node in module.body:
        if isinstance(node, ast.ClassDef) and node.name == class_name:
            members: set[str] = set()
            for child in node.body:
                if isinstance(child, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    name = child.name
                elif isinstance(child, ast.AnnAssign) and isinstance(child.target, ast.Name):
                    name = child.target.id
                else:
                    continue
                if name in PROTOCOL_DUNDERS:
                    continue
                if name.startswith("_"):
                    continue
                members.add(name)
            return members
    raise AssertionError(f"class {class_name!r} not found in {STUB_PATH}")


def _runtime_class_members(cls: type) -> set[str]:
    """Return the set of public member names exposed by the runtime class."""
    members: set[str] = set()
    for name, _value in inspect.getmembers(cls):
        if name in PROTOCOL_DUNDERS:
            continue
        if name.startswith("_"):
            continue
        members.add(name)
    return members


def _stub_function_params(module: ast.Module, func_name: str) -> list[str]:
    """Return the positional/keyword parameter names declared for a stub function."""
    for node in module.body:
        if isinstance(node, ast.FunctionDef) and node.name == func_name:
            args = node.args
            return [arg.arg for arg in (*args.posonlyargs, *args.args, *args.kwonlyargs)]
    raise AssertionError(f"function {func_name!r} not found in {STUB_PATH}")


def test_stub_classes_match_runtime_member_names():
    module = _load_stub_module()
    drift: dict[str, dict[str, set[str]]] = {}

    for class_name in PUBLIC_CLASSES:
        runtime_cls = getattr(liel, class_name)
        stub_members = _stub_class_members(module, class_name)
        runtime_members = _runtime_class_members(runtime_cls)

        only_in_stub = stub_members - runtime_members
        only_in_runtime = runtime_members - stub_members

        if only_in_stub or only_in_runtime:
            drift[class_name] = {
                "only_in_stub": only_in_stub,
                "only_in_runtime": only_in_runtime,
            }

    assert not drift, (
        "drift detected between liel.pyi and the runtime extension. "
        "Update python/liel/liel.pyi or the Rust binding so the public API matches.\n"
        f"{drift}"
    )


def test_module_open_signature_matches_stub():
    """``liel.open`` is part of the documented surface and should match the stub.

    PyO3 fills ``inspect.signature`` for top-level functions registered via
    ``wrap_pyfunction!``. If a future PyO3 release stops emitting a usable
    signature we still want this test to be useful, so we fall back to a
    skip-equivalent assertion that simply confirms the runtime symbol exists.
    """
    module = _load_stub_module()
    stub_params = _stub_function_params(module, "open")
    try:
        runtime_params = list(inspect.signature(liel.open).parameters)
    except (TypeError, ValueError):
        assert callable(liel.open), "liel.open must remain a callable"
        return

    assert stub_params == runtime_params, (
        f"liel.open parameter drift: stub={stub_params!r} runtime={runtime_params!r}"
    )
