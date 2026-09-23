#include <algorithm>
#include <cctype>
#include <cmath>
#include <cstdio>
#include <cstdint>
#include <deque>
#include <fstream>
#include <iostream>
#include <list>
#include <map>
#include <memory>
#include <optional>
#include <queue>
#include <sstream>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <unordered_set>
#include <utility>
#include <vector>

using namespace std;

struct ListNode {
  int val;
  ListNode* next;
  ListNode() : val(0), next(nullptr) {}
  ListNode(int x) : val(x), next(nullptr) {}
  ListNode(int x, ListNode* next) : val(x), next(next) {}
};

struct TreeNode {
  int val;
  TreeNode* left;
  TreeNode* right;
  TreeNode() : val(0), left(nullptr), right(nullptr) {}
  TreeNode(int x) : val(x), left(nullptr), right(nullptr) {}
  TreeNode(int x, TreeNode* left, TreeNode* right) : val(x), left(left), right(right) {}
};

class Node {
 public:
  int val;
  vector<Node*> neighbors;
  Node() { val = 0; }
  Node(int _val) { val = _val; }
  Node(int _val, vector<Node*> _neighbors) {
    val = _val;
    neighbors = _neighbors;
  }
};

static int g_bad_version = 0;
bool isBadVersion(int version) { return version >= g_bad_version; }

#include "solution.cpp"

// --- minimal JSON ---
enum class JType { Null, Bool, Number, String, Array, Object };

struct Json {
  JType type = JType::Null;
  bool b = false;
  double n = 0;
  string s;
  vector<Json> a;
  map<string, Json> o;

  static Json null() { return Json(); }
  static Json boolean(bool v) {
    Json j;
    j.type = JType::Bool;
    j.b = v;
    return j;
  }
  static Json number(double v) {
    Json j;
    j.type = JType::Number;
    j.n = v;
    return j;
  }
  static Json string_(string v) {
    Json j;
    j.type = JType::String;
    j.s = std::move(v);
    return j;
  }
  static Json array(vector<Json> v = {}) {
    Json j;
    j.type = JType::Array;
    j.a = std::move(v);
    return j;
  }
  static Json object(map<string, Json> v = {}) {
    Json j;
    j.type = JType::Object;
    j.o = std::move(v);
    return j;
  }

  bool is_null() const { return type == JType::Null; }
  bool is_bool() const { return type == JType::Bool; }
  bool is_number() const { return type == JType::Number; }
  bool is_string() const { return type == JType::String; }
  bool is_array() const { return type == JType::Array; }
  bool is_object() const { return type == JType::Object; }

  bool as_bool() const {
    if (type != JType::Bool) throw runtime_error("expected bool");
    return b;
  }
  int as_int() const {
    if (type != JType::Number) throw runtime_error("expected number");
    return static_cast<int>(llround(n));
  }
  long long as_ll() const {
    if (type != JType::Number) throw runtime_error("expected number");
    return llround(n);
  }
  double as_double() const {
    if (type != JType::Number) throw runtime_error("expected number");
    return n;
  }
  const string& as_string() const {
    if (type != JType::String) throw runtime_error("expected string");
    return s;
  }
  const vector<Json>& as_array() const {
    if (type != JType::Array) throw runtime_error("expected array");
    return a;
  }
  const map<string, Json>& as_object() const {
    if (type != JType::Object) throw runtime_error("expected object");
    return o;
  }

  bool has(const string& key) const { return type == JType::Object && o.count(key); }
  const Json& at(const string& key) const {
    auto it = o.find(key);
    if (it == o.end()) throw runtime_error("missing key: " + key);
    return it->second;
  }
  const Json& operator[](const string& key) const { return at(key); }
};

class JsonParser {
  const string& in;
  size_t i = 0;

 public:
  explicit JsonParser(const string& s) : in(s) {}

  Json parse() {
    skip();
    Json v = parse_value();
    skip();
    if (i != in.size()) throw runtime_error("trailing junk in json");
    return v;
  }

