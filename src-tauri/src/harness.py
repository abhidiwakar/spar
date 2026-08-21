import io, json, sys, traceback, importlib.util, math, collections

sol_path, tests_path, result_path, entry, mode, helpers_json, params_json = sys.argv[1:8]
helpers = json.loads(helpers_json)
params = json.loads(params_json)
tests = json.loads(open(tests_path).read())


class ListNode:
    def __init__(self, val=0, next=None):
        self.val = val
        self.next = next


class TreeNode:
    def __init__(self, val=0, left=None, right=None):
        self.val = val
        self.left = left
        self.right = right


class GraphNode:
    def __init__(self, val=0, neighbors=None):
        self.val = val
        self.neighbors = neighbors if neighbors is not None else []


def list_from_array(arr, pos=-1):
    if arr is None:
        return None
    nodes = [ListNode(v) for v in arr]
    for i in range(len(nodes) - 1):
        nodes[i].next = nodes[i + 1]
    if pos is not None and pos >= 0 and nodes:
        nodes[-1].next = nodes[pos]
    return nodes[0] if nodes else None


def list_to_array(node, limit=10000):
    out = []
    seen = set()
    while node and len(out) < limit:
        if id(node) in seen:
            break
        seen.add(id(node))
        out.append(node.val)
        node = node.next
    return out


def tree_from_array(arr):
    if not arr:
        return None
    root = TreeNode(arr[0])
    q = collections.deque([root])
    i = 1
    while q and i < len(arr):
        node = q.popleft()
        if i < len(arr) and arr[i] is not None:
            node.left = TreeNode(arr[i])
            q.append(node.left)
        i += 1
        if i < len(arr) and arr[i] is not None:
            node.right = TreeNode(arr[i])
            q.append(node.right)
        i += 1
    return root


def tree_to_array(root):
    if not root:
        return []
    out = []
    q = collections.deque([root])
    while q:
        node = q.popleft()
        if node is None:
            out.append(None)
            continue
        out.append(node.val)
        q.append(node.left)
        q.append(node.right)
    while out and out[-1] is None:
        out.pop()
    return out


def graph_from_adj(adj):
    if not adj:
        return None
    nodes = [GraphNode(i + 1) for i in range(len(adj))]
    for i, nbrs in enumerate(adj):
        nodes[i].neighbors = [nodes[j - 1] for j in nbrs]
    return nodes[0]


def graph_to_adj(node):
    if not node:
        return []
    seen = {}
    q = collections.deque([node])
    seen[node.val] = node
    while q:
        cur = q.popleft()
        for n in cur.neighbors:
            if n.val not in seen:
                seen[n.val] = n
                q.append(n)
    n = max(seen) if seen else 0
    adj = [[] for _ in range(n)]
    for v, nd in seen.items():
        adj[v - 1] = [x.val for x in nd.neighbors]
    return adj


def encode(value, kind):
    if kind == "list":
        return list_to_array(value)
    if kind == "listlist":
        return [list_to_array(x) for x in (value or [])]
    if kind == "tree":
        return tree_to_array(value)
    if kind == "graph":
        return graph_to_adj(value)
    return value


def decode(value, kind, extra=None):
    extra = extra or {}
    if kind == "list":
        return list_from_array(value, extra.get("pos", -1))
    if kind == "listlist":
        return [list_from_array(x) for x in (value or [])]
    if kind == "tree":
        return tree_from_array(value)
    if kind == "graph":
        return graph_from_adj(value)
    return value


def deep_equal(a, b, compare):
    if compare == "sorted":
        if isinstance(a, list) and isinstance(b, list):
            try:
                return sorted(a) == sorted(b)
            except TypeError:
                return sorted(map(str, a)) == sorted(map(str, b))
    if compare == "set":
        if isinstance(a, list) and isinstance(b, list):
            def norm(x):
                return tuple(sorted(x)) if isinstance(x, list) else x
            return sorted(map(norm, a)) == sorted(map(norm, b))
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return abs(float(a) - float(b)) < 1e-9
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(deep_equal(x, y, "exact") for x, y in zip(a, b))
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(deep_equal(a[k], b[k], "exact") for k in a)
    return a == b


spec = importlib.util.spec_from_file_location("solution", sol_path)
mod = importlib.util.module_from_spec(spec)
sys.modules["solution"] = mod
_real_stdout = sys.stdout
_load_buf = io.StringIO()
sys.stdout = _load_buf
try:
    spec.loader.exec_module(mod)
except Exception:
    sys.stdout = _real_stdout
    open(result_path, "w").write(json.dumps({"compileError": traceback.format_exc()}))
    sys.exit(0)
finally:
    sys.stdout = _real_stdout
load_stdout = _load_buf.getvalue()

out_kind = helpers[0] if helpers else "json"
in_kinds = [h for h in helpers[1:] if h not in ("first_bad_version",)]

cases = []
for i, t in enumerate(tests):
    expected = t.get("expected")
    compare = t.get("compare") or "exact"
    err = None
    actual = None
    passed = False
    buf = io.StringIO()
    sys.stdout = buf
    try:
        if mode == "class":
            ops = t.get("ops")
            actual = []
            obj = None
            for step in ops:
                name = step[0]
                args = step[1:]
                if obj is None:
                    cls = getattr(mod, name)
                    obj = cls(*args)
                    actual.append(None)
                else:
                    actual.append(getattr(obj, name)(*args))
            passed = deep_equal(actual, expected, compare)
        else:
            args_obj = t.get("args") or {}
            extra = t.get("extra") or {}
            call_args = []
            if isinstance(args_obj, dict):
                for idx, name in enumerate(params):
                    kind = in_kinds[idx] if idx < len(in_kinds) else "json"
                    call_args.append(decode(args_obj.get(name), kind, extra))
            else:
                call_args = args_obj
            if "first_bad_version" in helpers:
                bad = extra.get("bad")
                if bad is None and isinstance(args_obj, dict):
                    bad = args_obj.get("bad")
                def isBadVersion(version, _bad=bad):
                    return version >= _bad
                mod.isBadVersion = isBadVersion
                import builtins
                builtins.isBadVersion = isBadVersion
            if hasattr(mod, "Solution"):
                fn = getattr(mod.Solution(), entry)
            elif hasattr(mod, entry):
                fn = getattr(mod, entry)
            else:
                raise AttributeError("Could not find %s in your code" % entry)
            raw = fn(*call_args)
            actual = encode(raw, out_kind)
            passed = deep_equal(actual, expected, compare)
    except Exception:
        err = traceback.format_exc()
        actual = None
        passed = False
    finally:
        sys.stdout = _real_stdout
    captured = buf.getvalue()
    if i == 0 and load_stdout:
        captured = load_stdout + captured
    cases.append({
        "index": i,
        "passed": passed,
        "expected": expected,
        "actual": actual,
        "error": err,
        "stdout": captured,
    })

open(result_path, "w").write(json.dumps({"cases": cases}, default=str))
