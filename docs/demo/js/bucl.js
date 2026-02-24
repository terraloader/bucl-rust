// BUCL interpreter — pure JavaScript port of the Rust implementation

// ── Lexer ──────────────────────────────────────────────────────────────────

function tokenizeLine(line) {
  const indent = line.length - line.trimStart().length;
  const content = line.trim();
  if (!content || content.startsWith('#')) return null;
  const tokens = [];
  let i = 0;
  while (i < content.length) {
    const c = content[i];
    if (c === ' ' || c === '\t') { i++; continue; }
    if (c === '{') {
      i++;
      let name = '', depth = 1;
      while (i < content.length) {
        if (content[i] === '{') { depth++; name += '{'; i++; }
        else if (content[i] === '}') { if (--depth === 0) { i++; break; } name += '}'; i++; }
        else name += content[i++];
      }
      tokens.push({ type: 'variable', value: name });
    } else if (c === '"') {
      i++;
      let s = '';
      while (i < content.length && content[i] !== '"') {
        if (content[i] === '\\' && i + 1 < content.length) {
          const nc = content[++i];
          s += nc === '"' ? '"' : nc === 'n' ? '\n' : nc === 't' ? '\t' : nc === '\\' ? '\\' : ('\\' + nc);
        } else s += content[i];
        i++;
      }
      if (i < content.length) i++;
      tokens.push({ type: 'quoted', value: s });
    } else {
      let word = '';
      while (i < content.length && content[i] !== ' ' && content[i] !== '\t') word += content[i++];
      tokens.push({ type: 'bare', value: word });
    }
  }
  return tokens.length > 0 ? { indent, tokens } : null;
}

function tokenize(src) { return src.split('\n').map(tokenizeLine).filter(Boolean); }

// ── Parser ─────────────────────────────────────────────────────────────────

function parse(src) {
  const p = { lines: tokenize(src), cursor: 0 };
  return parseBlock(p, 0);
}

function isCont(p, i) {
  const l = p.lines[i];
  if (!l) return false;
  const f = l.tokens[0];
  return f && f.type === 'bare' && (f.value === 'elseif' || f.value === 'else');
}

function parseBlock(p, ei) {
  const stmts = [];
  while (p.cursor < p.lines.length) {
    const ind = p.lines[p.cursor].indent;
    if (ind < ei) break;
    if (ind > ei) throw new Error('unexpected indent: got ' + ind + ', expected ' + ei);
    if (isCont(p, p.cursor)) break;
    stmts.push(parseStmt(p, ei));
  }
  return stmts;
}

function parseStmt(p, ci) {
  const line = p.lines[p.cursor++];
  const parts = extractParts(line.tokens);
  let block = null;
  if (p.cursor < p.lines.length && p.lines[p.cursor].indent > ci)
    block = parseBlock(p, p.lines[p.cursor].indent);
  let continuation = null;
  if ((parts.fn === 'if' || parts.fn === 'elseif') && p.cursor < p.lines.length &&
      isCont(p, p.cursor) && p.lines[p.cursor].indent === ci)
    continuation = parseStmt(p, ci);
  return { target: parts.target, fn: parts.fn, args: parts.args, block: block, continuation: continuation };
}

function extractParts(tokens) {
  let i = 0;
  const first = tokens[i++];
  let target = null, fn;
  if (first.type === 'variable') {
    const next = tokens[i++];
    if (!next || next.type !== 'bare') throw new Error('expected function name after {' + first.value + '}');
    target = first.value; fn = next.value;
  } else if (first.type === 'bare') {
    fn = first.value;
  } else throw new Error('line cannot start with a string literal');
  return { target: target, fn: fn, args: tokens.slice(i) };
}

// ── Math expression evaluator ──────────────────────────────────────────────

function mathEval(s) {
  let p = 0;
  function ws() { while (p < s.length && (s[p] === ' ' || s[p] === '\t')) p++; }
  function primary() {
    ws();
    if (s[p] === '(') {
      p++;
      var v = addSub(); ws();
      if (s[p] !== ')') throw new Error('expected )');
      p++; return v;
    }
    var n = '';
    while (p < s.length && (s[p] >= '0' && s[p] <= '9' || s[p] === '.')) n += s[p++];
    if (!n) throw new Error('expected number at position ' + p);
    return parseFloat(n);
  }
  function unary() {
    ws();
    if (s[p] === '-') { p++; return -primary(); }
    if (s[p] === '+') p++;
    return primary();
  }
  function mulDiv() {
    var v = unary();
    while (true) {
      ws();
      if (s[p] === '*') { p++; v *= unary(); }
      else if (s[p] === '/') { p++; var r = unary(); if (r === 0) throw new Error('division by zero'); v /= r; }
      else if (s[p] === '%') { p++; var r = unary(); if (r === 0) throw new Error('modulo by zero'); v %= r; }
      else break;
    }
    return v;
  }
  function addSub() {
    var v = mulDiv();
    while (true) {
      ws();
      if (s[p] === '+') { p++; v += mulDiv(); }
      else if (s[p] === '-') { p++; v -= mulDiv(); }
      else break;
    }
    return v;
  }
  var result = addSub(); ws();
  if (p < s.length) throw new Error('unexpected character \'' + s[p] + '\'');
  return result;
}