 private:
  void skip() {
    while (i < in.size() && isspace(static_cast<unsigned char>(in[i]))) i++;
  }
  char peek() {
    if (i >= in.size()) throw runtime_error("unexpected end of json");
    return in[i];
  }
  char get() {
    char c = peek();
    i++;
    return c;
  }
  bool match(char c) {
    skip();
    if (i < in.size() && in[i] == c) {
      i++;
      return true;
    }
    return false;
  }

  Json parse_value() {
    skip();
    char c = peek();
    if (c == 'n') return parse_null();
    if (c == 't' || c == 'f') return parse_bool();
    if (c == '"') return parse_string();
    if (c == '[') return parse_array();
    if (c == '{') return parse_object();
    if (c == '-' || isdigit(static_cast<unsigned char>(c))) return parse_number();
    throw runtime_error(string("unexpected json char: ") + c);
  }

  Json parse_null() {
    if (in.compare(i, 4, "null") != 0) throw runtime_error("invalid null");
    i += 4;
    return Json::null();
  }
  Json parse_bool() {
    if (in.compare(i, 4, "true") == 0) {
      i += 4;
      return Json::boolean(true);
    }
    if (in.compare(i, 5, "false") == 0) {
      i += 5;
      return Json::boolean(false);
    }
    throw runtime_error("invalid bool");
  }
  Json parse_number() {
    size_t start = i;
    if (in[i] == '-') i++;
    if (i >= in.size() || !isdigit(static_cast<unsigned char>(in[i])))
      throw runtime_error("invalid number");
    while (i < in.size() && isdigit(static_cast<unsigned char>(in[i]))) i++;
    if (i < in.size() && in[i] == '.') {
      i++;
      while (i < in.size() && isdigit(static_cast<unsigned char>(in[i]))) i++;
    }
    if (i < in.size() && (in[i] == 'e' || in[i] == 'E')) {
      i++;
      if (i < in.size() && (in[i] == '+' || in[i] == '-')) i++;
      while (i < in.size() && isdigit(static_cast<unsigned char>(in[i]))) i++;
    }
    return Json::number(stod(in.substr(start, i - start)));
  }
  Json parse_string() {
    if (get() != '"') throw runtime_error("expected string");
    string out;
    while (true) {
      if (i >= in.size()) throw runtime_error("unterminated string");
      char c = get();
      if (c == '"') break;
      if (c == '\\') {
        char e = get();
        switch (e) {
          case '"':
          case '\\':
          case '/':
            out.push_back(e);
            break;
          case 'b':
            out.push_back('\b');
            break;
          case 'f':
            out.push_back('\f');
            break;
          case 'n':
            out.push_back('\n');
            break;
          case 'r':
            out.push_back('\r');
            break;
          case 't':
            out.push_back('\t');
            break;
          case 'u': {
            if (i + 4 > in.size()) throw runtime_error("bad unicode escape");
            unsigned code = 0;
            for (int k = 0; k < 4; k++) {
              char h = get();
              code <<= 4;
              if (h >= '0' && h <= '9') code |= h - '0';
              else if (h >= 'a' && h <= 'f') code |= h - 'a' + 10;
              else if (h >= 'A' && h <= 'F') code |= h - 'A' + 10;
              else throw runtime_error("bad unicode escape");
            }
            if (code <= 0x7F) out.push_back(static_cast<char>(code));
            else if (code <= 0x7FF) {
              out.push_back(static_cast<char>(0xC0 | (code >> 6)));
              out.push_back(static_cast<char>(0x80 | (code & 0x3F)));
            } else {
              out.push_back(static_cast<char>(0xE0 | (code >> 12)));
              out.push_back(static_cast<char>(0x80 | ((code >> 6) & 0x3F)));
              out.push_back(static_cast<char>(0x80 | (code & 0x3F)));
            }
            break;
          }
          default:
            throw runtime_error("bad escape");
        }
      } else {
        out.push_back(c);
      }
    }
    return Json::string_(out);
  }
  Json parse_array() {
    if (get() != '[') throw runtime_error("expected array");
    vector<Json> items;
    skip();
    if (match(']')) return Json::array(items);
    while (true) {
      items.push_back(parse_value());
      skip();
      if (match(']')) break;
      if (!match(',')) throw runtime_error("expected comma in array");
    }
    return Json::array(items);
  }
  Json parse_object() {
    if (get() != '{') throw runtime_error("expected object");
    map<string, Json> obj;
    skip();
    if (match('}')) return Json::object(obj);
    while (true) {
      skip();
      Json key = parse_string();
      skip();
      if (!match(':')) throw runtime_error("expected colon");
      obj[key.as_string()] = parse_value();
      skip();
      if (match('}')) break;
      if (!match(',')) throw runtime_error("expected comma in object");
    }
    return Json::object(obj);
  }
};

