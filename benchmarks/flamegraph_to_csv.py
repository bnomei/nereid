#!/usr/bin/env python3
from __future__ import annotations

import csv
import json
import re
import sys
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from pathlib import Path


TITLE_RE = re.compile(r"^(.*) \(([0-9,]+) samples?, ([0-9.]+)%\)$")


@dataclass
class Node:
    function: str
    samples: int
    percent: float
    x: str
    y: str
    width: str
    height: str


class UsageError(Exception):
    pass


def parse_args(argv: list[str]) -> tuple[Path, Path | None]:
    svg: Path | None = None
    out_prefix: Path | None = None

    it = iter(argv)
    for arg in it:
        if arg in ("--help", "-h"):
            raise UsageError("usage: flamegraph_to_csv <svg> [--out-prefix PATH]")
        if arg == "--out-prefix":
            try:
                value = next(it)
            except StopIteration as exc:
                raise ValueError("--out-prefix requires a value") from exc
            out_prefix = Path(value)
            continue

        if svg is None:
            svg = Path(arg)
            continue

        raise ValueError(f"unknown arg: {arg}")

    if svg is None:
        raise UsageError("usage: flamegraph_to_csv <svg> [--out-prefix PATH]")

    return svg, out_prefix


def strip_doctype(svg_text: str) -> str:
    while True:
        start = svg_text.find("<!DOCTYPE")
        if start == -1:
            break
        end_rel = svg_text[start:].find(">")
        if end_rel == -1:
            break
        svg_text = svg_text[:start] + svg_text[start + end_rel + 1 :]
    return svg_text


def local_name(tag: str) -> str:
    if "}" in tag:
        return tag.split("}", 1)[1]
    return tag


def parse_svg(svg_path: Path) -> list[Node]:
    svg_text = svg_path.read_text()
    svg_text = strip_doctype(svg_text)

    root = ET.fromstring(svg_text)
    nodes: list[Node] = []

    for g in root.iter():
        if local_name(g.tag) != "g":
            continue

        title_text: str | None = None
        rect = None
        for child in list(g):
            if not isinstance(child.tag, str):
                continue
            name = local_name(child.tag)
            if name == "title" and title_text is None:
                title_text = (child.text or "").strip()
            elif name == "rect" and rect is None:
                rect = child
            if title_text is not None and rect is not None:
                break

        if not title_text or rect is None:
            continue

        match = TITLE_RE.match(title_text)
        if match is None:
            continue

        function = match.group(1)
        samples = int(match.group(2).replace(",", ""))
        percent = float(match.group(3))

        nodes.append(
            Node(
                function=function,
                samples=samples,
                percent=percent,
                x=rect.get("x", ""),
                y=rect.get("y", ""),
                width=rect.get("width", ""),
                height=rect.get("height", ""),
            )
        )

    return nodes


def write_nodes_csv(nodes: list[Node], out_path: Path) -> None:
    with out_path.open("w", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(["function", "samples", "percent", "x", "y", "width", "height"])
        for node in nodes:
            writer.writerow(
                [
                    node.function,
                    str(node.samples),
                    f"{node.percent:.2f}",
                    node.x,
                    node.y,
                    node.width,
                    node.height,
                ]
            )


def write_top_json(nodes: list[Node], out_path: Path) -> None:
    # Flamegraph SVG nodes encode *inclusive* sample counts at a specific stack
    # position. The same function can appear in multiple nodes (different call
    # paths), so summing samples across nodes will over-count and can exceed
    # 100% of total samples.
    #
    # For a simple "what's hot" report, take the max inclusive samples per
    # function across all nodes.
    counts: dict[str, int] = {}
    for node in nodes:
        prev = counts.get(node.function, 0)
        if node.samples > prev:
            counts[node.function] = node.samples

    total = counts.get("all", max(counts.values(), default=0))
    items = sorted(counts.items(), key=lambda item: (-item[1], item[0]))

    payload = [
        {
            "function": function,
            "samples": samples,
            "percent": 0.0 if total == 0 else samples / total * 100.0,
        }
        for function, samples in items
    ]

    with out_path.open("w") as handle:
        json.dump(payload, handle, indent=2)
        handle.write("\n")


def run(argv: list[str]) -> int:
    try:
        svg, out_prefix = parse_args(argv)
    except UsageError as exc:
        print(exc, file=sys.stderr)
        return 1
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 1

    if not svg.exists():
        print(f"SVG not found: {svg}", file=sys.stderr)
        return 1

    prefix = out_prefix if out_prefix is not None else svg.with_suffix("")
    nodes_csv = prefix.parent / f"{prefix.name}.nodes.csv"
    top_json = prefix.parent / f"{prefix.name}.top.json"

    try:
        nodes = parse_svg(svg)
        write_nodes_csv(nodes, nodes_csv)
        write_top_json(nodes, top_json)
    except Exception as exc:
        print(str(exc), file=sys.stderr)
        return 1

    print(f"Wrote {nodes_csv} and {top_json} (nodes={len(nodes)})")
    return 0


def main() -> None:
    sys.exit(run(sys.argv[1:]))


if __name__ == "__main__":
    main()