// ── Evaluator ──────────────────────────────────────────────────────────────

function BuclEval() {
  this.vars = {};
  this.out  = [];
  this.fns  = {};
}

BuclEval.prototype.setVar = function(name, val) {
  if (name.indexOf('/') === -1) {
    this.vars[name + '/length'] = String(Array.from(val).length);
    this.vars[name + '/count']  = '1';
  }
  this.vars[name] = val;
};

BuclEval.prototype.getVar = function(name) {
  if (name.indexOf('{') !== -1) return this.getVar(this.interp(name));
  if (Object.prototype.hasOwnProperty.call(this.vars, name)) return this.vars[name];
  var sl = name.indexOf('/');
  if (sl !== -1) {
    var par = name.slice(0, sl), idx = name.slice(sl + 1);
    var n = parseInt(idx);
    if (Number.isInteger(n) && n >= 0 && String(n) === idx) {
      var cnt = parseInt(this.vars[par + '/count'] || '0') || 0;
      if (cnt === 1) {
        var ch = Array.from(this.vars[par] || '');
        return n < ch.length ? ch[n] : '';
      }
    }
  }
  return '';
};

BuclEval.prototype.getVarForInterp = function(name) {
  var rn = name.indexOf('{') !== -1 ? this.interp(name) : name;
  if (rn.indexOf('/') === -1) {
    var cnt = parseInt(this.vars[rn + '/count'] || '0') || 0;
    if (cnt > 1) {
      var parts = [];
      for (var i = 0; i < cnt; i++) parts.push(this.vars[rn + '/' + i] || '');
      return parts.join(' ');
    }
  }
  return this.getVar(rn);
};

BuclEval.prototype.interp = function(s) {
  var r = '', i = 0;
  while (i < s.length) {
    if (s[i] !== '{') { r += s[i++]; continue; }
    i++;
    var vn = '', closed = false, d = 1;
    while (i < s.length) {
      if (s[i] === '{') { d++; vn += '{'; i++; }
      else if (s[i] === '}') { if (--d === 0) { closed = true; i++; break; } vn += '}'; i++; }
      else vn += s[i++];
    }
    r += closed ? this.getVarForInterp(vn) : '{' + vn;
  }
  return r;
};

BuclEval.prototype.evalP = function(p) {
  if (p.type === 'quoted')   return this.interp(p.value);
  if (p.type === 'variable') return this.getVar(p.value);
  return p.value;
};

BuclEval.prototype.evalPs = function(params) {
  var r = [];
  for (var pi = 0; pi < params.length; pi++) {
    var p = params[pi];
    if (p.type === 'variable') {
      var rn = p.value.indexOf('{') !== -1 ? this.interp(p.value) : p.value;
      if (rn.indexOf('/') === -1) {
        var cnt = parseInt(this.vars[rn + '/count'] || '0') || 0;
        if (cnt > 1) {
          for (var j = 0; j < cnt; j++) r.push(this.vars[rn + '/' + j] || '');
          continue;
        }
      }
      r.push(this.getVar(p.value));
    } else {
      r.push(this.evalP(p));
    }
  }
  return r;
};

BuclEval.prototype.runStmts = function(stmts) {
  for (var i = 0; i < stmts.length; i++) this.exec(stmts[i]);
};

BuclEval.prototype.exec = function(stmt) {
  var args = this.evalPs(stmt.args);
  var tgt = stmt.target;
  if (tgt && tgt.indexOf('{') !== -1) tgt = this.interp(tgt);
  var res = this.dispatch(stmt.fn, tgt, args, stmt.block, stmt.continuation);
  if (res !== undefined && res !== null && tgt !== null) this.setVar(tgt, res);
};