static string json_escape(const string& s) {
  string out;
  out.reserve(s.size() + 8);
  for (unsigned char c : s) {
    switch (c) {
      case '"':
        out += "\\\"";
        break;
      case '\\':
        out += "\\\\";
        break;
      case '\b':
        out += "\\b";
        break;
      case '\f':
        out += "\\f";
        break;
      case '\n':
        out += "\\n";
        break;
      case '\r':
        out += "\\r";
        break;
      case '\t':
        out += "\\t";
        break;
      default:
        if (c < 0x20) {
          char buf[8];
          snprintf(buf, sizeof(buf), "\\u%04x", c);
          out += buf;
        } else {
          out.push_back(static_cast<char>(c));
        }
    }
  }
  return out;
}

static string dump_json(const Json& j) {
  switch (j.type) {
    case JType::Null:
      return "null";
    case JType::Bool:
      return j.b ? "true" : "false";
    case JType::Number: {
      if (isfinite(j.n) && floor(j.n) == j.n && fabs(j.n) < 1e15) {
        ostringstream oss;
        oss << static_cast<long long>(j.n);
        return oss.str();
      }
      ostringstream oss;
      oss.precision(15);
      oss << j.n;
      return oss.str();
    }
    case JType::String:
      return "\"" + json_escape(j.s) + "\"";
    case JType::Array: {
      string out = "[";
      for (size_t i = 0; i < j.a.size(); i++) {
        if (i) out += ",";
        out += dump_json(j.a[i]);
      }
      out += "]";
      return out;
    }
    case JType::Object: {
      string out = "{";
      bool first = true;
      for (const auto& kv : j.o) {
        if (!first) out += ",";
        first = false;
        out += "\"" + json_escape(kv.first) + "\":" + dump_json(kv.second);
      }
      out += "}";
      return out;
    }
  }
  return "null";
}

static string read_file(const string& path) {
  ifstream in(path);
  if (!in) throw runtime_error("cannot read " + path);
  ostringstream ss;
  ss << in.rdbuf();
  return ss.str();
}

static void write_file(const string& path, const string& data) {
  ofstream out(path);
  if (!out) throw runtime_error("cannot write " + path);
  out << data;
}

static vector<int> as_vector_int(const Json& j) {
  vector<int> out;
  for (const auto& x : j.as_array()) out.push_back(x.as_int());
  return out;
}
static vector<string> as_vector_string(const Json& j) {
  vector<string> out;
  for (const auto& x : j.as_array()) out.push_back(x.as_string());
  return out;
}
static vector<vector<int>> as_vector_vector_int(const Json& j) {
  vector<vector<int>> out;
  for (const auto& row : j.as_array()) out.push_back(as_vector_int(row));
  return out;
}
static vector<vector<char>> as_vector_vector_char(const Json& j) {
  vector<vector<char>> out;
  for (const auto& row : j.as_array()) {
    vector<char> r;
    for (const auto& cell : row.as_array()) {
      if (cell.is_string()) {
        const string& s = cell.as_string();
        r.push_back(s.empty() ? '\0' : s[0]);
      } else {
        throw runtime_error("expected char string in board/grid");
      }
    }
    out.push_back(std::move(r));
  }
  return out;
}

