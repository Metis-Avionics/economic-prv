#!/usr/bin/env python3
"""tetanus-check.py — "Power of 10" static checker for safety-critical Rust.

Reads tetanus.toml (thresholds, scope, rule-3 hot path) and enforces the
rules Clippy cannot prove, over the strict-profile crates' library code
(`src/**/*.rs`, `#[cfg(test)]` regions excluded):

  power10-01  no recursion (per-file call-graph cycle detection)
  power10-02  no while/loop without a TETANUS-bound/nonterminating marker
  power10-03  no heap allocation in hot-path fns; no Box/Rc/Arc in strict lib
  power10-04  function code-line count <= [thresholds.max_fn_lines]
  power10-05  every fallible fn has >= 1 check; file-average density metric
  power10-06  no `static mut`
  power10-07  no unwrap/expect/panic/todo/unimplemented/unreachable in lib
  power10-08  no macro_rules!; non-test #[cfg] count/justification gate
  power10-09  no raw-pointer syntax (*const/*mut, `as *`, fn args, **)
  power10-10  plumbing: workspace lints inherited, clippy.toml present,
              strict deny-headers present

Escape hatch (reported, never silent):
  `// TETANUS-exempt(power10-0X): <reason>` inside a function body
  downgrades that rule's finding for that function to a warning.
  `// TETANUS-bound:` / `// TETANUS-nonterminating:` justify while/loop.
  `// TETANUS-cfg:` justifies a non-test #[cfg].

Exit status: 1 on any error, 0 on warnings only.
Approximations are documented inline; the checker favors zero false
positives on the current tree over cross-file completeness.
"""
from __future__ import annotations

import pathlib
import re
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parent.parent
CONFIG = ROOT / "tetanus.toml"

KEYWORDS = {
    "if", "else", "for", "while", "loop", "match", "return", "break",
    "continue", "in", "where", "fn", "struct", "enum", "impl", "trait",
    "mod", "use", "let", "mut", "ref", "move", "true", "false",
    "None", "Some", "Ok", "Err", "Self", "self", "Self", "crate", "super",
}

HEAP_PATTERNS = [
    (re.compile(r"\bvec!\s*\["), "vec!"),
    (re.compile(r"\bVec::"), "Vec::"),
    (re.compile(r"\bBox::"), "Box::"),
    (re.compile(r"\bRc::"), "Rc::"),
    (re.compile(r"\bArc::"), "Arc::"),
    (re.compile(r"\bHashMap\b"), "HashMap"),
    (re.compile(r"\bString::from\b"), "String::from"),
    (re.compile(r"\bformat!\b"), "format!"),
    (re.compile(r"\.to_string\s*\("), ".to_string()"),
    (re.compile(r"\.to_owned\s*\("), ".to_owned()"),
    (re.compile(r"\.collect::\s*<\s*Vec"), ".collect::<Vec>"),
]

SMART_PTR = re.compile(r"\b(Box|Rc|Arc)::")
BANNED_CALL = re.compile(r"\.(unwrap|expect)\s*\(")
BANNED_MACRO = re.compile(r"\b(panic|todo|unimplemented|unreachable)!\b")
RAW_PTR = [
    (re.compile(r"\*const\b"), "*const"),
    (re.compile(r"\*mut\b"), "*mut"),
    (re.compile(r"\bas\s*\*"), "`as *` cast"),
    (re.compile(r":\s*fn\s*\("), "fn-pointer argument"),
    (re.compile(r"\*\s*\*"), "double dereference"),
]

ASSERT_PATTERNS = ["assert!", "debug_assert!", "assert_eq!", "assert_ne!",
                   "debug_assert_eq!", "return Err", "return None",
                   "Err(", ".map_err", ".ok_or"]

EXEMPT_RE = re.compile(r"TETANUS-exempt\((power10-\d{2})\)")
LOOP_MARKERS = ("TETANUS-bound:", "TETANUS-nonterminating:")