BuclEval.prototype.dispatch = function(fn, tgt, args, block, cont) {
  switch (fn) {
    case '=':        return this.doAssign(tgt, args);
    case 'echo':     { var v = args.join(' '); this.out.push(v); return undefined; }
    case 'if':
    case 'elseif':   return this.doIf(args, block, cont);
    case 'else':     if (block) this.runStmts(block); return undefined;
    case 'repeat':   return this.doRepeat(tgt, args, block);
    case 'each':     return this.doEach(tgt, args, block);
    case 'math': {
      var v = mathEval(args.join(''));
      return (v % 1 === 0 && Math.abs(v) < 1e15) ? String(Math.trunc(v)) : String(v);
    }
    case 'random': {
      var mn = 0, mx = Number.MAX_SAFE_INTEGER;
      if (args.length === 1) mx = parseInt(args[0]);
      else if (args.length >= 2) { mn = parseInt(args[0]); mx = parseInt(args[1]); }
      return String(Math.floor(Math.random() * (mx - mn + 1)) + mn);
    }
    case 'length':
      return String(args.reduce(function(s, a) { return s + Array.from(a).length; }, 0));
    case 'count':    return String(args.length);
    case 'substr': {
      if (args.length < 3) throw new Error('substr: requires start, length, string');
      var s = parseInt(args[0]), l = parseInt(args[1]), ch = Array.from(args[2]);
      var st = Math.min(s, ch.length);
      return ch.slice(st, Math.min(st + l, ch.length)).join('');
    }
    case 'strpos': {
      if (args.length < 2) throw new Error('strpos: requires text and needle');
      var pos = args[0].indexOf(args[1]);
      return pos < 0 ? '-1' : String(args[0].slice(0, pos).length);
    }
    case 'setvar':
      if (args.length >= 2) this.setVar(args[0], args[1]);
      return undefined;
    case 'getvar':   return args.length ? this.getVar(args[0]) : '';
    case 'readfile': return '[error] readfile: not available in browser';
    case 'writefile':return '[error] writefile: not available in browser';
    default:         return this.callBuclFn(fn, tgt, args);
  }
};

BuclEval.prototype.doAssign = function(tgt, args) {
  var v = args.join('');
  if (!tgt) return v;
  this.setVar(tgt, v);
  if (args.length > 1) {
    this.vars[tgt + '/count'] = String(args.length);
    for (var i = 0; i < args.length; i++) this.vars[tgt + '/' + i] = args[i];
  }
  return undefined;
};

BuclEval.prototype.doIf = function(args, block, cont) {
  var l = args[0], op = args[1], r = args[2];
  var ln = parseFloat(l), rn = parseFloat(r), num = !isNaN(ln) && !isNaN(rn);
  var cond =
    op === '='  ? l === r :
    op === '!=' ? l !== r :
    op === '>'  ? (num ? ln > rn  : l > r)  :
    op === '<'  ? (num ? ln < rn  : l < r)  :
    op === '>=' ? (num ? ln >= rn : l >= r) :
    op === '<=' ? (num ? ln <= rn : l <= r) : false;
  if (cond) { if (block) this.runStmts(block); }
  else if (cont) this.exec(cont);
  return undefined;
};

BuclEval.prototype.doRepeat = function(tgt, args, block) {
  var px = tgt || 'r', n = parseInt(args[0]);
  if (isNaN(n)) throw new Error('repeat: \'' + args[0] + '\' is not a valid count');
  this.setVar(px, String(n));
  this.vars[px + '/count'] = String(n);
  if (block) {
    for (var i = 0; i < n; i++) {
      this.vars[px + '/index'] = String(i + 1);
      this.runStmts(block);
    }
  }
  return undefined;
};

BuclEval.prototype.doEach = function(tgt, args, block) {
  var px = tgt || 'e';
  this.setVar(px, String(args.length));
  this.vars[px + '/count'] = String(args.length);
  this.vars[px + '/length'] = String(args.reduce(function(s, a) { return s + Array.from(a).length; }, 0));
  for (var i = 0; i < args.length; i++) this.vars[px + '/' + i] = args[i];
  if (block) {
    for (var i = 0; i < args.length; i++) {
      this.vars[px + '/index'] = String(i);
      this.vars[px + '/value'] = args[i];
      this.runStmts(block);
    }
  }
  return undefined;
};

BuclEval.prototype.callBuclFn = function(name, tgt, args) {
  var src = this.fns[name];
  if (!src) throw new Error('unknown function: ' + name);
  var stmts = parse(src);
  var child = new BuclEval();
  child.fns = this.fns;
  child.vars['argc'] = String(args.length);
  for (var i = 0; i < args.length; i++) child.vars[String(i)] = args[i];
  child.vars['args'] = args.join('');
  child.vars['args/count'] = String(args.length);
  child.vars['args/length'] = String(args.reduce(function(s, a) { return s + Array.from(a).length; }, 0));
  for (var i = 0; i < args.length; i++) child.vars['args/' + i] = args[i];
  if (tgt) child.vars['target'] = tgt;
  child.runStmts(stmts);
  for (var i = 0; i < child.out.length; i++) this.out.push(child.out[i]);
  var retVal = child.vars['return'];
  if (tgt) {
    // Set primary return value first (auto-sets count=1), then copy
    // sub-vars so that {return/count} etc. can override auto-metadata.
    if (retVal !== undefined) this.setVar(tgt, retVal);
    var keys = Object.keys(child.vars);
    for (var i = 0; i < keys.length; i++) {
      if (keys[i].indexOf('return/') === 0)
        this.vars[tgt + '/' + keys[i].slice(7)] = child.vars[keys[i]];
    }
    return undefined;
  }
  return retVal;
};