static Json to_json(int v) { return Json::number(v); }
static Json to_json(bool v) { return Json::boolean(v); }
static Json to_json(const string& v) { return Json::string_(v); }
static Json to_json(const vector<int>& v) {
  vector<Json> a;
  for (int x : v) a.push_back(Json::number(x));
  return Json::array(a);
}
static Json to_json(const vector<string>& v) {
  vector<Json> a;
  for (const auto& x : v) a.push_back(Json::string_(x));
  return Json::array(a);
}
static Json to_json(const vector<vector<int>>& v) {
  vector<Json> a;
  for (const auto& row : v) a.push_back(to_json(row));
  return Json::array(a);
}
static Json to_json(const vector<vector<string>>& v) {
  vector<Json> a;
  for (const auto& row : v) a.push_back(to_json(row));
  return Json::array(a);
}
static Json to_json(const vector<vector<char>>& v) {
  vector<Json> a;
  for (const auto& row : v) {
    vector<Json> r;
    for (char c : row) r.push_back(Json::string_(string(1, c)));
    a.push_back(Json::array(r));
  }
  return Json::array(a);
}

static ListNode* decode_list(const Json& value, const Json& extra) {
  if (value.is_null()) return nullptr;
  const auto& arr = value.as_array();
  if (arr.empty()) return nullptr;
  vector<ListNode*> nodes;
  nodes.reserve(arr.size());
  for (const auto& x : arr) nodes.push_back(new ListNode(x.as_int()));
  for (size_t i = 0; i + 1 < nodes.size(); i++) nodes[i]->next = nodes[i + 1];
  int pos = -1;
  if (extra.has("pos") && !extra["pos"].is_null()) pos = extra["pos"].as_int();
  if (pos >= 0 && pos < static_cast<int>(nodes.size())) {
    nodes.back()->next = nodes[pos];
  }
  return nodes[0];
}

static Json encode_list(ListNode* node, int limit = 10000) {
  vector<Json> out;
  unordered_set<ListNode*> seen;
  while (node && static_cast<int>(out.size()) < limit) {
    if (seen.count(node)) break;
    seen.insert(node);
    out.push_back(Json::number(node->val));
    node = node->next;
  }
  return Json::array(out);
}

static vector<ListNode*> decode_listlist(const Json& value) {
  vector<ListNode*> out;
  if (value.is_null()) return out;
  Json empty = Json::object();
  for (const auto& item : value.as_array()) out.push_back(decode_list(item, empty));
  return out;
}

static TreeNode* decode_tree(const Json& value) {
  if (value.is_null()) return nullptr;
  const auto& arr = value.as_array();
  if (arr.empty()) return nullptr;
  if (arr[0].is_null()) return nullptr;
  TreeNode* root = new TreeNode(arr[0].as_int());
  deque<TreeNode*> q;
  q.push_back(root);
  size_t i = 1;
  while (!q.empty() && i < arr.size()) {
    TreeNode* node = q.front();
    q.pop_front();
    if (i < arr.size()) {
      if (!arr[i].is_null()) {
        node->left = new TreeNode(arr[i].as_int());
        q.push_back(node->left);
      }
      i++;
    }
    if (i < arr.size()) {
      if (!arr[i].is_null()) {
        node->right = new TreeNode(arr[i].as_int());
        q.push_back(node->right);
      }
      i++;
    }
  }
  return root;
}

static Json encode_tree(TreeNode* root) {
  if (!root) return Json::array();
  vector<Json> out;
  deque<TreeNode*> q;
  q.push_back(root);
  while (!q.empty()) {
    TreeNode* node = q.front();
    q.pop_front();
    if (!node) {
      out.push_back(Json::null());
      continue;
    }
    out.push_back(Json::number(node->val));
    q.push_back(node->left);
    q.push_back(node->right);
  }
  while (!out.empty() && out.back().is_null()) out.pop_back();
  return Json::array(out);
}

