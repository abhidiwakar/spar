const fs = require("fs");
const vm = require("vm");

const [solPath, testsPath, resultPath, entry, mode, helpersJson, paramsJson] = process.argv.slice(2);
const helpers = JSON.parse(helpersJson);
const params = JSON.parse(paramsJson);
const tests = JSON.parse(fs.readFileSync(testsPath, "utf8"));

class ListNode {
  constructor(val, next) {
    this.val = val === undefined ? 0 : val;
    this.next = next === undefined ? null : next;
  }
}
class TreeNode {
  constructor(val, left, right) {
    this.val = val === undefined ? 0 : val;
    this.left = left === undefined ? null : left;
    this.right = right === undefined ? null : right;
  }
}
class GraphNode {
  constructor(val, neighbors) {
    this.val = val === undefined ? 0 : val;
    this.neighbors = neighbors === undefined ? [] : neighbors;
  }
}

function listFromArray(arr, pos = -1) {
  if (!arr || arr.length === 0) return null;
  const nodes = arr.map((v) => new ListNode(v));
  for (let i = 0; i < nodes.length - 1; i++) nodes[i].next = nodes[i + 1];
  if (pos >= 0) nodes[nodes.length - 1].next = nodes[pos];
  return nodes[0];
}
function listToArray(node, limit = 10000) {
  const out = [];
  const seen = new Set();
  while (node && out.length < limit) {
    if (seen.has(node)) break;
    seen.add(node);
    out.push(node.val);
    node = node.next;
  }
  return out;
}
function treeFromArray(arr) {
  if (!arr || arr.length === 0) return null;
  const root = new TreeNode(arr[0]);
  const q = [root];
  let i = 1;
  while (q.length && i < arr.length) {
    const node = q.shift();
    if (i < arr.length && arr[i] !== null) {
      node.left = new TreeNode(arr[i]);
      q.push(node.left);
    }
    i++;
    if (i < arr.length && arr[i] !== null) {
      node.right = new TreeNode(arr[i]);
      q.push(node.right);
    }
    i++;
  }
  return root;
}
function treeToArray(root) {
  if (!root) return [];
  const out = [];
  const q = [root];
  while (q.length) {
    const node = q.shift();
    if (!node) {
      out.push(null);
      continue;
    }
    out.push(node.val);
    q.push(node.left);
    q.push(node.right);
  }
  while (out.length && out[out.length - 1] === null) out.pop();
  return out;
}
function graphFromAdj(adj) {
  if (!adj || adj.length === 0) return null;
  const nodes = adj.map((_, i) => new GraphNode(i + 1));
  adj.forEach((nbrs, i) => {
    nodes[i].neighbors = nbrs.map((j) => nodes[j - 1]);
  });
  return nodes[0];
}
function graphToAdj(node) {
  if (!node) return [];
  const seen = new Map();
  const q = [node];
  seen.set(node.val, node);
  while (q.length) {
    const cur = q.shift();
    for (const n of cur.neighbors) {
      if (!seen.has(n.val)) {
        seen.set(n.val, n);
        q.push(n);
      }
    }
  }
  const n = Math.max(...seen.keys());
  const adj = Array.from({ length: n }, () => []);
  for (const [v, nd] of seen) adj[v - 1] = nd.neighbors.map((x) => x.val);
  return adj;
}
function encode(value, kind) {
  if (kind === "list") return listToArray(value);
  if (kind === "listlist") return (value || []).map(listToArray);
  if (kind === "tree") return treeToArray(value);
  if (kind === "graph") return graphToAdj(value);
  return value;
}
function decode(value, kind, extra) {
  extra = extra || {};
  if (kind === "list") return listFromArray(value, extra.pos != null ? extra.pos : -1);
  if (kind === "listlist") return (value || []).map((x) => listFromArray(x));
  if (kind === "tree") return treeFromArray(value);
  if (kind === "graph") return graphFromAdj(value);
  return value;
}
function deepEqual(a, b, compare) {
  if (compare === "sorted" && Array.isArray(a) && Array.isArray(b)) {
    const sa = [...a].sort((x, y) => (x > y ? 1 : x < y ? -1 : 0));
    const sb = [...b].sort((x, y) => (x > y ? 1 : x < y ? -1 : 0));
    return JSON.stringify(sa) === JSON.stringify(sb);
  }
  if (compare === "set" && Array.isArray(a) && Array.isArray(b)) {
    const norm = (x) => (Array.isArray(x) ? JSON.stringify([...x].sort()) : JSON.stringify(x));
    return JSON.stringify([...a].map(norm).sort()) === JSON.stringify([...b].map(norm).sort());
  }
  return JSON.stringify(a) === JSON.stringify(b);
}

