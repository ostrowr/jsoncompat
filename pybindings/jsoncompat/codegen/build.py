"""Prepare generated dataclasses before application startup.

Usage: python -m jsoncompat.codegen.build models.py --output build/models.py

The input is executable Python and must be trusted, just like a module imported
by the application. The original generated module remains usable on its own.
"""

from __future__ import annotations

import argparse
import ast
import dataclasses
import hashlib
import importlib.util
import os
import py_compile
import sys
import tempfile
from collections.abc import Sequence
from pathlib import Path
from types import ModuleType

from jsoncompat import prepare_model_plan, prepare_model_schema

from . import dataclasses as dc
from .prepared import PREPARED_VERSION


def _load(path: Path) -> ModuleType:
    name = "_jsoncompat_build_" + hashlib.sha256(str(path).encode()).hexdigest()
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ValueError(f"Cannot load generated module {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    try:
        spec.loader.exec_module(module)
    except BaseException:
        sys.modules.pop(name, None)
        raise
    return module


type DescriptorValue = type[dc.DataclassModel] | tuple[
    "DescriptorValue", ...
] | Sequence["DescriptorValue"] | str | bool | int | float | None


def _expression(value: DescriptorValue) -> str:
    if isinstance(value, type):
        return value.__name__
    if isinstance(value, tuple):
        return (
            "("
            + ", ".join(_expression(item) for item in value)
            + (",)" if value else ")")
        )
    if value is None or isinstance(value, (str, bool, int, float)):
        return repr(value)
    return "[" + ", ".join(_expression(item) for item in value) + "]"


def _methods(model: type[dc.DataclassModel]) -> list[ast.stmt]:
    fields = dataclasses.fields(model)
    names = tuple(field.name for field in fields)
    parameters = ["self", "*", "skip_validation: bool = False"]
    for field in fields:
        parameter = field.name + ": " + str(field.type)
        if field.default is dc.JSONCOMPAT_MISSING:
            parameter += "=dc.JSONCOMPAT_MISSING"
        elif field.default_factory is not dataclasses.MISSING:
            parameter += '=getattr(_dataclasses, "_HAS_DEFAULT_FACTORY")'
        elif field.default is not dataclasses.MISSING:
            raise ValueError("Generated fields cannot define Python defaults")
        parameters.append(parameter)
    own_values = (
        "(" + ", ".join(f"self.{name}" for name in names) + ("," if names else "") + ")"
    )
    other_values = (
        "("
        + ", ".join(f"other.{name}" for name in names)
        + ("," if names else "")
        + ")"
    )
    shown = ", ".join(
        f"{field.name}={{self.{field.name}!r}}" for field in fields if field.repr
    )
    source = f"""
__slots__ = {names!r}
__match_args__ = ()

def __init__({", ".join(parameters)}) -> None:
    self.__post_init__(skip_validation)

@_recursive_repr()
def __repr__(self) -> str:
    return self.__class__.__qualname__ + f"({shown})"

def __eq__(self, other: typing.Any) -> bool:
    if other.__class__ is self.__class__:
        return {own_values} == {other_values}
    return NotImplemented

def __hash__(self) -> int:
    return hash({own_values})

def __setattr__(self, name: str, value: object) -> typing.NoReturn:
    raise _dataclasses.FrozenInstanceError(f"cannot assign to field {{name!r}}")

def __delattr__(self, name: str) -> typing.NoReturn:
    raise _dataclasses.FrozenInstanceError(f"cannot delete field {{name!r}}")

def __getstate__(self) -> list[typing.Any]:
    return list({own_values})

def __setstate__(self, state: typing.Sequence[typing.Any]) -> None:
    for name, value in zip({names!r}, state):
        object.__setattr__(self, name, value)
"""
    return ast.parse(source).body


def _prepare_class(
    node: ast.ClassDef, model: type[dc.DataclassModel]
) -> list[ast.stmt]:
    definitions: list[str] = []
    body: list[ast.stmt] = []
    for statement in node.body:
        if isinstance(statement, ast.AnnAssign) and isinstance(
            statement.target, ast.Name
        ):
            name = statement.target.id
            annotation = ast.unparse(statement.annotation)
            if name == dc.JSONCOMPAT_SCHEMA_FIELD:
                expression = f"_dataclasses.field(default={model.__name__}.{name})"
                kind = "classvar"
            else:
                if statement.value is None:
                    raise ValueError(
                        f"Generated field {model.__name__}.{name} lacks metadata"
                    )
                expression = ast.unparse(statement.value)
                kind = "field"
                statement.value = None
            definitions.append(f"({name!r}, {annotation!r}, {expression}, {kind!r})")
        body.append(statement)
    node.decorator_list = [ast.parse("typing.final", mode="eval").body]
    node.body = _methods(model) + body
    prepared = prepare_model_schema(model.__jsoncompat_schema__)
    setattr(model, "__jsoncompat_prepared_schema__", prepared)
    metadata = ", ".join(definitions)
    tail = ast.parse(f"""
_finish_dataclass({model.__name__}, ({metadata},))
setattr({model.__name__}, "__jsoncompat_prepared_schema__", {prepared!r})
{model.__name__}.__doc__ = {model.__doc__!r}
""").body
    return [node, *tail]


def prepare_module(source: Path, destination: Path) -> None:
    """Write a self-contained prepared module, replacing the destination atomically."""
    source = source.resolve()
    destination = destination.resolve()
    if source == destination:
        raise ValueError("Build output must differ from the original generated module")
    module = _load(source)
    try:
        root = getattr(module, "JSONCOMPAT_MODEL", None)
        if not isinstance(root, type) or not issubclass(root, dc.DataclassModel):
            raise ValueError("Input is not a jsoncompat-generated dataclass module")
        models = dict.fromkeys(
            value
            for value in vars(module).values()
            if isinstance(value, type)
            and issubclass(value, dc.DataclassModel)
            and value.__module__ == module.__name__
        )
        inspect_model = (
            dc._inspect_generated_model  # pyright: ignore[reportPrivateUsage]
        )
        specs = {model: inspect_model(model, vars(module)) for model in models}
        builder = dc._NativePlanBuilder(specs)  # pyright: ignore[reportPrivateUsage]
        roots = [(model, builder.add(model)) for model in models]
        descriptors = builder.finish()
        tree = ast.parse(source.read_text(encoding="utf-8"), filename=str(source))
        body: list[ast.stmt] = []
        for statement in tree.body:
            if (
                isinstance(statement, ast.ImportFrom)
                and statement.module == "dataclasses"
            ):
                statement.names = [
                    name for name in statement.names if name.name != "dataclass"
                ]
                if not statement.names:
                    continue
            if isinstance(statement, ast.ClassDef) and statement.name in vars(module):
                model = getattr(module, statement.name)
                if model in models:
                    body.extend(_prepare_class(statement, model))
                    continue
            body.append(statement)
        imports = ast.parse("""
import dataclasses as _dataclasses
from reprlib import recursive_repr as _recursive_repr
from jsoncompat.codegen.prepared import finish_dataclass as _finish_dataclass, bind_module as _bind_module
""").body
        # __future__ imports must remain first.
        insertion = 0
        while insertion < len(body):
            statement = body[insertion]
            if (
                not isinstance(statement, ast.ImportFrom)
                or statement.module != "__future__"
            ):
                break
            insertion += 1
        body[insertion:insertion] = imports
        prepared_plan = prepare_model_plan(descriptors, dc.FrozenList, dc.FrozenDict)
        body.extend(
            ast.parse(
                f"_bind_module({PREPARED_VERSION}, {_expression(roots)}, {_expression(descriptors)}, {prepared_plan!r})"
            ).body
        )
        tree.body = body
        rendered = (
            "# Prepared by python -m jsoncompat.codegen.build; regenerate from the source module.\n"
            + ast.unparse(ast.fix_missing_locations(tree))
            + "\n"
        )
        # Check before publishing. Also cache bytecode for this interpreter;
        # wheel installation can regenerate it for another Python version.
        compile(rendered, str(destination), "exec")
        destination.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile(
            "w", encoding="utf-8", dir=destination.parent, delete=False
        ) as handle:
            temporary = Path(handle.name)
            handle.write(rendered)
        try:
            mode = (
                destination.stat().st_mode
                if destination.exists()
                else source.stat().st_mode
            )
            os.chmod(temporary, mode & 0o777)
            os.replace(temporary, destination)
            py_compile.compile(str(destination), doraise=True)
        finally:
            temporary.unlink(missing_ok=True)
    finally:
        sys.modules.pop(module.__name__, None)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        prepare_module(args.source, args.output)
    except (TypeError, ValueError) as error:
        parser.exit(1, f"Cannot prepare models: {error}\n")


if __name__ == "__main__":
    main()
