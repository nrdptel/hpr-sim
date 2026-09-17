// ThrustCurve.org's own statistics for every curve bundled with hpr-motor.
//
// What this pins: the total impulse, NFPA 1125 burn time (between the 5%-of-peak crossings),
// average thrust (total impulse over that burn time), peak thrust and burn window that
// ThrustCurve.org computes for each bundled .eng/.rse file, using the site's own code:
// simulate/analyze/analyze.js from github.com/JohnCoker/thrustcurve3 at commit
// 577afa62302f70c6b2ba04e97a39240638cd704b (ISC license, copyright John Coker). The file is
// fetched into refs/sources/thrustcurve3/analyze.js (see the `thrustcurve3-analyze` entry in
// validation/refs.lock.toml); this script refuses to run if its sha256 differs.
//
// How ThrustCurve calls it: routes/simfiles.js at the same commit parses a simfile with
// `parsers.parseData(format, data, errs)` and then calls `analyze.stats(parsed, errs)`, where
// `errs` is an error-collector function. With a function as the second argument, `stats` uses
// the default `StdParams` (5% burn cutoff, 50 us time merge, 500 uN leading-zero threshold).
// Before integrating, `stats` normalizes the points: it drops invalid points, sorts by time,
// drops leading points with thrust below 500 uN and merges points closer than 50 us (averaging
// their thrust). This script calls `stats({points}, error)` the same way, and separately calls
// `normalize` with the same defaults to report whether that cleaning changed a curve.
//
// Loading: analyze.js does `require('../../lib/errors')`. The real module
// (lib/errors/errors.js) depends on the npm package `strformat`, so it is not loaded. analyze.js
// only passes its numeric codes to the error callback, which cannot change any statistic, so
// they are supplied in-process with the values from lib/errors/errors.js at the same commit
// (INVALID_INFO 104, INVALID_POINTS 105, DUPLICATE_POINTS 106). The verified bytes are compiled
// with vm.compileFunction and a require that serves only that module, so the pinned file is run
// unmodified and nothing else is resolved.
//
// Point extraction is a minimal reader that follows ThrustCurve's parsers
// (simulate/parsers/rasp.js and rocksim.js):
// - .eng (RASP): lines are trimmed and split on whitespace; leading blank and `;` lines are
//   skipped, the first other line is the seven-field header, and the following two-field lines
//   are (time s, thrust N) points. Anything but blank or `;` lines after the data block (a
//   second motor, a malformed line) is an error.
// - .rse (RockSim): the `t` and `f` attributes of each `<eng-data .../>` element; the file must
//   hold exactly one `<engine ...>` element.
// Numbers must match lib/number's isNumber syntax (plain decimals, no exponent) and be
// non-negative, which is what those parsers accept; each file's sha256 must match catalog.json.
//
// Output: JSON on stdout, 2-space indent, trailing newline, one entry per curve in catalog order.
// Field mapping from the `stats` result: points <- pointCount, total_impulse_ns <- totalImpulse,
// burn_time_s <- burnTime, average_thrust_n <- avgThrust, max_thrust_n <- maxThrust,
// burn_start_s <- burnStart, burn_end_s <- burnEnd. raw_points is the count read from the file;
// merged_or_dropped is true when normalization changed the points. Numbers are printed as
// JavaScript's shortest round-trip form. The output depends only on the inputs and the Node
// version (recorded in "node"), so two runs are byte-identical.
//
// Run from the repository root (Node.js only, no npm packages):
//
//     node validation/oracles/thrustcurve/analyze_stats.js > validation/fixtures/motor/thrustcurve-analyze-stats.json

'use strict';

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const ANALYZE_PATH = path.join(ROOT, 'refs', 'sources', 'thrustcurve3', 'analyze.js');
const ANALYZE_SHA256 = '267eb5364a7ca13b4ec32246c97ee0194a4ee6fd5489765ff178fddf666c637a';
const DATA_DIR = path.join(ROOT, 'crates', 'hpr-motor', 'data', 'thrustcurve');
const CATALOG_PATH = path.join(DATA_DIR, 'catalog.json');