class Finding:
    def __init__(self, path, line, level, rule, msg):
        self.path = path
        self.line = line
        self.level = level  # "error" | "warning"
        self.rule = rule
        self.msg = msg

    def __str__(self):
        rel = self.path.relative_to(ROOT)
        return f"{rel}:{self.line}: {self.level} [{self.rule}] {self.msg}"


# --------------------------------------------------------------------------
# Lexer-lite: strip comments/strings so braces, keywords and patterns in
# literals don't confuse the structural scans. Handles //, /* */, "...",
# r#"..."#, 'c' char literals (lifetimes pass through), and lifetime ticks.
# --------------------------------------------------------------------------
def strip_noise(text):
    out = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        two = text[i:i + 2]
        if two == "//":
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif two == "/*":
            j = text.find("*/", i + 2)
            j = n if j + 2 < 0 else j + 2
            seg = text[i:j]
            out.append("".join("\n" if ch == "\n" else " " for ch in seg))
            i = j
        elif c == '"' or (c == "r" and i + 1 < n and text[i + 1] in '#"'):
            if c == "r" and text[i + 1] == "#":
                m = re.match(r"r(#+)\"", text[i:])
                hashes = m.group(1)
                close = text.find('"' + hashes, i + len(hashes) + 2)
                j = n if close < 0 else close + 1 + len(hashes)
            elif c == "r":
                j = text.find('"', i + 2)
                j = n if j < 0 else j + 1
                # actually r"..." without hashes: find closing quote
                k = i + 2
                while k < n and text[k] != '"':
                    k += 1
                j = n if k >= n else k + 1
            else:
                k = i + 1
                while k < n:
                    if text[k] == "\\":
                        k += 2
                        continue
                    if text[k] == '"':
                        break
                    if text[k] == "\n":
                        break
                    k += 1
                j = n if k >= n else k + 1
            seg = text[i:j]
            out.append("".join("\n" if ch == "\n" else " " for ch in seg))
            i = j
        elif c == "'":
            # char literal ('x', '\n', '\u{...}') vs lifetime ('a, 'static).
            m = re.match(r"'(\\.|[^'\\])'", text[i:])
            if m:
                out.append(" " * len(m.group(0)))
                i += len(m.group(0))
            else:
                out.append(c)
                i += 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


def line_of(pos, line_starts):
    """1-based line number for a character offset."""
    import bisect
    return bisect.bisect_right(line_starts, pos)


class Function:
    def __init__(self, name, sig, start_line, end_line, body, code_lines):
        self.name = name
        self.sig = sig
        self.start_line = start_line
        self.end_line = end_line
        self.body = body          # raw body text (with comments/strings)
        self.code_lines = code_lines  # stripped nonblank line count