static Node* decode_graph(const Json& value) {
  if (value.is_null()) return nullptr;
  const auto& adj = value.as_array();
  if (adj.empty()) return nullptr;
  vector<Node*> nodes;
  nodes.reserve(adj.size());
  for (size_t i = 0; i < adj.size(); i++) nodes.push_back(new Node(static_cast<int>(i + 1)));
  for (size_t i = 0; i < adj.size(); i++) {
    for (const auto& nbr : adj[i].as_array()) {
      nodes[i]->neighbors.push_back(nodes[nbr.as_int() - 1]);
    }
  }
  return nodes[0];
}

static Json encode_graph(Node* node) {
  if (!node) return Json::array();
  unordered_map<int, Node*> seen;
  deque<Node*> q;
  q.push_back(node);
  seen[node->val] = node;
  while (!q.empty()) {
    Node* cur = q.front();
    q.pop_front();
    for (Node* n : cur->neighbors) {
      if (!seen.count(n->val)) {
        seen[n->val] = n;
        q.push_back(n);
      }
    }
  }
  int n = 0;
  for (const auto& kv : seen) n = max(n, kv.first);
  vector<Json> adj(n, Json::array());
  for (const auto& kv : seen) {
    vector<Json> nbrs;
    for (Node* x : kv.second->neighbors) nbrs.push_back(Json::number(x->val));
    adj[kv.first - 1] = Json::array(nbrs);
  }
  return Json::array(adj);
}

static Json encode_by_kind(const Json& value, const string&) { return value; }
static Json encode_by_kind(ListNode* value, const string& kind) {
  if (kind == "list") return encode_list(value);
  return encode_list(value);
}
static Json encode_by_kind(TreeNode* value, const string& kind) {
  if (kind == "tree") return encode_tree(value);
  return encode_tree(value);
}
static Json encode_by_kind(Node* value, const string& kind) {
  if (kind == "graph") return encode_graph(value);
  return encode_graph(value);
}
template <typename T>
static Json encode_by_kind(const T& value, const string&) {
  return to_json(value);
}

static int cmp_json_scalar(const Json& a, const Json& b) {
  if (a.is_number() && b.is_number()) {
    if (a.n < b.n) return -1;
    if (a.n > b.n) return 1;
    return 0;
  }
  if (a.is_string() && b.is_string()) {
    if (a.s < b.s) return -1;
    if (a.s > b.s) return 1;
    return 0;
  }
  string sa = dump_json(a), sb = dump_json(b);
  if (sa < sb) return -1;
  if (sa > sb) return 1;
  return 0;
}

static bool deep_equal(const Json& a, const Json& b, const string& compare);

static bool deep_equal(const Json& a, const Json& b, const string& compare) {
  if (compare == "sorted" && a.is_array() && b.is_array()) {
    auto sa = a.a, sb = b.a;
    sort(sa.begin(), sa.end(), [](const Json& x, const Json& y) {
      return cmp_json_scalar(x, y) < 0;
    });
    sort(sb.begin(), sb.end(), [](const Json& x, const Json& y) {
      return cmp_json_scalar(x, y) < 0;
    });
    return deep_equal(Json::array(sa), Json::array(sb), "exact");
  }
  if (compare == "set" && a.is_array() && b.is_array()) {
    auto norm = [](Json x) {
      if (x.is_array()) {
        sort(x.a.begin(), x.a.end(), [](const Json& u, const Json& v) {
          return cmp_json_scalar(u, v) < 0;
        });
      }
      return x;
    };
    vector<Json> sa, sb;
    for (const auto& x : a.a) sa.push_back(norm(x));
    for (const auto& x : b.a) sb.push_back(norm(x));
    sort(sa.begin(), sa.end(), [](const Json& x, const Json& y) {
      return cmp_json_scalar(x, y) < 0;
    });
    sort(sb.begin(), sb.end(), [](const Json& x, const Json& y) {
      return cmp_json_scalar(x, y) < 0;
    });
    return deep_equal(Json::array(sa), Json::array(sb), "exact");
  }
  if (a.is_number() && b.is_number()) return fabs(a.n - b.n) < 1e-9;
  if (a.is_bool() && b.is_bool()) return a.b == b.b;
  if (a.is_string() && b.is_string()) return a.s == b.s;
  if (a.is_null() && b.is_null()) return true;
  if (a.is_array() && b.is_array()) {
    if (a.a.size() != b.a.size()) return false;
    for (size_t i = 0; i < a.a.size(); i++) {
      if (!deep_equal(a.a[i], b.a[i], "exact")) return false;
    }
    return true;
  }
  if (a.is_object() && b.is_object()) {
    if (a.o.size() != b.o.size()) return false;
    for (const auto& kv : a.o) {
      auto it = b.o.find(kv.first);
      if (it == b.o.end() || !deep_equal(kv.second, it->second, "exact")) return false;
    }
    return true;
  }
  return false;
}