function formatLogArg(a) {
  if (typeof a === "string") return a;
  if (typeof a === "undefined") return "undefined";
  try {
    return JSON.stringify(a);
  } catch {
    return String(a);
  }
}

const realConsole = console;
const sandbox = {
  ListNode,
  TreeNode,
  Node: GraphNode,
  GraphNode,
  module: { exports: {} },
  exports: {},
  require,
};
sandbox.global = sandbox;

function attachConsole(buf) {
  const write = (level, args) => {
    buf.push(args.map(formatLogArg).join(" "));
    try {
      realConsole[level](...args);
    } catch (_) {}
  };
  sandbox.console = {
    log: (...a) => write("log", a),
    info: (...a) => write("info", a),
    warn: (...a) => write("warn", a),
    error: (...a) => write("error", a),
    debug: (...a) => write("debug", a),
  };
}

const userCode = fs.readFileSync(solPath, "utf8");
const loadLogs = [];
attachConsole(loadLogs);
try {
  vm.createContext(sandbox);
  vm.runInContext(userCode, sandbox, { filename: "solution.js", timeout: 4000 });
} catch (e) {
  fs.writeFileSync(resultPath, JSON.stringify({ compileError: String(e && e.stack ? e.stack : e) }));
  process.exit(0);
}

const outKind = helpers[0] || "json";
const inKinds = helpers.slice(1).filter((h) => h !== "first_bad_version");
const cases = [];

function getFn() {
  if (typeof sandbox[entry] === "function") return sandbox[entry];
  if (typeof sandbox.Solution === "function") {
    const s = new sandbox.Solution();
    if (typeof s[entry] === "function") return s[entry].bind(s);
  }
  throw new Error("Could not find " + entry + " in your code");
}

for (let i = 0; i < tests.length; i++) {
  const t = tests[i];
  const expected = t.expected;
  const compare = t.compare || "exact";
  let actual = null;
  let passed = false;
  let error = null;
  const logs = [];
  attachConsole(logs);
  try {
    if (mode === "class") {
      const ops = t.ops;
      actual = [];
      let obj = null;
      for (const step of ops) {
        const name = step[0];
        const args = step.slice(1);
        if (!obj) {
          const Cls = sandbox[name];
          obj = new Cls(...args);
          actual.push(null);
        } else {
          actual.push(obj[name](...args));
        }
      }
      passed = deepEqual(actual, expected, compare);
    } else {
      const argsObj = t.args || {};
      const extra = t.extra || {};
      const callArgs = params.map((name, idx) => {
        const kind = inKinds[idx] || "json";
        return decode(argsObj[name], kind, extra);
      });
      if (helpers.includes("first_bad_version")) {
        const bad = extra.bad ?? argsObj.bad;
        sandbox.isBadVersion = (version) => version >= bad;
      }
      const fn = getFn();
      const raw = fn(...callArgs);
      if (raw === undefined) {
        actual = null;
        error = "Your function did not return a value. Add a return statement.";
        passed = false;
      } else {
        actual = encode(raw, outKind);
        if (actual === undefined) actual = null;
        passed = deepEqual(actual, expected, compare);
      }
    }
  } catch (e) {
    error = String(e && e.stack ? e.stack : e);
    passed = false;
    if (actual === undefined) actual = null;
  }
  const stdoutLines = i === 0 && loadLogs.length ? loadLogs.concat(logs) : logs;
  cases.push({
    index: i,
    passed,
    expected: expected === undefined ? null : expected,
    actual: actual === undefined ? null : actual,
    error,
    stdout: stdoutLines.join("\n"),
  });
}

fs.writeFileSync(
  resultPath,
  JSON.stringify({ cases }, (_key, value) => (value === undefined ? null : value)),
);