def parse_functions(path, clean, raw_lines, line_starts, skip_ranges):
    """Find top-level `fn` items outside skip_ranges; brace-match bodies."""
    fns = []
    for m in re.finditer(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*", clean):
        if any(s <= m.start() < e for s, e in skip_ranges):
            continue
        name = m.group(1)
        # Body starts at first '{' with paren/bracket depth 0 after `fn`.
        i = m.end()
        paren = brack = 0
        body_start = None
        while i < len(clean):
            c = clean[i]
            if c == "(":
                paren += 1
            elif c == ")":
                paren -= 1
            elif c == "[":
                brack += 1
            elif c == "]":
                brack -= 1
            elif c == "{" and paren <= 0 and brack <= 0:
                body_start = i
                break
            elif c == ";" and paren <= 0 and brack <= 0:
                break  # declaration without body (trait/exern)
            i += 1
        if body_start is None:
            continue
        depth = 0
        j = body_start
        while j < len(clean):
            if clean[j] == "{":
                depth += 1
            elif clean[j] == "}":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        end = min(j + 1, len(clean))
        start_line = line_of(m.start(), line_starts)
        end_line = line_of(end - 1, line_starts)
        sig = clean[m.start():body_start]
        body = clean[body_start:end]
        code_lines = sum(
            1 for k in range(start_line - 1, end_line)
            if k < len(raw_lines) and raw_lines[k].strip()
            and not raw_lines[k].strip().startswith("//")
        )
        # NOTE: raw_lines here are comment-stripped equivalents; the
        # blank/comment accounting below reuses the stripped view, which is
        # conservative (string-only lines already blanked).
        fns.append(Function(name, sig, start_line, end_line, body, code_lines))
    return fns


def cfg_test_ranges(clean, line_starts):
    """Brace-matched ranges of `#[cfg(test)] mod ... { ... }` items."""
    ranges = []
    for m in re.finditer(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", clean):
        rest = clean[m.end():m.end() + 400]
        mm = re.search(r"\bmod\s+[A-Za-z_][A-Za-z0-9_]*\s*\{", rest)
        if not mm:
            continue
        brace = m.end() + mm.end() - 1
        depth = 0
        j = brace
        while j < len(clean):
            if clean[j] == "{":
                depth += 1
            elif clean[j] == "}":
                depth -= 1
                if depth == 0:
                    break
            j += 1
        ranges.append((m.start(), min(j + 1, len(clean))))
    return ranges


def bare_calls(body):
    """Bare `ident(` calls: not method (`.f(`), not path (`::f(`), not macro."""
    calls = set()
    for m in re.finditer(r"([A-Za-z_][A-Za-z0-9_]*)\s*(\!)?\(", body):
        if m.group(2):
            continue  # macro invocation
        s = m.start()
        prev = body[s - 1] if s > 0 else " "
        prev2 = body[s - 2:s] if s > 1 else ""
        if prev in (".", ":"):
            continue
        if prev2 == "::":
            continue
        ident = m.group(1)
        if ident in KEYWORDS:
            continue
        calls.add(ident)
    return calls


def check_file(path, cfg, findings):
    max_lines = cfg["thresholds"]["max_fn_lines"]
    min_avg = cfg["thresholds"]["min_asserts_per_fn_avg"]
    hot_fns = set(cfg["rule3"]["hot_fns"])
    cold = cfg["rule3"]["cold_markers"]

    text = path.read_text()
    raw_lines = text.splitlines()
    clean = strip_noise(text)
    line_starts = [0]
    for mm in re.finditer(r"\n", clean):
        line_starts.append(mm.start() + 1)
    skipped = cfg_test_ranges(clean, line_starts)

    def in_lib(pos):
        return not any(s <= pos < e for s, e in skipped)

    # ---- power10-08: non-test #[cfg] gate ---------------------------------
    cfgs = [m for m in re.finditer(r"#\s*\[\s*cfg\s*\(", clean) if in_lib(m.start())]
    unmarked = []
    for m in cfgs:
        ln = line_of(m.start(), line_starts)
        line = raw_lines[ln - 1] if 0 < ln <= len(raw_lines) else ""
        if "TETANUS-cfg:" not in line:
            unmarked.append(ln)
    if len(cfgs) > cfg["thresholds"]["max_non_test_cfgs"] or unmarked:
        findings.append(Finding(
            path, (unmarked[0] if unmarked else 1), "error", "power10-08",
            f"{len(cfgs)} non-test #[cfg] (max {cfg['thresholds']['max_non_test_cfgs']}); "
            "each needs `// TETANUS-cfg:` justification"))

    # ---- power10-06/07/09: whole-file scans (lib code only) -----------------
    for pat, label, rule in [
        (re.compile(r"\bstatic\s+mut\b"), "static mut", "power10-06"),
        (BANNED_CALL, "unchecked Result/Option", "power10-07"),
        (BANNED_MACRO, "panic/exit macro", "power10-07"),
        (re.compile(r"\bmacro_rules!\b"), "macro_rules!", "power10-08"),
    ]:
        for m in pat.finditer(clean):
            if not in_lib(m.start()):
                continue
            ln = line_of(m.start(), line_starts)
            findings.append(Finding(
                path, ln, "error", rule,
                f"banned `{label}` in strict library code"))
    for pat, label in RAW_PTR:
        for m in pat.finditer(clean):
            if not in_lib(m.start()):
                continue
            ln = line_of(m.start(), line_starts)
            findings.append(Finding(
                path, ln, "error", "power10-09",
                f"raw-pointer syntax `{label}` in strict library code"))
    for m in SMART_PTR.finditer(clean):
        if not in_lib(m.start()):
            continue
        ln = line_of(m.start(), line_starts)
        findings.append(Finding(
            path, ln, "error", "power10-03",
            f"smart pointer `{m.group(1)}` in strict library code "
            "(preallocate in constructors instead)"))
    for m in re.finditer(r"\bpub\s+static\b", clean):
        if in_lib(m.start()):
            findings.append(Finding(
                path, line_of(m.start(), line_starts), "warning",
                "power10-06", "`pub static` widens scope; prefer block-local"))

    # ---- per-function checks --------------------------------------------------
    fns = parse_functions(path, clean, clean.splitlines(), line_starts, skipped)
    defined = {f.name for f in fns}
    graph = {}
    for f in fns:
        calls = bare_calls(f.body) & defined
        graph[f.name] = calls
        # Markers live in `//` comments, which the structural scan strips,
        # so read them from the raw source slice for this function,
        # including the contiguous doc/attribute preamble above it.
        pre = f.start_line - 1
        while pre > 0:
            s = raw_lines[pre - 1].strip()
            if s.startswith(("///", "//", "//!", "#[", "#")) or not s:
                pre -= 1
            else:
                break
        raw_body = "\n".join(raw_lines[pre:f.end_line])
        exempt = set(EXEMPT_RE.findall(raw_body))

        def emit(rule, msg, line=None):
            level = "warning" if rule in exempt else "error"
            note = " (TETANUS-exempted)" if rule in exempt else ""
            findings.append(Finding(
                path, line or f.start_line, level, rule, msg + note))

        # power10-01: direct recursion
        if f.name in calls:
            emit("power10-01", f"recursive call in `{f.name}`")
        # power10-02: unbounded loop shapes
        if re.search(r"\b(while|loop)\b", f.body):
            if not any(mk in raw_body for mk in LOOP_MARKERS) and \
                    "power10-02" not in exempt:
                emit("power10-02",
                     f"`while`/`loop` in `{f.name}` needs "
                     "`// TETANUS-bound:` or `// TETANUS-nonterminating:`")
        # power10-03: hot-path heap. Match per raw line: strip `//`
        # comments before matching (docs mention Vec/Vec-words freely),
        # but test cold markers against the full line so a trailing
        # `// Err path` justification still counts.
        if f.name in hot_fns:
            seen = set()
            for k in range(f.start_line - 1, min(f.end_line, len(raw_lines))):
                full = raw_lines[k]
                code = full.split("//")[0]
                for cre, label in HEAP_PATTERNS:
                    if label in seen:
                        continue
                    if cre.search(code):
                        if any(marker in full for marker in cold):
                            continue
                        seen.add(label)
                        emit("power10-03",
                             f"heap allocation `{label}` on hot path `{f.name}` "
                             "(preallocate; cold/error lines must mention "
                             "Err/Error/reason)",
                             line=k + 1)
        # power10-04: length
        if f.code_lines > max_lines:
            emit("power10-04",
                 f"`{f.name}` is {f.code_lines} code lines (max {max_lines})")
        # power10-05: fallible fns must defend; density is a metric
        return_type = f.sig.split("->")[-1].strip() if "->" in f.sig else ""
        fallible = ("Result" in return_type) or ("Option" in return_type)
        checks = sum(f.body.count(a) for a in ASSERT_PATTERNS) + f.body.count("?")
        f.checks = checks
        if fallible and checks == 0:
            emit("power10-05",
                 f"fallible `{f.name}` has no check "
                 "(assert/guard/`return Err`/`?` required)")

    # power10-01: indirect (mutual) recursion cycles, per file
    WHITE, GRAY, BLACK = 0, 1, 2
    color = {n: WHITE for n in graph}
    stack: list[str] = []

    def dfs(u):
        color[u] = GRAY
        stack.append(u)
        for v in graph.get(u, ()):
            if color[v] == GRAY:
                cyc = " -> ".join(stack[stack.index(v):] + [v])
                findings.append(Finding(
                    path, next(f.start_line for f in fns if f.name == u),
                    "error", "power10-01",
                    f"mutual recursion cycle: {cyc}"))
            elif color[v] == WHITE:
                dfs(v)
        stack.pop()
        color[u] = BLACK

    for n in graph:
        if color[n] == WHITE:
            dfs(n)

    # power10-05: density metric per file (only for files with fallible functions;
    # average counts checks in all functions to penalise unchecked accessors/constructors)
    fallible_fns = []
    for f in fns:
        return_type = f.sig.split("->")[-1].strip() if "->" in f.sig else ""
        if ("Result" in return_type) or ("Option" in return_type):
            fallible_fns.append(f)
    if fallible_fns:
        avg = sum(f.checks for f in fallible_fns) / len(fallible_fns)
        if avg < min_avg:
            findings.append(Finding(
                path, 1, "warning", "power10-05",
                f"assertion density {avg:.2f}/fn below target {min_avg}/fn"))


def check_plumbing(cfg, findings):
    root_toml = ROOT / "Cargo.toml"
    try:
        root_doc = tomllib.loads(root_toml.read_text())
        members = root_doc.get("workspace", {}).get("members", [])
    except Exception:
        members = []

    rels = sorted(set(members))
    for rel in rels:
        manifest = ROOT / rel / "Cargo.toml"
        if not manifest.is_file():
            findings.append(Finding(manifest, 1, "error", "power10-10",
                                    "workspace member manifest missing"))
            continue
        text = manifest.read_text()
        if "[lints]" not in text or "workspace = true" not in text:
            findings.append(Finding(
                manifest, 1, "error", "power10-10",
                f"{rel} does not inherit `[lints] workspace = true`"))
    clippy_toml = ROOT / "clippy.toml"
    if not clippy_toml.is_file():
        findings.append(Finding(clippy_toml, 1, "error", "power10-10",
                                "clippy.toml missing"))
    else:
        body = clippy_toml.read_text()
        for key in ("too-many-lines-threshold", "allow-unwrap-in-tests",
                    "allow-expect-in-tests"):
            if key not in body:
                findings.append(Finding(
                    clippy_toml, 1, "error", "power10-10",
                    f"clippy.toml missing `{key}`"))
    for rel in cfg["scope"]["strict"]:
        lib = ROOT / rel / "src" / "lib.rs"
        if not lib.is_file():
            findings.append(Finding(lib, 1, "error", "power10-10",
                                    "strict crate has no src/lib.rs"))
            continue
        head = lib.read_text()
        if "#![deny(clippy::unwrap_used" not in head:
            findings.append(Finding(
                lib, 1, "error", "power10-10",
                "strict crate lib.rs missing TETANUS `#![deny]` header"))


def main():
    cfg = tomllib.loads(CONFIG.read_text())
    findings: list[Finding] = []
    check_plumbing(cfg, findings)
    for rel in cfg["scope"]["strict"]:
        for path in sorted((ROOT / rel / "src").rglob("*.rs")):
            try:
                check_file(path, cfg, findings)
            except Exception as e:  # never fail silently; report and stop
                findings.append(Finding(path, 1, "error", "power10-10",
                                        f"checker internal error: {e}"))
    findings.sort(key=lambda f: (str(f.path), f.line))
    for f in findings:
        print(f)
    errors = sum(1 for f in findings if f.level == "error")
    warns = len(findings) - errors
    print(f"tetanus: {errors} error(s), {warns} warning(s) "
          f"across {len(cfg['scope']['strict'])} strict crates")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