#include "invoke.inc"

int main(int argc, char** argv) {
  if (argc < 8) {
    cerr << "usage: harness solution tests results entry mode helpers params\n";
    return 2;
  }
  string sol_path = argv[1];
  string tests_path = argv[2];
  string result_path = argv[3];
  string entry = argv[4];
  string mode = argv[5];
  string helpers_json = argv[6];
  string params_json = argv[7];
  (void)sol_path;

  try {
    Json helpers = JsonParser(helpers_json).parse();
    Json params = JsonParser(params_json).parse();
    Json tests = JsonParser(read_file(tests_path)).parse();
    string out_kind = "json";
    vector<string> in_kinds;
    const auto& harr = helpers.as_array();
    if (!harr.empty()) out_kind = harr[0].as_string();
    for (size_t i = 1; i < harr.size(); i++) {
      string h = harr[i].as_string();
      if (h != "first_bad_version") in_kinds.push_back(h);
    }
    bool first_bad = false;
    for (const auto& h : harr) {
      if (h.is_string() && h.as_string() == "first_bad_version") first_bad = true;
    }

    vector<Json> cases;
    const auto& tarr = tests.as_array();
    for (size_t i = 0; i < tarr.size(); i++) {
      const Json& t = tarr[i];
      Json expected = t.has("expected") ? t["expected"] : Json::null();
      string compare = t.has("compare") && t["compare"].is_string() ? t["compare"].as_string()
                                                                    : "exact";
      Json actual = Json::null();
      bool passed = false;
      string error;
      string captured;
      try {
        if (mode == "class") {
          if (!t.has("ops")) throw runtime_error("missing ops");
          actual = run_class_case(entry, t["ops"]);
          passed = deep_equal(actual, expected, compare);
        } else {
          Json args = t.has("args") ? t["args"] : Json::object();
          Json extra = t.has("extra") ? t["extra"] : Json::object();
          if (first_bad) {
            if (extra.has("bad")) g_bad_version = extra["bad"].as_int();
            else if (args.has("bad")) g_bad_version = args["bad"].as_int();
          }
          actual = run_function_case(args, extra, params, in_kinds, out_kind);
          passed = deep_equal(actual, expected, compare);
        }
      } catch (const exception& e) {
        error = e.what();
        actual = Json::null();
        passed = false;
      }
      map<string, Json> one;
      one["index"] = Json::number(static_cast<double>(i));
      one["passed"] = Json::boolean(passed);
      one["expected"] = expected;
      one["actual"] = actual;
      if (error.empty()) one["error"] = Json::null();
      else one["error"] = Json::string_(error);
      one["stdout"] = Json::string_(captured);
      cases.push_back(Json::object(one));
    }
    map<string, Json> payload;
    payload["cases"] = Json::array(cases);
    write_file(result_path, dump_json(Json::object(payload)));
  } catch (const exception& e) {
    map<string, Json> payload;
    payload["compileError"] = Json::string_(e.what());
    write_file(result_path, dump_json(Json::object(payload)));
  }
  return 0;
}
