#!/usr/bin/env python3
"""Deterministic LSP protocol fixture, never a production semantic backend.

Supports simple named-function fixtures for cross-adapter contract tests.
"""

import json
import pathlib
import re
import subprocess
import sys
import time
import urllib.parse


def send(value):
    body = json.dumps(value).encode()
    sys.stdout.buffer.write(
        ("Content-Length: %d\r\n\r\n" % len(body)).encode() + body
    )
    sys.stdout.buffer.flush()


def read():
    length = None
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            return None
        if line in (b"\r\n", b"\n"):
            break
        if line.lower().startswith(b"content-length:"):
            length = int(line.split(b":", 1)[1])
    return json.loads(sys.stdin.buffer.read(length))


def file_path(uri):
    return pathlib.Path(urllib.parse.unquote(urllib.parse.urlparse(uri).path))


def position(text, offset):
    previous = text.rfind("\n", 0, offset)
    return {
        "line": text.count("\n", 0, offset),
        "character": len(text[previous + 1 : offset].encode("utf-16-le")) // 2,
    }


def source_range(text, start, end):
    return {"start": position(text, start), "end": position(text, end)}


def text_for(uri):
    if uri in documents:
        return documents[uri]["text"]
    path = file_path(uri)
    return path.read_text() if path.is_file() else ""


def sources():
    uris = set(documents)
    if root is not None:
        uris.update(path.resolve().as_uri() for path in root.rglob("*.rs")
                    if "target" not in path.parts)
    return [(uri, text_for(uri)) for uri in sorted(uris)]


def functions(uri, text):
    result = []
    for found in re.finditer(r"\b(?:pub\s+)?fn\s+(\w+)\s*\(", text):
        brace = text.find("{", found.end())
        end = found.end()
        if brace >= 0:
            depth = 1
            end = brace + 1
            while end < len(text) and depth:
                depth += (text[end] == "{") - (text[end] == "}")
                end += 1
        result.append({
            "name": found.group(1),
            "kind": 12,
            "range": source_range(text, found.start(), end),
            "selectionRange": source_range(text, found.start(1), found.end(1)),
            "uri": uri,
        })
    return result


def target_name(params):
    uri = params["textDocument"]["uri"]
    text = text_for(uri)
    wanted = params["position"]
    lines = text.splitlines(keepends=True)
    start = sum(len(line) for line in lines[: wanted["line"]])
    line = lines[wanted["line"]] if wanted["line"] < len(lines) else ""
    units = 0
    offset = start
    for char in line:
        if units >= wanted["character"]:
            break
        units += len(char.encode("utf-16-le")) // 2
        offset += 1
    for word in re.finditer(r"\w+", text):
        if word.start() <= offset < word.end():
            if word.group() not in {"pub", "fn"}:
                return word.group()
    available = functions(uri, text)
    for function in available:
        if function["range"]["start"]["line"] <= wanted["line"] <= function["range"]["end"]["line"]:
            return function["name"]
    return available[0]["name"] if available else ""


documents = {}
opened = None
configuration = None
root = None

while True:
    message = read()
    if message is None:
        break
    method = message.get("method")
    params = message.get("params") or {}
    if method == "initialize":
        root = file_path(params["rootUri"])
        configuration = params["initializationOptions"]
        send({"jsonrpc": "2.0", "id": message["id"], "result": {
            "capabilities": {"positionEncoding": "utf-16"},
            "serverInfo": {"version": "fixture-1"},
        }})
        continue
    if method == "initialized":
        send({"jsonrpc": "2.0", "id": 700, "method": "workspace/configuration",
              "params": {"items": [{"section": "rust-analyzer"}]}})
        continue
    if message.get("id") == 700:
        assert message["result"] == [configuration]
        send({"jsonrpc": "2.0", "method": "experimental/serverStatus",
              "params": {"health": "ok", "quiescent": True}})
        continue
    if method == "textDocument/didOpen":
        opened = params["textDocument"]
        documents[opened["uri"]] = opened
        continue
    if method == "textDocument/didChange":
        opened = documents[params["textDocument"]["uri"]]
        opened["version"] = params["textDocument"]["version"]
        opened["text"] = params["contentChanges"][0]["text"]
        continue
    if method == "fixture/slow":
        time.sleep(10)
        continue
    if method == "fixture/no-read":
        send({"jsonrpc": "2.0", "id": message["id"], "result": None})
        time.sleep(10)
        continue
    if method == "fixture/state":
        result = {"document": opened, "configuration": configuration}
    elif method == "fixture/warning":
        send({"jsonrpc": "2.0", "method": "experimental/serverStatus", "params": {
            "health": "warning", "quiescent": True,
            "message": "fixture workspace needs attention",
        }})
        result = None
    elif method == "fixture/spawn-child":
        child = subprocess.Popen(
            [sys.executable, "-c", "import time; time.sleep(30)"],
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        result = {"pid": child.pid}
    elif method == "workspace/symbol":
        result = [
            {"name": function["name"], "kind": function["kind"],
             "location": {"uri": uri, "range": function["selectionRange"]}}
            for uri, text in sources()
            for function in functions(uri, text)
            if params.get("query", "") in function["name"]
        ]
    elif method == "textDocument/documentSymbol":
        uri = params["textDocument"]["uri"]
        result = [{key: value for key, value in function.items() if key != "uri"}
                  for function in functions(uri, text_for(uri))]
    elif method == "textDocument/definition":
        name = target_name(params)
        result = [{"uri": uri, "range": function["selectionRange"]}
                  for uri, text in sources()
                  for function in functions(uri, text)
                  if function["name"] == name]
    elif method == "textDocument/references":
        name = target_name(params)
        result = []
        for uri, text in sources():
            declarations = [function["selectionRange"] for function in functions(uri, text)]
            for word in re.finditer(r"\b" + re.escape(name) + r"\b", text):
                span = source_range(text, word.start(), word.end())
                if params.get("context", {}).get("includeDeclaration") or span not in declarations:
                    result.append({"uri": uri, "range": span})
    elif method == "textDocument/hover":
        result = {"contents": {"kind": "plaintext", "value": "fn " + target_name(params) + "() -> i32"}}
    elif method == "textDocument/typeDefinition":
        result = []
    elif method == "rust-analyzer/relatedTests":
        name = target_name(params)
        result = []
        for uri, text in sources():
            for function in functions(uri, text):
                if "test" in function["name"]:
                    result.append({"runnable": {
                        "label": function["name"], "kind": "cargo",
                        "location": {"targetUri": uri, "targetRange": function["range"],
                                     "targetSelectionRange": function["selectionRange"]},
                        "args": {"cargoArgs": ["test", function["name"]],
                                 "cargoExtraArgs": [], "executableArgs": [],
                                 "workspaceRoot": str(root)},
                    }})
    else:
        send({"jsonrpc": "2.0", "id": message["id"], "error": {
            "code": -32601, "message": "unsupported fixture method",
        }})
        continue
    send({"jsonrpc": "2.0", "id": message["id"], "result": result})