// Error codes from lib/errors/errors.js at commit 577afa6 (the only ones analyze.js uses).
const ERRORS_STUB = Object.freeze({
  INVALID_INFO: 104,
  INVALID_POINTS: 105,
  DUPLICATE_POINTS: 106,
});

function fail(message) {
  process.stderr.write('analyze_stats.js: ' + message + '\n');
  process.exit(1);
}

function sha256(bytes) {
  return crypto.createHash('sha256').update(bytes).digest('hex');
}

function loadAnalyze() {
  let bytes;
  try {
    bytes = fs.readFileSync(ANALYZE_PATH);
  } catch (e) {
    fail('cannot read ' + ANALYZE_PATH + ' (' + e.message + '); fetch the thrustcurve3-analyze ' +
         'entry of validation/refs.lock.toml');
  }
  const digest = sha256(bytes);
  if (digest !== ANALYZE_SHA256) {
    fail('sha256 of ' + ANALYZE_PATH + ' is ' + digest + ', expected ' + ANALYZE_SHA256);
  }
  const params = ['exports', 'require', 'module', '__filename', '__dirname'];
  const factory = vm.compileFunction(bytes.toString('utf8'), params, { filename: ANALYZE_PATH });
  const mod = { exports: {} };
  const requireStub = function(id) {
    if (id === '../../lib/errors') return ERRORS_STUB;
    fail('analyze.js required unexpected module ' + JSON.stringify(id));
  };
  factory.call(mod.exports, mod.exports, requireStub, mod, ANALYZE_PATH,
               path.dirname(ANALYZE_PATH));
  const analyze = mod.exports;
  if (typeof analyze.stats !== 'function' || typeof analyze.normalize !== 'function') {
    fail('analyze.js does not export stats and normalize');
  }
  return analyze;
}

// lib/number isNumber at commit 577afa6: what ThrustCurve's parsers accept before parseFloat.
function parseNumber(text, where) {
  if (!(/^-?([0-9]+)(\.[0-9]*)?$/.test(text) || /^\.[0-9]+$/.test(text))) {
    fail(where + ': "' + text + '" is not a plain decimal number');
  }
  const n = parseFloat(text);
  if (!Number.isFinite(n) || n < 0) fail(where + ': "' + text + '" is not a non-negative number');
  return n;
}

function readEng(text, file) {
  const lines = text.trim().split(/\r?\n/).map((l) => l.trim());
  let i = 0;
  while (i < lines.length && (lines[i] === '' || lines[i].charAt(0) === ';')) i++;
  if (i >= lines.length) fail(file + ': no header line');
  if (lines[i].split(/\s+/).length < 7) fail(file + ': header line has fewer than seven fields');
  const points = [];
  for (i++; i < lines.length; i++) {
    const line = lines[i];
    if (line === '' || line.charAt(0) === ';') break;
    const fields = line.split(/\s+/);
    if (fields.length !== 2) {
      fail(file + ':' + (i + 1) + ': expected two fields (a second motor or a malformed line)');
    }
    points.push({
      time: parseNumber(fields[0], file + ':' + (i + 1)),
      thrust: parseNumber(fields[1], file + ':' + (i + 1)),
    });
  }
  for (; i < lines.length; i++) {
    if (lines[i] !== '' && lines[i].charAt(0) !== ';') {
      fail(file + ':' + (i + 1) + ': content after the data block (expected exactly one motor)');
    }
  }
  return points;
}