// ── Standard library (embedded BUCL source) ───────────────────────────────

var BUCL_STDLIB = {
  reverse: [
    '{_text} = ""',
    '{r} repeat {argc}',
    '\t{_i} math "{r/index}-1"',
    '\t{_arg} getvar {_i}',
    '\t{_text} = "{_text}{_arg}"',
    '{_len} length {_text}',
    '{_result} = ""',
    '{r} repeat {_len}',
    '\t{_i} math "{_len}-{r/index}"',
    '\t{_char} substr {_i} 1 {_text}',
    '\t{_result} = "{_result}{_char}"',
    '{return} = {_result}'
  ].join('\n'),

  explode: [
    '{_sep} = {0}',
    '{_text} = {1}',
    '{_sep_len} length {_sep}',
    '{_text_len} length {_text}',
    '{_count} = "0"',
    '{_done} = "0"',
    '{_remaining} = {1}',
    '{_concat} = ""',
    '{_iters} math "{_text_len}+1"',
    '{r} repeat {_iters}',
    '\tif {_done} = "0"',
    '\t\t{_pos} strpos {_remaining} {_sep}',
    '\t\tif {_pos} > "-1"',
    '\t\t\t{_part} substr 0 {_pos} {_remaining}',
    '\t\t\t{return/{_count}} = {_part}',
    '\t\t\t{_concat} = "{_concat}{_part}"',
    '\t\t\t{_count} math "{_count}+1"',
    '\t\t\t{_after} math "{_pos}+{_sep_len}"',
    '\t\t\t{_rem_len} length {_remaining}',
    '\t\t\t{_remaining} substr {_after} {_rem_len} {_remaining}',
    '\t\telse',
    '\t\t\t{return/{_count}} = {_remaining}',
    '\t\t\t{_concat} = "{_concat}{_remaining}"',
    '\t\t\t{_count} math "{_count}+1"',
    '\t\t\t{_done} = "1"',
    '{return} = {_concat}',
    '{return/count} = {_count}'
  ].join('\n'),

  implode: [
    '{_sep} = {args/0}',
    '{_n_items} math "{argc}-1"',
    'if {_n_items} = "0"',
    '\t{return} = ""',
    'else',
    '\t{_result} = {args/1}',
    '\t{_remaining} math "{_n_items}-1"',
    '\t{r} repeat {_remaining}',
    '\t\t{_i} math "{r/index}+1"',
    '\t\t{_result} = "{_result}{_sep}{args/{_i}}"',
    '\t{return} = {_result}'
  ].join('\n'),

  maxlength: [
    '{_max} = "0"',
    '{r} repeat {argc}',
    '\t{_i} math "{r/index}-1"',
    '\t{_item} getvar {_i}',
    '\t{_len} length {_item}',
    '\tif {_len} > {_max}',
    '\t\t{_max} = {_len}',
    '{return} = {_max}'
  ].join('\n'),

  slice: [
    '{_start} = {0}',
    '{_end} = {1}',
    '{_item_count} math "{argc}-2"',
    'if {_start} < "0"',
    '\t{_start} math "{_item_count}+{_start}"',
    'if {_end} < "0"',
    '\t{_end} math "{_item_count}+{_end}"',
    '{_result} = ""',
    '{r} repeat {_item_count}',
    '\t{_i} math "{r/index}-1"',
    '\t{_actual} math "{_i}+2"',
    '\tif {_i} >= {_start}',
    '\t\tif {_i} < {_end}',
    '\t\t\t{_item} getvar {_actual}',
    '\t\t\tif {_result} = ""',
    '\t\t\t\t{_result} = {_item}',
    '\t\t\telse',
    '\t\t\t\t{_result} = "{_result} {_item}"',
    '{return} = {_result}'
  ].join('\n')
};

// ── Public entry point ─────────────────────────────────────────────────────

function buclRun(source) {
  try {
    var ev = new BuclEval();
    var keys = Object.keys(BUCL_STDLIB);
    for (var i = 0; i < keys.length; i++) ev.fns[keys[i]] = BUCL_STDLIB[keys[i]];
    ev.runStmts(parse(source));
    return ev.out.join('\n');
  } catch (e) {
    return '[error] ' + e.message;
  }
}