function readRse(text, file) {
  const engines = text.match(/<engine[\s>]/g) || [];
  if (engines.length !== 1) fail(file + ': ' + engines.length + ' <engine> elements, expected 1');
  const opened = (text.match(/<eng-data[\s/>]/g) || []).length;
  const points = [];
  const re = /<eng-data\s([^>]*?)\/>/g;
  let m;
  while ((m = re.exec(text)) !== null) {
    const attrs = m[1];
    const t = /(?:^|\s)t="([^"]*)"/.exec(attrs);
    const f = /(?:^|\s)f="([^"]*)"/.exec(attrs);
    if (!t || !f) fail(file + ': <eng-data> element #' + (points.length + 1) + ' lacks t or f');
    points.push({
      time: parseNumber(t[1], file + ' eng-data #' + (points.length + 1) + ' t'),
      thrust: parseNumber(f[1], file + ' eng-data #' + (points.length + 1) + ' f'),
    });
  }
  if (points.length !== opened) {
    fail(file + ': ' + opened + ' <eng-data elements but ' + points.length + ' parsed');
  }
  return points;
}

function copyPoints(points) {
  return points.map((p) => ({ time: p.time, thrust: p.thrust }));
}

function finite(value, name, file) {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    fail(file + ': stats.' + name + ' is not a finite number (' + value + ')');
  }
  return value;
}

function main() {
  const analyze = loadAnalyze();
  const catalog = JSON.parse(fs.readFileSync(CATALOG_PATH, 'utf8'));
  const curves = [];

  for (const motor of catalog.motors) {
    for (const curve of motor.curves) {
      const file = curve.file;
      const bytes = fs.readFileSync(path.join(DATA_DIR, file));
      if (sha256(bytes) !== curve.sha256) fail(file + ': sha256 does not match catalog.json');
      const text = bytes.toString('utf8');

      let points;
      if (file.endsWith('.eng') && curve.format === 'RASP') {
        points = readEng(text, file);
      } else if (file.endsWith('.rse') && curve.format === 'RockSim') {
        points = readRse(text, file);
      } else {
        fail(file + ': unsupported format ' + curve.format);
      }
      if (points.length < 2) fail(file + ': fewer than two data points');

      const reports = [];
      const error = function(code) {
        reports.push(code);
      };
      const stats = analyze.stats({ points: copyPoints(points) }, error);
      if (stats == null) fail(file + ': analyze.stats returned nothing');
      if (reports.includes(ERRORS_STUB.INVALID_POINTS)) {
        fail(file + ': analyze.js reported invalid points');
      }

      const cleaned = analyze.normalize(copyPoints(points), function() {});
      if (cleaned == null || cleaned.length !== stats.pointCount) {
        fail(file + ': normalize and stats disagree on the point count');
      }
      const changed = cleaned.length !== points.length ||
        cleaned.some((p, k) => p.time !== points[k].time || p.thrust !== points[k].thrust);

      curves.push({
        file: file,
        designation: motor.designation,
        raw_points: points.length,
        points: stats.pointCount,
        merged_or_dropped: changed,
        total_impulse_ns: finite(stats.totalImpulse, 'totalImpulse', file),
        burn_time_s: finite(stats.burnTime, 'burnTime', file),
        average_thrust_n: finite(stats.avgThrust, 'avgThrust', file),
        max_thrust_n: finite(stats.maxThrust, 'maxThrust', file),
        burn_start_s: finite(stats.burnStart, 'burnStart', file),
        burn_end_s: finite(stats.burnEnd, 'burnEnd', file),
      });
    }
  }

  const out = {
    oracle: 'ThrustCurve.org simulate/analyze/analyze.js at commit 577afa6 (ISC)',
    generator: 'validation/oracles/thrustcurve/analyze_stats.js',
    node: process.version,
    params: {
      burn_time_cutoff: analyze.StdParams.burnTimeCutoff,
      time_epsilon_s: analyze.StdParams.timeEpsilon,
      thrust_epsilon_n: analyze.StdParams.thrustEpsilon,
    },
    curves: curves,
  };
  process.stdout.write(JSON.stringify(out, null, 2) + '\n');
}

main();
