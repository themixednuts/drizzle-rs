"use strict";
var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __esm = (fn, res) => function __init() {
  return fn && (res = (0, fn[__getOwnPropNames(fn)[0]])(fn = 0)), res;
};
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

// ../../node_modules/.pnpm/pluralize@8.0.0/node_modules/pluralize/pluralize.js
var require_pluralize = __commonJS({
  "../../node_modules/.pnpm/pluralize@8.0.0/node_modules/pluralize/pluralize.js"(exports2, module2) {
    "use strict";
    (function(root, pluralize3) {
      if (typeof require === "function" && typeof exports2 === "object" && typeof module2 === "object") {
        module2.exports = pluralize3();
      } else if (typeof define === "function" && define.amd) {
        define(function() {
          return pluralize3();
        });
      } else {
        root.pluralize = pluralize3();
      }
    })(exports2, function() {
      var pluralRules = [];
      var singularRules = [];
      var uncountables = {};
      var irregularPlurals = {};
      var irregularSingles = {};
      function sanitizeRule(rule) {
        if (typeof rule === "string") {
          return new RegExp("^" + rule + "$", "i");
        }
        return rule;
      }
      function restoreCase(word, token) {
        if (word === token) return token;
        if (word === word.toLowerCase()) return token.toLowerCase();
        if (word === word.toUpperCase()) return token.toUpperCase();
        if (word[0] === word[0].toUpperCase()) {
          return token.charAt(0).toUpperCase() + token.substr(1).toLowerCase();
        }
        return token.toLowerCase();
      }
      function interpolate(str, args) {
        return str.replace(/\$(\d{1,2})/g, function(match, index) {
          return args[index] || "";
        });
      }
      function replace(word, rule) {
        return word.replace(rule[0], function(match, index) {
          var result = interpolate(rule[1], arguments);
          if (match === "") {
            return restoreCase(word[index - 1], result);
          }
          return restoreCase(match, result);
        });
      }
      function sanitizeWord(token, word, rules) {
        if (!token.length || uncountables.hasOwnProperty(token)) {
          return word;
        }
        var len = rules.length;
        while (len--) {
          var rule = rules[len];
          if (rule[0].test(word)) return replace(word, rule);
        }
        return word;
      }
      function replaceWord(replaceMap, keepMap, rules) {
        return function(word) {
          var token = word.toLowerCase();
          if (keepMap.hasOwnProperty(token)) {
            return restoreCase(word, token);
          }
          if (replaceMap.hasOwnProperty(token)) {
            return restoreCase(word, replaceMap[token]);
          }
          return sanitizeWord(token, word, rules);
        };
      }
      function checkWord(replaceMap, keepMap, rules, bool) {
        return function(word) {
          var token = word.toLowerCase();
          if (keepMap.hasOwnProperty(token)) return true;
          if (replaceMap.hasOwnProperty(token)) return false;
          return sanitizeWord(token, token, rules) === token;
        };
      }
      function pluralize3(word, count, inclusive) {
        var pluralized = count === 1 ? pluralize3.singular(word) : pluralize3.plural(word);
        return (inclusive ? count + " " : "") + pluralized;
      }
      pluralize3.plural = replaceWord(
        irregularSingles,
        irregularPlurals,
        pluralRules
      );
      pluralize3.isPlural = checkWord(
        irregularSingles,
        irregularPlurals,
        pluralRules
      );
      pluralize3.singular = replaceWord(
        irregularPlurals,
        irregularSingles,
        singularRules
      );
      pluralize3.isSingular = checkWord(
        irregularPlurals,
        irregularSingles,
        singularRules
      );
      pluralize3.addPluralRule = function(rule, replacement) {
        pluralRules.push([sanitizeRule(rule), replacement]);
      };
      pluralize3.addSingularRule = function(rule, replacement) {
        singularRules.push([sanitizeRule(rule), replacement]);
      };
      pluralize3.addUncountableRule = function(word) {
        if (typeof word === "string") {
          uncountables[word.toLowerCase()] = true;
          return;
        }
        pluralize3.addPluralRule(word, "$0");
        pluralize3.addSingularRule(word, "$0");
      };
      pluralize3.addIrregularRule = function(single, plural) {
        plural = plural.toLowerCase();
        single = single.toLowerCase();
        irregularSingles[single] = plural;
        irregularPlurals[plural] = single;
      };
      [
        // Pronouns.
        ["I", "we"],
        ["me", "us"],
        ["he", "they"],
        ["she", "they"],
        ["them", "them"],
        ["myself", "ourselves"],
        ["yourself", "yourselves"],
        ["itself", "themselves"],
        ["herself", "themselves"],
        ["himself", "themselves"],
        ["themself", "themselves"],
        ["is", "are"],
        ["was", "were"],
        ["has", "have"],
        ["this", "these"],
        ["that", "those"],
        // Words ending in with a consonant and `o`.
        ["echo", "echoes"],
        ["dingo", "dingoes"],
        ["volcano", "volcanoes"],
        ["tornado", "tornadoes"],
        ["torpedo", "torpedoes"],
        // Ends with `us`.
        ["genus", "genera"],
        ["viscus", "viscera"],
        // Ends with `ma`.
        ["stigma", "stigmata"],
        ["stoma", "stomata"],
        ["dogma", "dogmata"],
        ["lemma", "lemmata"],
        ["schema", "schemata"],
        ["anathema", "anathemata"],
        // Other irregular rules.
        ["ox", "oxen"],
        ["axe", "axes"],
        ["die", "dice"],
        ["yes", "yeses"],
        ["foot", "feet"],
        ["eave", "eaves"],
        ["goose", "geese"],
        ["tooth", "teeth"],
        ["quiz", "quizzes"],
        ["human", "humans"],
        ["proof", "proofs"],
        ["carve", "carves"],
        ["valve", "valves"],
        ["looey", "looies"],
        ["thief", "thieves"],
        ["groove", "grooves"],
        ["pickaxe", "pickaxes"],
        ["passerby", "passersby"]
      ].forEach(function(rule) {
        return pluralize3.addIrregularRule(rule[0], rule[1]);
      });
      [
        [/s?$/i, "s"],
        [/[^\u0000-\u007F]$/i, "$0"],
        [/([^aeiou]ese)$/i, "$1"],
        [/(ax|test)is$/i, "$1es"],
        [/(alias|[^aou]us|t[lm]as|gas|ris)$/i, "$1es"],
        [/(e[mn]u)s?$/i, "$1s"],
        [/([^l]ias|[aeiou]las|[ejzr]as|[iu]am)$/i, "$1"],
        [/(alumn|syllab|vir|radi|nucle|fung|cact|stimul|termin|bacill|foc|uter|loc|strat)(?:us|i)$/i, "$1i"],
        [/(alumn|alg|vertebr)(?:a|ae)$/i, "$1ae"],
        [/(seraph|cherub)(?:im)?$/i, "$1im"],
        [/(her|at|gr)o$/i, "$1oes"],
        [/(agend|addend|millenni|dat|extrem|bacteri|desiderat|strat|candelabr|errat|ov|symposi|curricul|automat|quor)(?:a|um)$/i, "$1a"],
        [/(apheli|hyperbat|periheli|asyndet|noumen|phenomen|criteri|organ|prolegomen|hedr|automat)(?:a|on)$/i, "$1a"],
        [/sis$/i, "ses"],
        [/(?:(kni|wi|li)fe|(ar|l|ea|eo|oa|hoo)f)$/i, "$1$2ves"],
        [/([^aeiouy]|qu)y$/i, "$1ies"],
        [/([^ch][ieo][ln])ey$/i, "$1ies"],
        [/(x|ch|ss|sh|zz)$/i, "$1es"],
        [/(matr|cod|mur|sil|vert|ind|append)(?:ix|ex)$/i, "$1ices"],
        [/\b((?:tit)?m|l)(?:ice|ouse)$/i, "$1ice"],
        [/(pe)(?:rson|ople)$/i, "$1ople"],
        [/(child)(?:ren)?$/i, "$1ren"],
        [/eaux$/i, "$0"],
        [/m[ae]n$/i, "men"],
        ["thou", "you"]
      ].forEach(function(rule) {
        return pluralize3.addPluralRule(rule[0], rule[1]);
      });
      [
        [/s$/i, ""],
        [/(ss)$/i, "$1"],
        [/(wi|kni|(?:after|half|high|low|mid|non|night|[^\w]|^)li)ves$/i, "$1fe"],
        [/(ar|(?:wo|[ae])l|[eo][ao])ves$/i, "$1f"],
        [/ies$/i, "y"],
        [/\b([pl]|zomb|(?:neck|cross)?t|coll|faer|food|gen|goon|group|lass|talk|goal|cut)ies$/i, "$1ie"],
        [/\b(mon|smil)ies$/i, "$1ey"],
        [/\b((?:tit)?m|l)ice$/i, "$1ouse"],
        [/(seraph|cherub)im$/i, "$1"],
        [/(x|ch|ss|sh|zz|tto|go|cho|alias|[^aou]us|t[lm]as|gas|(?:her|at|gr)o|[aeiou]ris)(?:es)?$/i, "$1"],
        [/(analy|diagno|parenthe|progno|synop|the|empha|cri|ne)(?:sis|ses)$/i, "$1sis"],
        [/(movie|twelve|abuse|e[mn]u)s$/i, "$1"],
        [/(test)(?:is|es)$/i, "$1is"],
        [/(alumn|syllab|vir|radi|nucle|fung|cact|stimul|termin|bacill|foc|uter|loc|strat)(?:us|i)$/i, "$1us"],
        [/(agend|addend|millenni|dat|extrem|bacteri|desiderat|strat|candelabr|errat|ov|symposi|curricul|quor)a$/i, "$1um"],
        [/(apheli|hyperbat|periheli|asyndet|noumen|phenomen|criteri|organ|prolegomen|hedr|automat)a$/i, "$1on"],
        [/(alumn|alg|vertebr)ae$/i, "$1a"],
        [/(cod|mur|sil|vert|ind)ices$/i, "$1ex"],
        [/(matr|append)ices$/i, "$1ix"],
        [/(pe)(rson|ople)$/i, "$1rson"],
        [/(child)ren$/i, "$1"],
        [/(eau)x?$/i, "$1"],
        [/men$/i, "man"]
      ].forEach(function(rule) {
        return pluralize3.addSingularRule(rule[0], rule[1]);
      });
      [
        // Singular words with no plurals.
        "adulthood",
        "advice",
        "agenda",
        "aid",
        "aircraft",
        "alcohol",
        "ammo",
        "analytics",
        "anime",
        "athletics",
        "audio",
        "bison",
        "blood",
        "bream",
        "buffalo",
        "butter",
        "carp",
        "cash",
        "chassis",
        "chess",
        "clothing",
        "cod",
        "commerce",
        "cooperation",
        "corps",
        "debris",
        "diabetes",
        "digestion",
        "elk",
        "energy",
        "equipment",
        "excretion",
        "expertise",
        "firmware",
        "flounder",
        "fun",
        "gallows",
        "garbage",
        "graffiti",
        "hardware",
        "headquarters",
        "health",
        "herpes",
        "highjinks",
        "homework",
        "housework",
        "information",
        "jeans",
        "justice",
        "kudos",
        "labour",
        "literature",
        "machinery",
        "mackerel",
        "mail",
        "media",
        "mews",
        "moose",
        "music",
        "mud",
        "manga",
        "news",
        "only",
        "personnel",
        "pike",
        "plankton",
        "pliers",
        "police",
        "pollution",
        "premises",
        "rain",
        "research",
        "rice",
        "salmon",
        "scissors",
        "series",
        "sewage",
        "shambles",
        "shrimp",
        "software",
        "species",
        "staff",
        "swine",
        "tennis",
        "traffic",
        "transportation",
        "trout",
        "tuna",
        "wealth",
        "welfare",
        "whiting",
        "wildebeest",
        "wildlife",
        "you",
        /pok[eé]mon$/i,
        // Regexes.
        /[^aeiou]ese$/i,
        // "chinese", "japanese"
        /deer$/i,
        // "deer", "reindeer"
        /fish$/i,
        // "fish", "blowfish", "angelfish"
        /measles$/i,
        /o[iu]s$/i,
        // "carnivorous"
        /pox$/i,
        // "chickpox", "smallpox"
        /sheep$/i
      ].forEach(pluralize3.addUncountableRule);
      return pluralize3;
    });
  }
});

// ../../node_modules/.pnpm/universalify@2.0.1/node_modules/universalify/index.js
var require_universalify = __commonJS({
  "../../node_modules/.pnpm/universalify@2.0.1/node_modules/universalify/index.js"(exports2) {
    "use strict";
    exports2.fromCallback = function(fn) {
      return Object.defineProperty(function(...args) {
        if (typeof args[args.length - 1] === "function") fn.apply(this, args);
        else {
          return new Promise((resolve3, reject) => {
            args.push((err, res) => err != null ? reject(err) : resolve3(res));
            fn.apply(this, args);
          });
        }
      }, "name", { value: fn.name });
    };
    exports2.fromPromise = function(fn) {
      return Object.defineProperty(function(...args) {
        const cb = args[args.length - 1];
        if (typeof cb !== "function") return fn.apply(this, args);
        else {
          args.pop();
          fn.apply(this, args).then((r) => cb(null, r), cb);
        }
      }, "name", { value: fn.name });
    };
  }
});

// ../../node_modules/.pnpm/graceful-fs@4.2.11/node_modules/graceful-fs/polyfills.js
var require_polyfills = __commonJS({
  "../../node_modules/.pnpm/graceful-fs@4.2.11/node_modules/graceful-fs/polyfills.js"(exports2, module2) {
    "use strict";
    var constants2 = require("constants");
    var origCwd = process.cwd;
    var cwd = null;
    var platform = process.env.GRACEFUL_FS_PLATFORM || process.platform;
    process.cwd = function() {
      if (!cwd)
        cwd = origCwd.call(process);
      return cwd;
    };
    try {
      process.cwd();
    } catch (er) {
    }
    if (typeof process.chdir === "function") {
      chdir = process.chdir;
      process.chdir = function(d) {
        cwd = null;
        chdir.call(process, d);
      };
      if (Object.setPrototypeOf) Object.setPrototypeOf(process.chdir, chdir);
    }
    var chdir;
    module2.exports = patch;
    function patch(fs4) {
      if (constants2.hasOwnProperty("O_SYMLINK") && process.version.match(/^v0\.6\.[0-2]|^v0\.5\./)) {
        patchLchmod(fs4);
      }
      if (!fs4.lutimes) {
        patchLutimes(fs4);
      }
      fs4.chown = chownFix(fs4.chown);
      fs4.fchown = chownFix(fs4.fchown);
      fs4.lchown = chownFix(fs4.lchown);
      fs4.chmod = chmodFix(fs4.chmod);
      fs4.fchmod = chmodFix(fs4.fchmod);
      fs4.lchmod = chmodFix(fs4.lchmod);
      fs4.chownSync = chownFixSync(fs4.chownSync);
      fs4.fchownSync = chownFixSync(fs4.fchownSync);
      fs4.lchownSync = chownFixSync(fs4.lchownSync);
      fs4.chmodSync = chmodFixSync(fs4.chmodSync);
      fs4.fchmodSync = chmodFixSync(fs4.fchmodSync);
      fs4.lchmodSync = chmodFixSync(fs4.lchmodSync);
      fs4.stat = statFix(fs4.stat);
      fs4.fstat = statFix(fs4.fstat);
      fs4.lstat = statFix(fs4.lstat);
      fs4.statSync = statFixSync(fs4.statSync);
      fs4.fstatSync = statFixSync(fs4.fstatSync);
      fs4.lstatSync = statFixSync(fs4.lstatSync);
      if (fs4.chmod && !fs4.lchmod) {
        fs4.lchmod = function(path10, mode2, cb) {
          if (cb) process.nextTick(cb);
        };
        fs4.lchmodSync = function() {
        };
      }
      if (fs4.chown && !fs4.lchown) {
        fs4.lchown = function(path10, uid, gid, cb) {
          if (cb) process.nextTick(cb);
        };
        fs4.lchownSync = function() {
        };
      }
      if (platform === "win32") {
        fs4.rename = typeof fs4.rename !== "function" ? fs4.rename : function(fs$rename) {
          function rename(from, to, cb) {
            var start = Date.now();
            var backoff = 0;
            fs$rename(from, to, function CB(er) {
              if (er && (er.code === "EACCES" || er.code === "EPERM" || er.code === "EBUSY") && Date.now() - start < 6e4) {
                setTimeout(function() {
                  fs4.stat(to, function(stater, st) {
                    if (stater && stater.code === "ENOENT")
                      fs$rename(from, to, CB);
                    else
                      cb(er);
                  });
                }, backoff);
                if (backoff < 100)
                  backoff += 10;
                return;
              }
              if (cb) cb(er);
            });
          }
          if (Object.setPrototypeOf) Object.setPrototypeOf(rename, fs$rename);
          return rename;
        }(fs4.rename);
      }
      fs4.read = typeof fs4.read !== "function" ? fs4.read : function(fs$read) {
        function read(fd, buffer, offset, length, position, callback_) {
          var callback;
          if (callback_ && typeof callback_ === "function") {
            var eagCounter = 0;
            callback = function(er, _, __) {
              if (er && er.code === "EAGAIN" && eagCounter < 10) {
                eagCounter++;
                return fs$read.call(fs4, fd, buffer, offset, length, position, callback);
              }
              callback_.apply(this, arguments);
            };
          }
          return fs$read.call(fs4, fd, buffer, offset, length, position, callback);
        }
        if (Object.setPrototypeOf) Object.setPrototypeOf(read, fs$read);
        return read;
      }(fs4.read);
      fs4.readSync = typeof fs4.readSync !== "function" ? fs4.readSync : /* @__PURE__ */ function(fs$readSync) {
        return function(fd, buffer, offset, length, position) {
          var eagCounter = 0;
          while (true) {
            try {
              return fs$readSync.call(fs4, fd, buffer, offset, length, position);
            } catch (er) {
              if (er.code === "EAGAIN" && eagCounter < 10) {
                eagCounter++;
                continue;
              }
              throw er;
            }
          }
        };
      }(fs4.readSync);
      function patchLchmod(fs5) {
        fs5.lchmod = function(path10, mode2, callback) {
          fs5.open(
            path10,
            constants2.O_WRONLY | constants2.O_SYMLINK,
            mode2,
            function(err, fd) {
              if (err) {
                if (callback) callback(err);
                return;
              }
              fs5.fchmod(fd, mode2, function(err2) {
                fs5.close(fd, function(err22) {
                  if (callback) callback(err2 || err22);
                });
              });
            }
          );
        };
        fs5.lchmodSync = function(path10, mode2) {
          var fd = fs5.openSync(path10, constants2.O_WRONLY | constants2.O_SYMLINK, mode2);
          var threw = true;
          var ret;
          try {
            ret = fs5.fchmodSync(fd, mode2);
            threw = false;
          } finally {
            if (threw) {
              try {
                fs5.closeSync(fd);
              } catch (er) {
              }
            } else {
              fs5.closeSync(fd);
            }
          }
          return ret;
        };
      }
      function patchLutimes(fs5) {
        if (constants2.hasOwnProperty("O_SYMLINK") && fs5.futimes) {
          fs5.lutimes = function(path10, at, mt, cb) {
            fs5.open(path10, constants2.O_SYMLINK, function(er, fd) {
              if (er) {
                if (cb) cb(er);
                return;
              }
              fs5.futimes(fd, at, mt, function(er2) {
                fs5.close(fd, function(er22) {
                  if (cb) cb(er2 || er22);
                });
              });
            });
          };
          fs5.lutimesSync = function(path10, at, mt) {
            var fd = fs5.openSync(path10, constants2.O_SYMLINK);
            var ret;
            var threw = true;
            try {
              ret = fs5.futimesSync(fd, at, mt);
              threw = false;
            } finally {
              if (threw) {
                try {
                  fs5.closeSync(fd);
                } catch (er) {
                }
              } else {
                fs5.closeSync(fd);
              }
            }
            return ret;
          };
        } else if (fs5.futimes) {
          fs5.lutimes = function(_a, _b, _c, cb) {
            if (cb) process.nextTick(cb);
          };
          fs5.lutimesSync = function() {
          };
        }
      }
      function chmodFix(orig) {
        if (!orig) return orig;
        return function(target, mode2, cb) {
          return orig.call(fs4, target, mode2, function(er) {
            if (chownErOk(er)) er = null;
            if (cb) cb.apply(this, arguments);
          });
        };
      }
      function chmodFixSync(orig) {
        if (!orig) return orig;
        return function(target, mode2) {
          try {
            return orig.call(fs4, target, mode2);
          } catch (er) {
            if (!chownErOk(er)) throw er;
          }
        };
      }
      function chownFix(orig) {
        if (!orig) return orig;
        return function(target, uid, gid, cb) {
          return orig.call(fs4, target, uid, gid, function(er) {
            if (chownErOk(er)) er = null;
            if (cb) cb.apply(this, arguments);
          });
        };
      }
      function chownFixSync(orig) {
        if (!orig) return orig;
        return function(target, uid, gid) {
          try {
            return orig.call(fs4, target, uid, gid);
          } catch (er) {
            if (!chownErOk(er)) throw er;
          }
        };
      }
      function statFix(orig) {
        if (!orig) return orig;
        return function(target, options2, cb) {
          if (typeof options2 === "function") {
            cb = options2;
            options2 = null;
          }
          function callback(er, stats) {
            if (stats) {
              if (stats.uid < 0) stats.uid += 4294967296;
              if (stats.gid < 0) stats.gid += 4294967296;
            }
            if (cb) cb.apply(this, arguments);
          }
          return options2 ? orig.call(fs4, target, options2, callback) : orig.call(fs4, target, callback);
        };
      }
      function statFixSync(orig) {
        if (!orig) return orig;
        return function(target, options2) {
          var stats = options2 ? orig.call(fs4, target, options2) : orig.call(fs4, target);
          if (stats) {
            if (stats.uid < 0) stats.uid += 4294967296;
            if (stats.gid < 0) stats.gid += 4294967296;
          }
          return stats;
        };
      }
      function chownErOk(er) {
        if (!er)
          return true;
        if (er.code === "ENOSYS")
          return true;
        var nonroot = !process.getuid || process.getuid() !== 0;
        if (nonroot) {
          if (er.code === "EINVAL" || er.code === "EPERM")
            return true;
        }
        return false;
      }
    }
  }
});

// ../../node_modules/.pnpm/graceful-fs@4.2.11/node_modules/graceful-fs/legacy-streams.js
var require_legacy_streams = __commonJS({
  "../../node_modules/.pnpm/graceful-fs@4.2.11/node_modules/graceful-fs/legacy-streams.js"(exports2, module2) {
    "use strict";
    var Stream = require("stream").Stream;
    module2.exports = legacy;
    function legacy(fs4) {
      return {
        ReadStream,
        WriteStream
      };
      function ReadStream(path10, options2) {
        if (!(this instanceof ReadStream)) return new ReadStream(path10, options2);
        Stream.call(this);
        var self2 = this;
        this.path = path10;
        this.fd = null;
        this.readable = true;
        this.paused = false;
        this.flags = "r";
        this.mode = 438;
        this.bufferSize = 64 * 1024;
        options2 = options2 || {};
        var keys = Object.keys(options2);
        for (var index = 0, length = keys.length; index < length; index++) {
          var key = keys[index];
          this[key] = options2[key];
        }
        if (this.encoding) this.setEncoding(this.encoding);
        if (this.start !== void 0) {
          if ("number" !== typeof this.start) {
            throw TypeError("start must be a Number");
          }
          if (this.end === void 0) {
            this.end = Infinity;
          } else if ("number" !== typeof this.end) {
            throw TypeError("end must be a Number");
          }
          if (this.start > this.end) {
            throw new Error("start must be <= end");
          }
          this.pos = this.start;
        }
        if (this.fd !== null) {
          process.nextTick(function() {
            self2._read();
          });
          return;
        }
        fs4.open(this.path, this.flags, this.mode, function(err, fd) {
          if (err) {
            self2.emit("error", err);
            self2.readable = false;
            return;
          }
          self2.fd = fd;
          self2.emit("open", fd);
          self2._read();
        });
      }
      function WriteStream(path10, options2) {
        if (!(this instanceof WriteStream)) return new WriteStream(path10, options2);
        Stream.call(this);
        this.path = path10;
        this.fd = null;
        this.writable = true;
        this.flags = "w";
        this.encoding = "binary";
        this.mode = 438;
        this.bytesWritten = 0;
        options2 = options2 || {};
        var keys = Object.keys(options2);
        for (var index = 0, length = keys.length; index < length; index++) {
          var key = keys[index];
          this[key] = options2[key];
        }
        if (this.start !== void 0) {
          if ("number" !== typeof this.start) {
            throw TypeError("start must be a Number");
          }
          if (this.start < 0) {
            throw new Error("start must be >= zero");
          }
          this.pos = this.start;
        }
        this.busy = false;
        this._queue = [];
        if (this.fd === null) {
          this._open = fs4.open;
          this._queue.push([this._open, this.path, this.flags, this.mode, void 0]);
          this.flush();
        }
      }
    }
  }
});

// ../../node_modules/.pnpm/graceful-fs@4.2.11/node_modules/graceful-fs/clone.js
var require_clone = __commonJS({
  "../../node_modules/.pnpm/graceful-fs@4.2.11/node_modules/graceful-fs/clone.js"(exports2, module2) {
    "use strict";
    module2.exports = clone;
    var getPrototypeOf = Object.getPrototypeOf || function(obj) {
      return obj.__proto__;
    };
    function clone(obj) {
      if (obj === null || typeof obj !== "object")
        return obj;
      if (obj instanceof Object)
        var copy = { __proto__: getPrototypeOf(obj) };
      else
        var copy = /* @__PURE__ */ Object.create(null);
      Object.getOwnPropertyNames(obj).forEach(function(key) {
        Object.defineProperty(copy, key, Object.getOwnPropertyDescriptor(obj, key));
      });
      return copy;
    }
  }
});

// ../../node_modules/.pnpm/graceful-fs@4.2.11/node_modules/graceful-fs/graceful-fs.js
var require_graceful_fs = __commonJS({
  "../../node_modules/.pnpm/graceful-fs@4.2.11/node_modules/graceful-fs/graceful-fs.js"(exports2, module2) {
    "use strict";
    var fs4 = require("fs");
    var polyfills = require_polyfills();
    var legacy = require_legacy_streams();
    var clone = require_clone();
    var util2 = require("util");
    var gracefulQueue;
    var previousSymbol;
    if (typeof Symbol === "function" && typeof Symbol.for === "function") {
      gracefulQueue = Symbol.for("graceful-fs.queue");
      previousSymbol = Symbol.for("graceful-fs.previous");
    } else {
      gracefulQueue = "___graceful-fs.queue";
      previousSymbol = "___graceful-fs.previous";
    }
    function noop2() {
    }
    function publishQueue(context, queue2) {
      Object.defineProperty(context, gracefulQueue, {
        get: function() {
          return queue2;
        }
      });
    }
    var debug2 = noop2;
    if (util2.debuglog)
      debug2 = util2.debuglog("gfs4");
    else if (/\bgfs4\b/i.test(process.env.NODE_DEBUG || ""))
      debug2 = function() {
        var m = util2.format.apply(util2, arguments);
        m = "GFS4: " + m.split(/\n/).join("\nGFS4: ");
        console.error(m);
      };
    if (!fs4[gracefulQueue]) {
      queue = global[gracefulQueue] || [];
      publishQueue(fs4, queue);
      fs4.close = function(fs$close) {
        function close(fd, cb) {
          return fs$close.call(fs4, fd, function(err) {
            if (!err) {
              resetQueue();
            }
            if (typeof cb === "function")
              cb.apply(this, arguments);
          });
        }
        Object.defineProperty(close, previousSymbol, {
          value: fs$close
        });
        return close;
      }(fs4.close);
      fs4.closeSync = function(fs$closeSync) {
        function closeSync(fd) {
          fs$closeSync.apply(fs4, arguments);
          resetQueue();
        }
        Object.defineProperty(closeSync, previousSymbol, {
          value: fs$closeSync
        });
        return closeSync;
      }(fs4.closeSync);
      if (/\bgfs4\b/i.test(process.env.NODE_DEBUG || "")) {
        process.on("exit", function() {
          debug2(fs4[gracefulQueue]);
          require("assert").equal(fs4[gracefulQueue].length, 0);
        });
      }
    }
    var queue;
    if (!global[gracefulQueue]) {
      publishQueue(global, fs4[gracefulQueue]);
    }
    module2.exports = patch(clone(fs4));
    if (process.env.TEST_GRACEFUL_FS_GLOBAL_PATCH && !fs4.__patched) {
      module2.exports = patch(fs4);
      fs4.__patched = true;
    }
    function patch(fs5) {
      polyfills(fs5);
      fs5.gracefulify = patch;
      fs5.createReadStream = createReadStream2;
      fs5.createWriteStream = createWriteStream2;
      var fs$readFile = fs5.readFile;
      fs5.readFile = readFile;
      function readFile(path10, options2, cb) {
        if (typeof options2 === "function")
          cb = options2, options2 = null;
        return go$readFile(path10, options2, cb);
        function go$readFile(path11, options3, cb2, startTime) {
          return fs$readFile(path11, options3, function(err) {
            if (err && (err.code === "EMFILE" || err.code === "ENFILE"))
              enqueue([go$readFile, [path11, options3, cb2], err, startTime || Date.now(), Date.now()]);
            else {
              if (typeof cb2 === "function")
                cb2.apply(this, arguments);
            }
          });
        }
      }
      var fs$writeFile = fs5.writeFile;
      fs5.writeFile = writeFile;
      function writeFile(path10, data, options2, cb) {
        if (typeof options2 === "function")
          cb = options2, options2 = null;
        return go$writeFile(path10, data, options2, cb);
        function go$writeFile(path11, data2, options3, cb2, startTime) {
          return fs$writeFile(path11, data2, options3, function(err) {
            if (err && (err.code === "EMFILE" || err.code === "ENFILE"))
              enqueue([go$writeFile, [path11, data2, options3, cb2], err, startTime || Date.now(), Date.now()]);
            else {
              if (typeof cb2 === "function")
                cb2.apply(this, arguments);
            }
          });
        }
      }
      var fs$appendFile = fs5.appendFile;
      if (fs$appendFile)
        fs5.appendFile = appendFile;
      function appendFile(path10, data, options2, cb) {
        if (typeof options2 === "function")
          cb = options2, options2 = null;
        return go$appendFile(path10, data, options2, cb);
        function go$appendFile(path11, data2, options3, cb2, startTime) {
          return fs$appendFile(path11, data2, options3, function(err) {
            if (err && (err.code === "EMFILE" || err.code === "ENFILE"))
              enqueue([go$appendFile, [path11, data2, options3, cb2], err, startTime || Date.now(), Date.now()]);
            else {
              if (typeof cb2 === "function")
                cb2.apply(this, arguments);
            }
          });
        }
      }
      var fs$copyFile = fs5.copyFile;
      if (fs$copyFile)
        fs5.copyFile = copyFile;
      function copyFile(src2, dest, flags, cb) {
        if (typeof flags === "function") {
          cb = flags;
          flags = 0;
        }
        return go$copyFile(src2, dest, flags, cb);
        function go$copyFile(src3, dest2, flags2, cb2, startTime) {
          return fs$copyFile(src3, dest2, flags2, function(err) {
            if (err && (err.code === "EMFILE" || err.code === "ENFILE"))
              enqueue([go$copyFile, [src3, dest2, flags2, cb2], err, startTime || Date.now(), Date.now()]);
            else {
              if (typeof cb2 === "function")
                cb2.apply(this, arguments);
            }
          });
        }
      }
      var fs$readdir = fs5.readdir;
      fs5.readdir = readdir;
      var noReaddirOptionVersions = /^v[0-5]\./;
      function readdir(path10, options2, cb) {
        if (typeof options2 === "function")
          cb = options2, options2 = null;
        var go$readdir = noReaddirOptionVersions.test(process.version) ? function go$readdir2(path11, options3, cb2, startTime) {
          return fs$readdir(path11, fs$readdirCallback(
            path11,
            options3,
            cb2,
            startTime
          ));
        } : function go$readdir2(path11, options3, cb2, startTime) {
          return fs$readdir(path11, options3, fs$readdirCallback(
            path11,
            options3,
            cb2,
            startTime
          ));
        };
        return go$readdir(path10, options2, cb);
        function fs$readdirCallback(path11, options3, cb2, startTime) {
          return function(err, files) {
            if (err && (err.code === "EMFILE" || err.code === "ENFILE"))
              enqueue([
                go$readdir,
                [path11, options3, cb2],
                err,
                startTime || Date.now(),
                Date.now()
              ]);
            else {
              if (files && files.sort)
                files.sort();
              if (typeof cb2 === "function")
                cb2.call(this, err, files);
            }
          };
        }
      }
      if (process.version.substr(0, 4) === "v0.8") {
        var legStreams = legacy(fs5);
        ReadStream = legStreams.ReadStream;
        WriteStream = legStreams.WriteStream;
      }
      var fs$ReadStream = fs5.ReadStream;
      if (fs$ReadStream) {
        ReadStream.prototype = Object.create(fs$ReadStream.prototype);
        ReadStream.prototype.open = ReadStream$open;
      }
      var fs$WriteStream = fs5.WriteStream;
      if (fs$WriteStream) {
        WriteStream.prototype = Object.create(fs$WriteStream.prototype);
        WriteStream.prototype.open = WriteStream$open;
      }
      Object.defineProperty(fs5, "ReadStream", {
        get: function() {
          return ReadStream;
        },
        set: function(val) {
          ReadStream = val;
        },
        enumerable: true,
        configurable: true
      });
      Object.defineProperty(fs5, "WriteStream", {
        get: function() {
          return WriteStream;
        },
        set: function(val) {
          WriteStream = val;
        },
        enumerable: true,
        configurable: true
      });
      var FileReadStream = ReadStream;
      Object.defineProperty(fs5, "FileReadStream", {
        get: function() {
          return FileReadStream;
        },
        set: function(val) {
          FileReadStream = val;
        },
        enumerable: true,
        configurable: true
      });
      var FileWriteStream = WriteStream;
      Object.defineProperty(fs5, "FileWriteStream", {
        get: function() {
          return FileWriteStream;
        },
        set: function(val) {
          FileWriteStream = val;
        },
        enumerable: true,
        configurable: true
      });
      function ReadStream(path10, options2) {
        if (this instanceof ReadStream)
          return fs$ReadStream.apply(this, arguments), this;
        else
          return ReadStream.apply(Object.create(ReadStream.prototype), arguments);
      }
      function ReadStream$open() {
        var that = this;
        open(that.path, that.flags, that.mode, function(err, fd) {
          if (err) {
            if (that.autoClose)
              that.destroy();
            that.emit("error", err);
          } else {
            that.fd = fd;
            that.emit("open", fd);
            that.read();
          }
        });
      }
      function WriteStream(path10, options2) {
        if (this instanceof WriteStream)
          return fs$WriteStream.apply(this, arguments), this;
        else
          return WriteStream.apply(Object.create(WriteStream.prototype), arguments);
      }
      function WriteStream$open() {
        var that = this;
        open(that.path, that.flags, that.mode, function(err, fd) {
          if (err) {
            that.destroy();
            that.emit("error", err);
          } else {
            that.fd = fd;
            that.emit("open", fd);
          }
        });
      }
      function createReadStream2(path10, options2) {
        return new fs5.ReadStream(path10, options2);
      }
      function createWriteStream2(path10, options2) {
        return new fs5.WriteStream(path10, options2);
      }
      var fs$open = fs5.open;
      fs5.open = open;
      function open(path10, flags, mode2, cb) {
        if (typeof mode2 === "function")
          cb = mode2, mode2 = null;
        return go$open(path10, flags, mode2, cb);
        function go$open(path11, flags2, mode3, cb2, startTime) {
          return fs$open(path11, flags2, mode3, function(err, fd) {
            if (err && (err.code === "EMFILE" || err.code === "ENFILE"))
              enqueue([go$open, [path11, flags2, mode3, cb2], err, startTime || Date.now(), Date.now()]);
            else {
              if (typeof cb2 === "function")
                cb2.apply(this, arguments);
            }
          });
        }
      }
      return fs5;
    }
    function enqueue(elem) {
      debug2("ENQUEUE", elem[0].name, elem[1]);
      fs4[gracefulQueue].push(elem);
      retry();
    }
    var retryTimer;
    function resetQueue() {
      var now = Date.now();
      for (var i = 0; i < fs4[gracefulQueue].length; ++i) {
        if (fs4[gracefulQueue][i].length > 2) {
          fs4[gracefulQueue][i][3] = now;
          fs4[gracefulQueue][i][4] = now;
        }
      }
      retry();
    }
    function retry() {
      clearTimeout(retryTimer);
      retryTimer = void 0;
      if (fs4[gracefulQueue].length === 0)
        return;
      var elem = fs4[gracefulQueue].shift();
      var fn = elem[0];
      var args = elem[1];
      var err = elem[2];
      var startTime = elem[3];
      var lastTime = elem[4];
      if (startTime === void 0) {
        debug2("RETRY", fn.name, args);
        fn.apply(null, args);
      } else if (Date.now() - startTime >= 6e4) {
        debug2("TIMEOUT", fn.name, args);
        var cb = args.pop();
        if (typeof cb === "function")
          cb.call(null, err);
      } else {
        var sinceAttempt = Date.now() - lastTime;
        var sinceStart = Math.max(lastTime - startTime, 1);
        var desiredDelay = Math.min(sinceStart * 1.2, 100);
        if (sinceAttempt >= desiredDelay) {
          debug2("RETRY", fn.name, args);
          fn.apply(null, args.concat([startTime]));
        } else {
          fs4[gracefulQueue].push(elem);
        }
      }
      if (retryTimer === void 0) {
        retryTimer = setTimeout(retry, 0);
      }
    }
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/fs/index.js
var require_fs = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/fs/index.js"(exports2) {
    "use strict";
    var u = require_universalify().fromCallback;
    var fs4 = require_graceful_fs();
    var api = [
      "access",
      "appendFile",
      "chmod",
      "chown",
      "close",
      "copyFile",
      "cp",
      "fchmod",
      "fchown",
      "fdatasync",
      "fstat",
      "fsync",
      "ftruncate",
      "futimes",
      "glob",
      "lchmod",
      "lchown",
      "lutimes",
      "link",
      "lstat",
      "mkdir",
      "mkdtemp",
      "open",
      "opendir",
      "readdir",
      "readFile",
      "readlink",
      "realpath",
      "rename",
      "rm",
      "rmdir",
      "stat",
      "statfs",
      "symlink",
      "truncate",
      "unlink",
      "utimes",
      "writeFile"
    ].filter((key) => {
      return typeof fs4[key] === "function";
    });
    Object.assign(exports2, fs4);
    api.forEach((method2) => {
      exports2[method2] = u(fs4[method2]);
    });
    exports2.exists = function(filename, callback) {
      if (typeof callback === "function") {
        return fs4.exists(filename, callback);
      }
      return new Promise((resolve3) => {
        return fs4.exists(filename, resolve3);
      });
    };
    exports2.read = function(fd, buffer, offset, length, position, callback) {
      if (typeof callback === "function") {
        return fs4.read(fd, buffer, offset, length, position, callback);
      }
      return new Promise((resolve3, reject) => {
        fs4.read(fd, buffer, offset, length, position, (err, bytesRead, buffer2) => {
          if (err) return reject(err);
          resolve3({ bytesRead, buffer: buffer2 });
        });
      });
    };
    exports2.write = function(fd, buffer, ...args) {
      if (typeof args[args.length - 1] === "function") {
        return fs4.write(fd, buffer, ...args);
      }
      return new Promise((resolve3, reject) => {
        fs4.write(fd, buffer, ...args, (err, bytesWritten, buffer2) => {
          if (err) return reject(err);
          resolve3({ bytesWritten, buffer: buffer2 });
        });
      });
    };
    exports2.readv = function(fd, buffers, ...args) {
      if (typeof args[args.length - 1] === "function") {
        return fs4.readv(fd, buffers, ...args);
      }
      return new Promise((resolve3, reject) => {
        fs4.readv(fd, buffers, ...args, (err, bytesRead, buffers2) => {
          if (err) return reject(err);
          resolve3({ bytesRead, buffers: buffers2 });
        });
      });
    };
    exports2.writev = function(fd, buffers, ...args) {
      if (typeof args[args.length - 1] === "function") {
        return fs4.writev(fd, buffers, ...args);
      }
      return new Promise((resolve3, reject) => {
        fs4.writev(fd, buffers, ...args, (err, bytesWritten, buffers2) => {
          if (err) return reject(err);
          resolve3({ bytesWritten, buffers: buffers2 });
        });
      });
    };
    if (typeof fs4.realpath.native === "function") {
      exports2.realpath.native = u(fs4.realpath.native);
    } else {
      process.emitWarning(
        "fs.realpath.native is not a function. Is fs being monkey-patched?",
        "Warning",
        "fs-extra-WARN0003"
      );
    }
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/mkdirs/utils.js
var require_utils = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/mkdirs/utils.js"(exports2, module2) {
    "use strict";
    var path10 = require("path");
    module2.exports.checkPath = function checkPath(pth) {
      if (process.platform === "win32") {
        const pathHasInvalidWinCharacters = /[<>:"|?*]/.test(pth.replace(path10.parse(pth).root, ""));
        if (pathHasInvalidWinCharacters) {
          const error = new Error(`Path contains invalid characters: ${pth}`);
          error.code = "EINVAL";
          throw error;
        }
      }
    };
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/mkdirs/make-dir.js
var require_make_dir = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/mkdirs/make-dir.js"(exports2, module2) {
    "use strict";
    var fs4 = require_fs();
    var { checkPath } = require_utils();
    var getMode = (options2) => {
      const defaults = { mode: 511 };
      if (typeof options2 === "number") return options2;
      return { ...defaults, ...options2 }.mode;
    };
    module2.exports.makeDir = async (dir, options2) => {
      checkPath(dir);
      return fs4.mkdir(dir, {
        mode: getMode(options2),
        recursive: true
      });
    };
    module2.exports.makeDirSync = (dir, options2) => {
      checkPath(dir);
      return fs4.mkdirSync(dir, {
        mode: getMode(options2),
        recursive: true
      });
    };
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/mkdirs/index.js
var require_mkdirs = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/mkdirs/index.js"(exports2, module2) {
    "use strict";
    var u = require_universalify().fromPromise;
    var { makeDir: _makeDir, makeDirSync } = require_make_dir();
    var makeDir = u(_makeDir);
    module2.exports = {
      mkdirs: makeDir,
      mkdirsSync: makeDirSync,
      // alias
      mkdirp: makeDir,
      mkdirpSync: makeDirSync,
      ensureDir: makeDir,
      ensureDirSync: makeDirSync
    };
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/path-exists/index.js
var require_path_exists = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/path-exists/index.js"(exports2, module2) {
    "use strict";
    var u = require_universalify().fromPromise;
    var fs4 = require_fs();
    function pathExists(path10) {
      return fs4.access(path10).then(() => true).catch(() => false);
    }
    module2.exports = {
      pathExists: u(pathExists),
      pathExistsSync: fs4.existsSync
    };
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/util/utimes.js
var require_utimes = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/util/utimes.js"(exports2, module2) {
    "use strict";
    var fs4 = require_fs();
    var u = require_universalify().fromPromise;
    async function utimesMillis(path10, atime, mtime) {
      const fd = await fs4.open(path10, "r+");
      let closeErr = null;
      try {
        await fs4.futimes(fd, atime, mtime);
      } finally {
        try {
          await fs4.close(fd);
        } catch (e) {
          closeErr = e;
        }
      }
      if (closeErr) {
        throw closeErr;
      }
    }
    function utimesMillisSync(path10, atime, mtime) {
      const fd = fs4.openSync(path10, "r+");
      fs4.futimesSync(fd, atime, mtime);
      return fs4.closeSync(fd);
    }
    module2.exports = {
      utimesMillis: u(utimesMillis),
      utimesMillisSync
    };
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/util/stat.js
var require_stat = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/util/stat.js"(exports2, module2) {
    "use strict";
    var fs4 = require_fs();
    var path10 = require("path");
    var u = require_universalify().fromPromise;
    function getStats(src2, dest, opts) {
      const statFunc = opts.dereference ? (file2) => fs4.stat(file2, { bigint: true }) : (file2) => fs4.lstat(file2, { bigint: true });
      return Promise.all([
        statFunc(src2),
        statFunc(dest).catch((err) => {
          if (err.code === "ENOENT") return null;
          throw err;
        })
      ]).then(([srcStat, destStat]) => ({ srcStat, destStat }));
    }
    function getStatsSync(src2, dest, opts) {
      let destStat;
      const statFunc = opts.dereference ? (file2) => fs4.statSync(file2, { bigint: true }) : (file2) => fs4.lstatSync(file2, { bigint: true });
      const srcStat = statFunc(src2);
      try {
        destStat = statFunc(dest);
      } catch (err) {
        if (err.code === "ENOENT") return { srcStat, destStat: null };
        throw err;
      }
      return { srcStat, destStat };
    }
    async function checkPaths(src2, dest, funcName, opts) {
      const { srcStat, destStat } = await getStats(src2, dest, opts);
      if (destStat) {
        if (areIdentical(srcStat, destStat)) {
          const srcBaseName = path10.basename(src2);
          const destBaseName = path10.basename(dest);
          if (funcName === "move" && srcBaseName !== destBaseName && srcBaseName.toLowerCase() === destBaseName.toLowerCase()) {
            return { srcStat, destStat, isChangingCase: true };
          }
          throw new Error("Source and destination must not be the same.");
        }
        if (srcStat.isDirectory() && !destStat.isDirectory()) {
          throw new Error(`Cannot overwrite non-directory '${dest}' with directory '${src2}'.`);
        }
        if (!srcStat.isDirectory() && destStat.isDirectory()) {
          throw new Error(`Cannot overwrite directory '${dest}' with non-directory '${src2}'.`);
        }
      }
      if (srcStat.isDirectory() && isSrcSubdir(src2, dest)) {
        throw new Error(errMsg(src2, dest, funcName));
      }
      return { srcStat, destStat };
    }
    function checkPathsSync(src2, dest, funcName, opts) {
      const { srcStat, destStat } = getStatsSync(src2, dest, opts);
      if (destStat) {
        if (areIdentical(srcStat, destStat)) {
          const srcBaseName = path10.basename(src2);
          const destBaseName = path10.basename(dest);
          if (funcName === "move" && srcBaseName !== destBaseName && srcBaseName.toLowerCase() === destBaseName.toLowerCase()) {
            return { srcStat, destStat, isChangingCase: true };
          }
          throw new Error("Source and destination must not be the same.");
        }
        if (srcStat.isDirectory() && !destStat.isDirectory()) {
          throw new Error(`Cannot overwrite non-directory '${dest}' with directory '${src2}'.`);
        }
        if (!srcStat.isDirectory() && destStat.isDirectory()) {
          throw new Error(`Cannot overwrite directory '${dest}' with non-directory '${src2}'.`);
        }
      }
      if (srcStat.isDirectory() && isSrcSubdir(src2, dest)) {
        throw new Error(errMsg(src2, dest, funcName));
      }
      return { srcStat, destStat };
    }
    async function checkParentPaths(src2, srcStat, dest, funcName) {
      const srcParent = path10.resolve(path10.dirname(src2));
      const destParent = path10.resolve(path10.dirname(dest));
      if (destParent === srcParent || destParent === path10.parse(destParent).root) return;
      let destStat;
      try {
        destStat = await fs4.stat(destParent, { bigint: true });
      } catch (err) {
        if (err.code === "ENOENT") return;
        throw err;
      }
      if (areIdentical(srcStat, destStat)) {
        throw new Error(errMsg(src2, dest, funcName));
      }
      return checkParentPaths(src2, srcStat, destParent, funcName);
    }
    function checkParentPathsSync(src2, srcStat, dest, funcName) {
      const srcParent = path10.resolve(path10.dirname(src2));
      const destParent = path10.resolve(path10.dirname(dest));
      if (destParent === srcParent || destParent === path10.parse(destParent).root) return;
      let destStat;
      try {
        destStat = fs4.statSync(destParent, { bigint: true });
      } catch (err) {
        if (err.code === "ENOENT") return;
        throw err;
      }
      if (areIdentical(srcStat, destStat)) {
        throw new Error(errMsg(src2, dest, funcName));
      }
      return checkParentPathsSync(src2, srcStat, destParent, funcName);
    }
    function areIdentical(srcStat, destStat) {
      return destStat.ino && destStat.dev && destStat.ino === srcStat.ino && destStat.dev === srcStat.dev;
    }
    function isSrcSubdir(src2, dest) {
      const srcArr = path10.resolve(src2).split(path10.sep).filter((i) => i);
      const destArr = path10.resolve(dest).split(path10.sep).filter((i) => i);
      return srcArr.every((cur, i) => destArr[i] === cur);
    }
    function errMsg(src2, dest, funcName) {
      return `Cannot ${funcName} '${src2}' to a subdirectory of itself, '${dest}'.`;
    }
    module2.exports = {
      // checkPaths
      checkPaths: u(checkPaths),
      checkPathsSync,
      // checkParent
      checkParentPaths: u(checkParentPaths),
      checkParentPathsSync,
      // Misc
      isSrcSubdir,
      areIdentical
    };
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/copy/copy.js
var require_copy = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/copy/copy.js"(exports2, module2) {
    "use strict";
    var fs4 = require_fs();
    var path10 = require("path");
    var { mkdirs } = require_mkdirs();
    var { pathExists } = require_path_exists();
    var { utimesMillis } = require_utimes();
    var stat = require_stat();
    async function copy(src2, dest, opts = {}) {
      if (typeof opts === "function") {
        opts = { filter: opts };
      }
      opts.clobber = "clobber" in opts ? !!opts.clobber : true;
      opts.overwrite = "overwrite" in opts ? !!opts.overwrite : opts.clobber;
      if (opts.preserveTimestamps && process.arch === "ia32") {
        process.emitWarning(
          "Using the preserveTimestamps option in 32-bit node is not recommended;\n\n	see https://github.com/jprichardson/node-fs-extra/issues/269",
          "Warning",
          "fs-extra-WARN0001"
        );
      }
      const { srcStat, destStat } = await stat.checkPaths(src2, dest, "copy", opts);
      await stat.checkParentPaths(src2, srcStat, dest, "copy");
      const include = await runFilter(src2, dest, opts);
      if (!include) return;
      const destParent = path10.dirname(dest);
      const dirExists = await pathExists(destParent);
      if (!dirExists) {
        await mkdirs(destParent);
      }
      await getStatsAndPerformCopy(destStat, src2, dest, opts);
    }
    async function runFilter(src2, dest, opts) {
      if (!opts.filter) return true;
      return opts.filter(src2, dest);
    }
    async function getStatsAndPerformCopy(destStat, src2, dest, opts) {
      const statFn = opts.dereference ? fs4.stat : fs4.lstat;
      const srcStat = await statFn(src2);
      if (srcStat.isDirectory()) return onDir(srcStat, destStat, src2, dest, opts);
      if (srcStat.isFile() || srcStat.isCharacterDevice() || srcStat.isBlockDevice()) return onFile(srcStat, destStat, src2, dest, opts);
      if (srcStat.isSymbolicLink()) return onLink(destStat, src2, dest, opts);
      if (srcStat.isSocket()) throw new Error(`Cannot copy a socket file: ${src2}`);
      if (srcStat.isFIFO()) throw new Error(`Cannot copy a FIFO pipe: ${src2}`);
      throw new Error(`Unknown file: ${src2}`);
    }
    async function onFile(srcStat, destStat, src2, dest, opts) {
      if (!destStat) return copyFile(srcStat, src2, dest, opts);
      if (opts.overwrite) {
        await fs4.unlink(dest);
        return copyFile(srcStat, src2, dest, opts);
      }
      if (opts.errorOnExist) {
        throw new Error(`'${dest}' already exists`);
      }
    }
    async function copyFile(srcStat, src2, dest, opts) {
      await fs4.copyFile(src2, dest);
      if (opts.preserveTimestamps) {
        if (fileIsNotWritable(srcStat.mode)) {
          await makeFileWritable(dest, srcStat.mode);
        }
        const updatedSrcStat = await fs4.stat(src2);
        await utimesMillis(dest, updatedSrcStat.atime, updatedSrcStat.mtime);
      }
      return fs4.chmod(dest, srcStat.mode);
    }
    function fileIsNotWritable(srcMode) {
      return (srcMode & 128) === 0;
    }
    function makeFileWritable(dest, srcMode) {
      return fs4.chmod(dest, srcMode | 128);
    }
    async function onDir(srcStat, destStat, src2, dest, opts) {
      if (!destStat) {
        await fs4.mkdir(dest);
      }
      const promises2 = [];
      for await (const item2 of await fs4.opendir(src2)) {
        const srcItem = path10.join(src2, item2.name);
        const destItem = path10.join(dest, item2.name);
        promises2.push(
          runFilter(srcItem, destItem, opts).then((include) => {
            if (include) {
              return stat.checkPaths(srcItem, destItem, "copy", opts).then(({ destStat: destStat2 }) => {
                return getStatsAndPerformCopy(destStat2, srcItem, destItem, opts);
              });
            }
          })
        );
      }
      await Promise.all(promises2);
      if (!destStat) {
        await fs4.chmod(dest, srcStat.mode);
      }
    }
    async function onLink(destStat, src2, dest, opts) {
      let resolvedSrc = await fs4.readlink(src2);
      if (opts.dereference) {
        resolvedSrc = path10.resolve(process.cwd(), resolvedSrc);
      }
      if (!destStat) {
        return fs4.symlink(resolvedSrc, dest);
      }
      let resolvedDest = null;
      try {
        resolvedDest = await fs4.readlink(dest);
      } catch (e) {
        if (e.code === "EINVAL" || e.code === "UNKNOWN") return fs4.symlink(resolvedSrc, dest);
        throw e;
      }
      if (opts.dereference) {
        resolvedDest = path10.resolve(process.cwd(), resolvedDest);
      }
      if (stat.isSrcSubdir(resolvedSrc, resolvedDest)) {
        throw new Error(`Cannot copy '${resolvedSrc}' to a subdirectory of itself, '${resolvedDest}'.`);
      }
      if (stat.isSrcSubdir(resolvedDest, resolvedSrc)) {
        throw new Error(`Cannot overwrite '${resolvedDest}' with '${resolvedSrc}'.`);
      }
      await fs4.unlink(dest);
      return fs4.symlink(resolvedSrc, dest);
    }
    module2.exports = copy;
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/copy/copy-sync.js
var require_copy_sync = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/copy/copy-sync.js"(exports2, module2) {
    "use strict";
    var fs4 = require_graceful_fs();
    var path10 = require("path");
    var mkdirsSync = require_mkdirs().mkdirsSync;
    var utimesMillisSync = require_utimes().utimesMillisSync;
    var stat = require_stat();
    function copySync(src2, dest, opts) {
      if (typeof opts === "function") {
        opts = { filter: opts };
      }
      opts = opts || {};
      opts.clobber = "clobber" in opts ? !!opts.clobber : true;
      opts.overwrite = "overwrite" in opts ? !!opts.overwrite : opts.clobber;
      if (opts.preserveTimestamps && process.arch === "ia32") {
        process.emitWarning(
          "Using the preserveTimestamps option in 32-bit node is not recommended;\n\n	see https://github.com/jprichardson/node-fs-extra/issues/269",
          "Warning",
          "fs-extra-WARN0002"
        );
      }
      const { srcStat, destStat } = stat.checkPathsSync(src2, dest, "copy", opts);
      stat.checkParentPathsSync(src2, srcStat, dest, "copy");
      if (opts.filter && !opts.filter(src2, dest)) return;
      const destParent = path10.dirname(dest);
      if (!fs4.existsSync(destParent)) mkdirsSync(destParent);
      return getStats(destStat, src2, dest, opts);
    }
    function getStats(destStat, src2, dest, opts) {
      const statSync = opts.dereference ? fs4.statSync : fs4.lstatSync;
      const srcStat = statSync(src2);
      if (srcStat.isDirectory()) return onDir(srcStat, destStat, src2, dest, opts);
      else if (srcStat.isFile() || srcStat.isCharacterDevice() || srcStat.isBlockDevice()) return onFile(srcStat, destStat, src2, dest, opts);
      else if (srcStat.isSymbolicLink()) return onLink(destStat, src2, dest, opts);
      else if (srcStat.isSocket()) throw new Error(`Cannot copy a socket file: ${src2}`);
      else if (srcStat.isFIFO()) throw new Error(`Cannot copy a FIFO pipe: ${src2}`);
      throw new Error(`Unknown file: ${src2}`);
    }
    function onFile(srcStat, destStat, src2, dest, opts) {
      if (!destStat) return copyFile(srcStat, src2, dest, opts);
      return mayCopyFile(srcStat, src2, dest, opts);
    }
    function mayCopyFile(srcStat, src2, dest, opts) {
      if (opts.overwrite) {
        fs4.unlinkSync(dest);
        return copyFile(srcStat, src2, dest, opts);
      } else if (opts.errorOnExist) {
        throw new Error(`'${dest}' already exists`);
      }
    }
    function copyFile(srcStat, src2, dest, opts) {
      fs4.copyFileSync(src2, dest);
      if (opts.preserveTimestamps) handleTimestamps(srcStat.mode, src2, dest);
      return setDestMode(dest, srcStat.mode);
    }
    function handleTimestamps(srcMode, src2, dest) {
      if (fileIsNotWritable(srcMode)) makeFileWritable(dest, srcMode);
      return setDestTimestamps(src2, dest);
    }
    function fileIsNotWritable(srcMode) {
      return (srcMode & 128) === 0;
    }
    function makeFileWritable(dest, srcMode) {
      return setDestMode(dest, srcMode | 128);
    }
    function setDestMode(dest, srcMode) {
      return fs4.chmodSync(dest, srcMode);
    }
    function setDestTimestamps(src2, dest) {
      const updatedSrcStat = fs4.statSync(src2);
      return utimesMillisSync(dest, updatedSrcStat.atime, updatedSrcStat.mtime);
    }
    function onDir(srcStat, destStat, src2, dest, opts) {
      if (!destStat) return mkDirAndCopy(srcStat.mode, src2, dest, opts);
      return copyDir(src2, dest, opts);
    }
    function mkDirAndCopy(srcMode, src2, dest, opts) {
      fs4.mkdirSync(dest);
      copyDir(src2, dest, opts);
      return setDestMode(dest, srcMode);
    }
    function copyDir(src2, dest, opts) {
      const dir = fs4.opendirSync(src2);
      try {
        let dirent;
        while ((dirent = dir.readSync()) !== null) {
          copyDirItem(dirent.name, src2, dest, opts);
        }
      } finally {
        dir.closeSync();
      }
    }
    function copyDirItem(item2, src2, dest, opts) {
      const srcItem = path10.join(src2, item2);
      const destItem = path10.join(dest, item2);
      if (opts.filter && !opts.filter(srcItem, destItem)) return;
      const { destStat } = stat.checkPathsSync(srcItem, destItem, "copy", opts);
      return getStats(destStat, srcItem, destItem, opts);
    }
    function onLink(destStat, src2, dest, opts) {
      let resolvedSrc = fs4.readlinkSync(src2);
      if (opts.dereference) {
        resolvedSrc = path10.resolve(process.cwd(), resolvedSrc);
      }
      if (!destStat) {
        return fs4.symlinkSync(resolvedSrc, dest);
      } else {
        let resolvedDest;
        try {
          resolvedDest = fs4.readlinkSync(dest);
        } catch (err) {
          if (err.code === "EINVAL" || err.code === "UNKNOWN") return fs4.symlinkSync(resolvedSrc, dest);
          throw err;
        }
        if (opts.dereference) {
          resolvedDest = path10.resolve(process.cwd(), resolvedDest);
        }
        if (stat.isSrcSubdir(resolvedSrc, resolvedDest)) {
          throw new Error(`Cannot copy '${resolvedSrc}' to a subdirectory of itself, '${resolvedDest}'.`);
        }
        if (stat.isSrcSubdir(resolvedDest, resolvedSrc)) {
          throw new Error(`Cannot overwrite '${resolvedDest}' with '${resolvedSrc}'.`);
        }
        return copyLink(resolvedSrc, dest);
      }
    }
    function copyLink(resolvedSrc, dest) {
      fs4.unlinkSync(dest);
      return fs4.symlinkSync(resolvedSrc, dest);
    }
    module2.exports = copySync;
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/copy/index.js
var require_copy2 = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/copy/index.js"(exports2, module2) {
    "use strict";
    var u = require_universalify().fromPromise;
    module2.exports = {
      copy: u(require_copy()),
      copySync: require_copy_sync()
    };
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/remove/index.js
var require_remove = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/remove/index.js"(exports2, module2) {
    "use strict";
    var fs4 = require_graceful_fs();
    var u = require_universalify().fromCallback;
    function remove2(path10, callback) {
      fs4.rm(path10, { recursive: true, force: true }, callback);
    }
    function removeSync(path10) {
      fs4.rmSync(path10, { recursive: true, force: true });
    }
    module2.exports = {
      remove: u(remove2),
      removeSync
    };
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/empty/index.js
var require_empty = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/empty/index.js"(exports2, module2) {
    "use strict";
    var u = require_universalify().fromPromise;
    var fs4 = require_fs();
    var path10 = require("path");
    var mkdir = require_mkdirs();
    var remove2 = require_remove();
    var emptyDir = u(async function emptyDir2(dir) {
      let items;
      try {
        items = await fs4.readdir(dir);
      } catch {
        return mkdir.mkdirs(dir);
      }
      return Promise.all(items.map((item2) => remove2.remove(path10.join(dir, item2))));
    });
    function emptyDirSync(dir) {
      let items;
      try {
        items = fs4.readdirSync(dir);
      } catch {
        return mkdir.mkdirsSync(dir);
      }
      items.forEach((item2) => {
        item2 = path10.join(dir, item2);
        remove2.removeSync(item2);
      });
    }
    module2.exports = {
      emptyDirSync,
      emptydirSync: emptyDirSync,
      emptyDir,
      emptydir: emptyDir
    };
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/ensure/file.js
var require_file = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/ensure/file.js"(exports2, module2) {
    "use strict";
    var u = require_universalify().fromPromise;
    var path10 = require("path");
    var fs4 = require_fs();
    var mkdir = require_mkdirs();
    async function createFile(file2) {
      let stats;
      try {
        stats = await fs4.stat(file2);
      } catch {
      }
      if (stats && stats.isFile()) return;
      const dir = path10.dirname(file2);
      let dirStats = null;
      try {
        dirStats = await fs4.stat(dir);
      } catch (err) {
        if (err.code === "ENOENT") {
          await mkdir.mkdirs(dir);
          await fs4.writeFile(file2, "");
          return;
        } else {
          throw err;
        }
      }
      if (dirStats.isDirectory()) {
        await fs4.writeFile(file2, "");
      } else {
        await fs4.readdir(dir);
      }
    }
    function createFileSync(file2) {
      let stats;
      try {
        stats = fs4.statSync(file2);
      } catch {
      }
      if (stats && stats.isFile()) return;
      const dir = path10.dirname(file2);
      try {
        if (!fs4.statSync(dir).isDirectory()) {
          fs4.readdirSync(dir);
        }
      } catch (err) {
        if (err && err.code === "ENOENT") mkdir.mkdirsSync(dir);
        else throw err;
      }
      fs4.writeFileSync(file2, "");
    }
    module2.exports = {
      createFile: u(createFile),
      createFileSync
    };
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/ensure/link.js
var require_link = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/ensure/link.js"(exports2, module2) {
    "use strict";
    var u = require_universalify().fromPromise;
    var path10 = require("path");
    var fs4 = require_fs();
    var mkdir = require_mkdirs();
    var { pathExists } = require_path_exists();
    var { areIdentical } = require_stat();
    async function createLink(srcpath, dstpath) {
      let dstStat;
      try {
        dstStat = await fs4.lstat(dstpath);
      } catch {
      }
      let srcStat;
      try {
        srcStat = await fs4.lstat(srcpath);
      } catch (err) {
        err.message = err.message.replace("lstat", "ensureLink");
        throw err;
      }
      if (dstStat && areIdentical(srcStat, dstStat)) return;
      const dir = path10.dirname(dstpath);
      const dirExists = await pathExists(dir);
      if (!dirExists) {
        await mkdir.mkdirs(dir);
      }
      await fs4.link(srcpath, dstpath);
    }
    function createLinkSync(srcpath, dstpath) {
      let dstStat;
      try {
        dstStat = fs4.lstatSync(dstpath);
      } catch {
      }
      try {
        const srcStat = fs4.lstatSync(srcpath);
        if (dstStat && areIdentical(srcStat, dstStat)) return;
      } catch (err) {
        err.message = err.message.replace("lstat", "ensureLink");
        throw err;
      }
      const dir = path10.dirname(dstpath);
      const dirExists = fs4.existsSync(dir);
      if (dirExists) return fs4.linkSync(srcpath, dstpath);
      mkdir.mkdirsSync(dir);
      return fs4.linkSync(srcpath, dstpath);
    }
    module2.exports = {
      createLink: u(createLink),
      createLinkSync
    };
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/ensure/symlink-paths.js
var require_symlink_paths = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/ensure/symlink-paths.js"(exports2, module2) {
    "use strict";
    var path10 = require("path");
    var fs4 = require_fs();
    var { pathExists } = require_path_exists();
    var u = require_universalify().fromPromise;
    async function symlinkPaths(srcpath, dstpath) {
      if (path10.isAbsolute(srcpath)) {
        try {
          await fs4.lstat(srcpath);
        } catch (err) {
          err.message = err.message.replace("lstat", "ensureSymlink");
          throw err;
        }
        return {
          toCwd: srcpath,
          toDst: srcpath
        };
      }
      const dstdir = path10.dirname(dstpath);
      const relativeToDst = path10.join(dstdir, srcpath);
      const exists = await pathExists(relativeToDst);
      if (exists) {
        return {
          toCwd: relativeToDst,
          toDst: srcpath
        };
      }
      try {
        await fs4.lstat(srcpath);
      } catch (err) {
        err.message = err.message.replace("lstat", "ensureSymlink");
        throw err;
      }
      return {
        toCwd: srcpath,
        toDst: path10.relative(dstdir, srcpath)
      };
    }
    function symlinkPathsSync(srcpath, dstpath) {
      if (path10.isAbsolute(srcpath)) {
        const exists2 = fs4.existsSync(srcpath);
        if (!exists2) throw new Error("absolute srcpath does not exist");
        return {
          toCwd: srcpath,
          toDst: srcpath
        };
      }
      const dstdir = path10.dirname(dstpath);
      const relativeToDst = path10.join(dstdir, srcpath);
      const exists = fs4.existsSync(relativeToDst);
      if (exists) {
        return {
          toCwd: relativeToDst,
          toDst: srcpath
        };
      }
      const srcExists = fs4.existsSync(srcpath);
      if (!srcExists) throw new Error("relative srcpath does not exist");
      return {
        toCwd: srcpath,
        toDst: path10.relative(dstdir, srcpath)
      };
    }
    module2.exports = {
      symlinkPaths: u(symlinkPaths),
      symlinkPathsSync
    };
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/ensure/symlink-type.js
var require_symlink_type = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/ensure/symlink-type.js"(exports2, module2) {
    "use strict";
    var fs4 = require_fs();
    var u = require_universalify().fromPromise;
    async function symlinkType(srcpath, type) {
      if (type) return type;
      let stats;
      try {
        stats = await fs4.lstat(srcpath);
      } catch {
        return "file";
      }
      return stats && stats.isDirectory() ? "dir" : "file";
    }
    function symlinkTypeSync(srcpath, type) {
      if (type) return type;
      let stats;
      try {
        stats = fs4.lstatSync(srcpath);
      } catch {
        return "file";
      }
      return stats && stats.isDirectory() ? "dir" : "file";
    }
    module2.exports = {
      symlinkType: u(symlinkType),
      symlinkTypeSync
    };
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/ensure/symlink.js
var require_symlink = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/ensure/symlink.js"(exports2, module2) {
    "use strict";
    var u = require_universalify().fromPromise;
    var path10 = require("path");
    var fs4 = require_fs();
    var { mkdirs, mkdirsSync } = require_mkdirs();
    var { symlinkPaths, symlinkPathsSync } = require_symlink_paths();
    var { symlinkType, symlinkTypeSync } = require_symlink_type();
    var { pathExists } = require_path_exists();
    var { areIdentical } = require_stat();
    async function createSymlink(srcpath, dstpath, type) {
      let stats;
      try {
        stats = await fs4.lstat(dstpath);
      } catch {
      }
      if (stats && stats.isSymbolicLink()) {
        const [srcStat, dstStat] = await Promise.all([
          fs4.stat(srcpath),
          fs4.stat(dstpath)
        ]);
        if (areIdentical(srcStat, dstStat)) return;
      }
      const relative = await symlinkPaths(srcpath, dstpath);
      srcpath = relative.toDst;
      const toType = await symlinkType(relative.toCwd, type);
      const dir = path10.dirname(dstpath);
      if (!await pathExists(dir)) {
        await mkdirs(dir);
      }
      return fs4.symlink(srcpath, dstpath, toType);
    }
    function createSymlinkSync(srcpath, dstpath, type) {
      let stats;
      try {
        stats = fs4.lstatSync(dstpath);
      } catch {
      }
      if (stats && stats.isSymbolicLink()) {
        const srcStat = fs4.statSync(srcpath);
        const dstStat = fs4.statSync(dstpath);
        if (areIdentical(srcStat, dstStat)) return;
      }
      const relative = symlinkPathsSync(srcpath, dstpath);
      srcpath = relative.toDst;
      type = symlinkTypeSync(relative.toCwd, type);
      const dir = path10.dirname(dstpath);
      const exists = fs4.existsSync(dir);
      if (exists) return fs4.symlinkSync(srcpath, dstpath, type);
      mkdirsSync(dir);
      return fs4.symlinkSync(srcpath, dstpath, type);
    }
    module2.exports = {
      createSymlink: u(createSymlink),
      createSymlinkSync
    };
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/ensure/index.js
var require_ensure = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/ensure/index.js"(exports2, module2) {
    "use strict";
    var { createFile, createFileSync } = require_file();
    var { createLink, createLinkSync } = require_link();
    var { createSymlink, createSymlinkSync } = require_symlink();
    module2.exports = {
      // file
      createFile,
      createFileSync,
      ensureFile: createFile,
      ensureFileSync: createFileSync,
      // link
      createLink,
      createLinkSync,
      ensureLink: createLink,
      ensureLinkSync: createLinkSync,
      // symlink
      createSymlink,
      createSymlinkSync,
      ensureSymlink: createSymlink,
      ensureSymlinkSync: createSymlinkSync
    };
  }
});

// ../../node_modules/.pnpm/jsonfile@6.1.0/node_modules/jsonfile/utils.js
var require_utils2 = __commonJS({
  "../../node_modules/.pnpm/jsonfile@6.1.0/node_modules/jsonfile/utils.js"(exports2, module2) {
    "use strict";
    function stringify2(obj, { EOL = "\n", finalEOL = true, replacer = null, spaces } = {}) {
      const EOF = finalEOL ? EOL : "";
      const str = JSON.stringify(obj, replacer, spaces);
      return str.replace(/\n/g, EOL) + EOF;
    }
    function stripBom(content) {
      if (Buffer.isBuffer(content)) content = content.toString("utf8");
      return content.replace(/^\uFEFF/, "");
    }
    module2.exports = { stringify: stringify2, stripBom };
  }
});

// ../../node_modules/.pnpm/jsonfile@6.1.0/node_modules/jsonfile/index.js
var require_jsonfile = __commonJS({
  "../../node_modules/.pnpm/jsonfile@6.1.0/node_modules/jsonfile/index.js"(exports2, module2) {
    "use strict";
    var _fs;
    try {
      _fs = require_graceful_fs();
    } catch (_) {
      _fs = require("fs");
    }
    var universalify = require_universalify();
    var { stringify: stringify2, stripBom } = require_utils2();
    async function _readFile(file2, options2 = {}) {
      if (typeof options2 === "string") {
        options2 = { encoding: options2 };
      }
      const fs4 = options2.fs || _fs;
      const shouldThrow = "throws" in options2 ? options2.throws : true;
      let data = await universalify.fromCallback(fs4.readFile)(file2, options2);
      data = stripBom(data);
      let obj;
      try {
        obj = JSON.parse(data, options2 ? options2.reviver : null);
      } catch (err) {
        if (shouldThrow) {
          err.message = `${file2}: ${err.message}`;
          throw err;
        } else {
          return null;
        }
      }
      return obj;
    }
    var readFile = universalify.fromPromise(_readFile);
    function readFileSync(file2, options2 = {}) {
      if (typeof options2 === "string") {
        options2 = { encoding: options2 };
      }
      const fs4 = options2.fs || _fs;
      const shouldThrow = "throws" in options2 ? options2.throws : true;
      try {
        let content = fs4.readFileSync(file2, options2);
        content = stripBom(content);
        return JSON.parse(content, options2.reviver);
      } catch (err) {
        if (shouldThrow) {
          err.message = `${file2}: ${err.message}`;
          throw err;
        } else {
          return null;
        }
      }
    }
    async function _writeFile(file2, obj, options2 = {}) {
      const fs4 = options2.fs || _fs;
      const str = stringify2(obj, options2);
      await universalify.fromCallback(fs4.writeFile)(file2, str, options2);
    }
    var writeFile = universalify.fromPromise(_writeFile);
    function writeFileSync(file2, obj, options2 = {}) {
      const fs4 = options2.fs || _fs;
      const str = stringify2(obj, options2);
      return fs4.writeFileSync(file2, str, options2);
    }
    var jsonfile = {
      readFile,
      readFileSync,
      writeFile,
      writeFileSync
    };
    module2.exports = jsonfile;
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/json/jsonfile.js
var require_jsonfile2 = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/json/jsonfile.js"(exports2, module2) {
    "use strict";
    var jsonFile = require_jsonfile();
    module2.exports = {
      // jsonfile exports
      readJson: jsonFile.readFile,
      readJsonSync: jsonFile.readFileSync,
      writeJson: jsonFile.writeFile,
      writeJsonSync: jsonFile.writeFileSync
    };
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/output-file/index.js
var require_output_file = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/output-file/index.js"(exports2, module2) {
    "use strict";
    var u = require_universalify().fromPromise;
    var fs4 = require_fs();
    var path10 = require("path");
    var mkdir = require_mkdirs();
    var pathExists = require_path_exists().pathExists;
    async function outputFile(file2, data, encoding = "utf-8") {
      const dir = path10.dirname(file2);
      if (!await pathExists(dir)) {
        await mkdir.mkdirs(dir);
      }
      return fs4.writeFile(file2, data, encoding);
    }
    function outputFileSync(file2, ...args) {
      const dir = path10.dirname(file2);
      if (!fs4.existsSync(dir)) {
        mkdir.mkdirsSync(dir);
      }
      fs4.writeFileSync(file2, ...args);
    }
    module2.exports = {
      outputFile: u(outputFile),
      outputFileSync
    };
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/json/output-json.js
var require_output_json = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/json/output-json.js"(exports2, module2) {
    "use strict";
    var { stringify: stringify2 } = require_utils2();
    var { outputFile } = require_output_file();
    async function outputJson(file2, data, options2 = {}) {
      const str = stringify2(data, options2);
      await outputFile(file2, str, options2);
    }
    module2.exports = outputJson;
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/json/output-json-sync.js
var require_output_json_sync = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/json/output-json-sync.js"(exports2, module2) {
    "use strict";
    var { stringify: stringify2 } = require_utils2();
    var { outputFileSync } = require_output_file();
    function outputJsonSync(file2, data, options2) {
      const str = stringify2(data, options2);
      outputFileSync(file2, str, options2);
    }
    module2.exports = outputJsonSync;
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/json/index.js
var require_json = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/json/index.js"(exports2, module2) {
    "use strict";
    var u = require_universalify().fromPromise;
    var jsonFile = require_jsonfile2();
    jsonFile.outputJson = u(require_output_json());
    jsonFile.outputJsonSync = require_output_json_sync();
    jsonFile.outputJSON = jsonFile.outputJson;
    jsonFile.outputJSONSync = jsonFile.outputJsonSync;
    jsonFile.writeJSON = jsonFile.writeJson;
    jsonFile.writeJSONSync = jsonFile.writeJsonSync;
    jsonFile.readJSON = jsonFile.readJson;
    jsonFile.readJSONSync = jsonFile.readJsonSync;
    module2.exports = jsonFile;
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/move/move.js
var require_move = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/move/move.js"(exports2, module2) {
    "use strict";
    var fs4 = require_fs();
    var path10 = require("path");
    var { copy } = require_copy2();
    var { remove: remove2 } = require_remove();
    var { mkdirp } = require_mkdirs();
    var { pathExists } = require_path_exists();
    var stat = require_stat();
    async function move(src2, dest, opts = {}) {
      const overwrite = opts.overwrite || opts.clobber || false;
      const { srcStat, isChangingCase = false } = await stat.checkPaths(src2, dest, "move", opts);
      await stat.checkParentPaths(src2, srcStat, dest, "move");
      const destParent = path10.dirname(dest);
      const parsedParentPath = path10.parse(destParent);
      if (parsedParentPath.root !== destParent) {
        await mkdirp(destParent);
      }
      return doRename(src2, dest, overwrite, isChangingCase);
    }
    async function doRename(src2, dest, overwrite, isChangingCase) {
      if (!isChangingCase) {
        if (overwrite) {
          await remove2(dest);
        } else if (await pathExists(dest)) {
          throw new Error("dest already exists.");
        }
      }
      try {
        await fs4.rename(src2, dest);
      } catch (err) {
        if (err.code !== "EXDEV") {
          throw err;
        }
        await moveAcrossDevice(src2, dest, overwrite);
      }
    }
    async function moveAcrossDevice(src2, dest, overwrite) {
      const opts = {
        overwrite,
        errorOnExist: true,
        preserveTimestamps: true
      };
      await copy(src2, dest, opts);
      return remove2(src2);
    }
    module2.exports = move;
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/move/move-sync.js
var require_move_sync = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/move/move-sync.js"(exports2, module2) {
    "use strict";
    var fs4 = require_graceful_fs();
    var path10 = require("path");
    var copySync = require_copy2().copySync;
    var removeSync = require_remove().removeSync;
    var mkdirpSync = require_mkdirs().mkdirpSync;
    var stat = require_stat();
    function moveSync(src2, dest, opts) {
      opts = opts || {};
      const overwrite = opts.overwrite || opts.clobber || false;
      const { srcStat, isChangingCase = false } = stat.checkPathsSync(src2, dest, "move", opts);
      stat.checkParentPathsSync(src2, srcStat, dest, "move");
      if (!isParentRoot(dest)) mkdirpSync(path10.dirname(dest));
      return doRename(src2, dest, overwrite, isChangingCase);
    }
    function isParentRoot(dest) {
      const parent = path10.dirname(dest);
      const parsedPath = path10.parse(parent);
      return parsedPath.root === parent;
    }
    function doRename(src2, dest, overwrite, isChangingCase) {
      if (isChangingCase) return rename(src2, dest, overwrite);
      if (overwrite) {
        removeSync(dest);
        return rename(src2, dest, overwrite);
      }
      if (fs4.existsSync(dest)) throw new Error("dest already exists.");
      return rename(src2, dest, overwrite);
    }
    function rename(src2, dest, overwrite) {
      try {
        fs4.renameSync(src2, dest);
      } catch (err) {
        if (err.code !== "EXDEV") throw err;
        return moveAcrossDevice(src2, dest, overwrite);
      }
    }
    function moveAcrossDevice(src2, dest, overwrite) {
      const opts = {
        overwrite,
        errorOnExist: true,
        preserveTimestamps: true
      };
      copySync(src2, dest, opts);
      return removeSync(src2);
    }
    module2.exports = moveSync;
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/move/index.js
var require_move2 = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/move/index.js"(exports2, module2) {
    "use strict";
    var u = require_universalify().fromPromise;
    module2.exports = {
      move: u(require_move()),
      moveSync: require_move_sync()
    };
  }
});

// ../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/index.js
var require_lib = __commonJS({
  "../../node_modules/.pnpm/fs-extra@11.3.0/node_modules/fs-extra/lib/index.js"(exports2, module2) {
    "use strict";
    module2.exports = {
      // Export promiseified graceful-fs:
      ...require_fs(),
      // Export extra methods:
      ...require_copy2(),
      ...require_empty(),
      ...require_ensure(),
      ...require_json(),
      ...require_mkdirs(),
      ...require_move2(),
      ...require_output_file(),
      ...require_path_exists(),
      ...require_remove()
    };
  }
});

// ../../node_modules/.pnpm/@prisma+engines-version@7.5.0-10.94a226be1cf2967af2541cca5529f0f7ba866919/node_modules/@prisma/engines-version/package.json
var require_package = __commonJS({
  "../../node_modules/.pnpm/@prisma+engines-version@7.5.0-10.94a226be1cf2967af2541cca5529f0f7ba866919/node_modules/@prisma/engines-version/package.json"(exports2, module2) {
    module2.exports = {
      name: "@prisma/engines-version",
      version: "7.5.0-10.94a226be1cf2967af2541cca5529f0f7ba866919",
      main: "index.js",
      types: "index.d.ts",
      license: "Apache-2.0",
      author: "Tim Suchanek <suchanek@prisma.io>",
      prisma: {
        enginesVersion: "94a226be1cf2967af2541cca5529f0f7ba866919"
      },
      repository: {
        type: "git",
        url: "https://github.com/prisma/engines-wrapper.git",
        directory: "packages/engines-version"
      },
      devDependencies: {
        "@types/node": "18.19.76",
        typescript: "4.9.5"
      },
      files: [
        "index.js",
        "index.d.ts"
      ],
      scripts: {
        build: "tsc -d"
      }
    };
  }
});

// ../../node_modules/.pnpm/@prisma+engines-version@7.5.0-10.94a226be1cf2967af2541cca5529f0f7ba866919/node_modules/@prisma/engines-version/index.js
var require_engines_version = __commonJS({
  "../../node_modules/.pnpm/@prisma+engines-version@7.5.0-10.94a226be1cf2967af2541cca5529f0f7ba866919/node_modules/@prisma/engines-version/index.js"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", { value: true });
    exports2.enginesVersion = void 0;
    exports2.enginesVersion = require_package().prisma.enginesVersion;
  }
});

// ../../node_modules/.pnpm/indent-string@4.0.0/node_modules/indent-string/index.js
var require_indent_string = __commonJS({
  "../../node_modules/.pnpm/indent-string@4.0.0/node_modules/indent-string/index.js"(exports2, module2) {
    "use strict";
    module2.exports = (string, count = 1, options2) => {
      options2 = {
        indent: " ",
        includeEmptyLines: false,
        ...options2
      };
      if (typeof string !== "string") {
        throw new TypeError(
          `Expected \`input\` to be a \`string\`, got \`${typeof string}\``
        );
      }
      if (typeof count !== "number") {
        throw new TypeError(
          `Expected \`count\` to be a \`number\`, got \`${typeof count}\``
        );
      }
      if (typeof options2.indent !== "string") {
        throw new TypeError(
          `Expected \`options.indent\` to be a \`string\`, got \`${typeof options2.indent}\``
        );
      }
      if (count === 0) {
        return string;
      }
      const regex2 = options2.includeEmptyLines ? /^/gm : /^(?!\s*$)/gm;
      return string.replace(regex2, options2.indent.repeat(count));
    };
  }
});

// ../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/homedir.js
var require_homedir = __commonJS({
  "../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/homedir.js"(exports2, module2) {
    "use strict";
    var os2 = require("os");
    module2.exports = os2.homedir || function homedir() {
      var home2 = process.env.HOME;
      var user = process.env.LOGNAME || process.env.USER || process.env.LNAME || process.env.USERNAME;
      if (process.platform === "win32") {
        return process.env.USERPROFILE || process.env.HOMEDRIVE + process.env.HOMEPATH || home2 || null;
      }
      if (process.platform === "darwin") {
        return home2 || (user ? "/Users/" + user : null);
      }
      if (process.platform === "linux") {
        return home2 || (process.getuid() === 0 ? "/root" : user ? "/home/" + user : null);
      }
      return home2 || null;
    };
  }
});

// ../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/caller.js
var require_caller = __commonJS({
  "../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/caller.js"(exports2, module2) {
    "use strict";
    module2.exports = function() {
      var origPrepareStackTrace = Error.prepareStackTrace;
      Error.prepareStackTrace = function(_, stack2) {
        return stack2;
      };
      var stack = new Error().stack;
      Error.prepareStackTrace = origPrepareStackTrace;
      return stack[2].getFileName();
    };
  }
});

// ../../node_modules/.pnpm/path-parse@1.0.7/node_modules/path-parse/index.js
var require_path_parse = __commonJS({
  "../../node_modules/.pnpm/path-parse@1.0.7/node_modules/path-parse/index.js"(exports2, module2) {
    "use strict";
    var isWindows2 = process.platform === "win32";
    var splitWindowsRe = /^(((?:[a-zA-Z]:|[\\\/]{2}[^\\\/]+[\\\/]+[^\\\/]+)?[\\\/]?)(?:[^\\\/]*[\\\/])*)((\.{1,2}|[^\\\/]+?|)(\.[^.\/\\]*|))[\\\/]*$/;
    var win322 = {};
    function win32SplitPath(filename) {
      return splitWindowsRe.exec(filename).slice(1);
    }
    win322.parse = function(pathString) {
      if (typeof pathString !== "string") {
        throw new TypeError(
          "Parameter 'pathString' must be a string, not " + typeof pathString
        );
      }
      var allParts = win32SplitPath(pathString);
      if (!allParts || allParts.length !== 5) {
        throw new TypeError("Invalid path '" + pathString + "'");
      }
      return {
        root: allParts[1],
        dir: allParts[0] === allParts[1] ? allParts[0] : allParts[0].slice(0, -1),
        base: allParts[2],
        ext: allParts[4],
        name: allParts[3]
      };
    };
    var splitPathRe = /^((\/?)(?:[^\/]*\/)*)((\.{1,2}|[^\/]+?|)(\.[^.\/]*|))[\/]*$/;
    var posix2 = {};
    function posixSplitPath(filename) {
      return splitPathRe.exec(filename).slice(1);
    }
    posix2.parse = function(pathString) {
      if (typeof pathString !== "string") {
        throw new TypeError(
          "Parameter 'pathString' must be a string, not " + typeof pathString
        );
      }
      var allParts = posixSplitPath(pathString);
      if (!allParts || allParts.length !== 5) {
        throw new TypeError("Invalid path '" + pathString + "'");
      }
      return {
        root: allParts[1],
        dir: allParts[0].slice(0, -1),
        base: allParts[2],
        ext: allParts[4],
        name: allParts[3]
      };
    };
    if (isWindows2)
      module2.exports = win322.parse;
    else
      module2.exports = posix2.parse;
    module2.exports.posix = posix2.parse;
    module2.exports.win32 = win322.parse;
  }
});

// ../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/node-modules-paths.js
var require_node_modules_paths = __commonJS({
  "../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/node-modules-paths.js"(exports2, module2) {
    "use strict";
    var path10 = require("path");
    var parse2 = path10.parse || require_path_parse();
    var getNodeModulesDirs = function getNodeModulesDirs2(absoluteStart, modules) {
      var prefix = "/";
      if (/^([A-Za-z]:)/.test(absoluteStart)) {
        prefix = "";
      } else if (/^\\\\/.test(absoluteStart)) {
        prefix = "\\\\";
      }
      var paths2 = [absoluteStart];
      var parsed = parse2(absoluteStart);
      while (parsed.dir !== paths2[paths2.length - 1]) {
        paths2.push(parsed.dir);
        parsed = parse2(parsed.dir);
      }
      return paths2.reduce(function(dirs, aPath) {
        return dirs.concat(modules.map(function(moduleDir) {
          return path10.resolve(prefix, aPath, moduleDir);
        }));
      }, []);
    };
    module2.exports = function nodeModulesPaths(start, opts, request) {
      var modules = opts && opts.moduleDirectory ? [].concat(opts.moduleDirectory) : ["node_modules"];
      if (opts && typeof opts.paths === "function") {
        return opts.paths(
          request,
          start,
          function() {
            return getNodeModulesDirs(start, modules);
          },
          opts
        );
      }
      var dirs = getNodeModulesDirs(start, modules);
      return opts && opts.paths ? dirs.concat(opts.paths) : dirs;
    };
  }
});

// ../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/normalize-options.js
var require_normalize_options = __commonJS({
  "../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/normalize-options.js"(exports2, module2) {
    "use strict";
    module2.exports = function(x, opts) {
      return opts || {};
    };
  }
});

// ../../node_modules/.pnpm/function-bind@1.1.2/node_modules/function-bind/implementation.js
var require_implementation = __commonJS({
  "../../node_modules/.pnpm/function-bind@1.1.2/node_modules/function-bind/implementation.js"(exports2, module2) {
    "use strict";
    var ERROR_MESSAGE = "Function.prototype.bind called on incompatible ";
    var toStr = Object.prototype.toString;
    var max = Math.max;
    var funcType = "[object Function]";
    var concatty = function concatty2(a, b) {
      var arr = [];
      for (var i = 0; i < a.length; i += 1) {
        arr[i] = a[i];
      }
      for (var j = 0; j < b.length; j += 1) {
        arr[j + a.length] = b[j];
      }
      return arr;
    };
    var slicy = function slicy2(arrLike, offset) {
      var arr = [];
      for (var i = offset || 0, j = 0; i < arrLike.length; i += 1, j += 1) {
        arr[j] = arrLike[i];
      }
      return arr;
    };
    var joiny = function(arr, joiner) {
      var str = "";
      for (var i = 0; i < arr.length; i += 1) {
        str += arr[i];
        if (i + 1 < arr.length) {
          str += joiner;
        }
      }
      return str;
    };
    module2.exports = function bind(that) {
      var target = this;
      if (typeof target !== "function" || toStr.apply(target) !== funcType) {
        throw new TypeError(ERROR_MESSAGE + target);
      }
      var args = slicy(arguments, 1);
      var bound;
      var binder = function() {
        if (this instanceof bound) {
          var result = target.apply(
            this,
            concatty(args, arguments)
          );
          if (Object(result) === result) {
            return result;
          }
          return this;
        }
        return target.apply(
          that,
          concatty(args, arguments)
        );
      };
      var boundLength = max(0, target.length - args.length);
      var boundArgs = [];
      for (var i = 0; i < boundLength; i++) {
        boundArgs[i] = "$" + i;
      }
      bound = Function("binder", "return function (" + joiny(boundArgs, ",") + "){ return binder.apply(this,arguments); }")(binder);
      if (target.prototype) {
        var Empty = function Empty2() {
        };
        Empty.prototype = target.prototype;
        bound.prototype = new Empty();
        Empty.prototype = null;
      }
      return bound;
    };
  }
});

// ../../node_modules/.pnpm/function-bind@1.1.2/node_modules/function-bind/index.js
var require_function_bind = __commonJS({
  "../../node_modules/.pnpm/function-bind@1.1.2/node_modules/function-bind/index.js"(exports2, module2) {
    "use strict";
    var implementation = require_implementation();
    module2.exports = Function.prototype.bind || implementation;
  }
});

// ../../node_modules/.pnpm/hasown@2.0.2/node_modules/hasown/index.js
var require_hasown = __commonJS({
  "../../node_modules/.pnpm/hasown@2.0.2/node_modules/hasown/index.js"(exports2, module2) {
    "use strict";
    var call = Function.prototype.call;
    var $hasOwn = Object.prototype.hasOwnProperty;
    var bind = require_function_bind();
    module2.exports = bind.call(call, $hasOwn);
  }
});

// ../../node_modules/.pnpm/is-core-module@2.16.1/node_modules/is-core-module/core.json
var require_core = __commonJS({
  "../../node_modules/.pnpm/is-core-module@2.16.1/node_modules/is-core-module/core.json"(exports2, module2) {
    module2.exports = {
      assert: true,
      "node:assert": [">= 14.18 && < 15", ">= 16"],
      "assert/strict": ">= 15",
      "node:assert/strict": ">= 16",
      async_hooks: ">= 8",
      "node:async_hooks": [">= 14.18 && < 15", ">= 16"],
      buffer_ieee754: ">= 0.5 && < 0.9.7",
      buffer: true,
      "node:buffer": [">= 14.18 && < 15", ">= 16"],
      child_process: true,
      "node:child_process": [">= 14.18 && < 15", ">= 16"],
      cluster: ">= 0.5",
      "node:cluster": [">= 14.18 && < 15", ">= 16"],
      console: true,
      "node:console": [">= 14.18 && < 15", ">= 16"],
      constants: true,
      "node:constants": [">= 14.18 && < 15", ">= 16"],
      crypto: true,
      "node:crypto": [">= 14.18 && < 15", ">= 16"],
      _debug_agent: ">= 1 && < 8",
      _debugger: "< 8",
      dgram: true,
      "node:dgram": [">= 14.18 && < 15", ">= 16"],
      diagnostics_channel: [">= 14.17 && < 15", ">= 15.1"],
      "node:diagnostics_channel": [">= 14.18 && < 15", ">= 16"],
      dns: true,
      "node:dns": [">= 14.18 && < 15", ">= 16"],
      "dns/promises": ">= 15",
      "node:dns/promises": ">= 16",
      domain: ">= 0.7.12",
      "node:domain": [">= 14.18 && < 15", ">= 16"],
      events: true,
      "node:events": [">= 14.18 && < 15", ">= 16"],
      freelist: "< 6",
      fs: true,
      "node:fs": [">= 14.18 && < 15", ">= 16"],
      "fs/promises": [">= 10 && < 10.1", ">= 14"],
      "node:fs/promises": [">= 14.18 && < 15", ">= 16"],
      _http_agent: ">= 0.11.1",
      "node:_http_agent": [">= 14.18 && < 15", ">= 16"],
      _http_client: ">= 0.11.1",
      "node:_http_client": [">= 14.18 && < 15", ">= 16"],
      _http_common: ">= 0.11.1",
      "node:_http_common": [">= 14.18 && < 15", ">= 16"],
      _http_incoming: ">= 0.11.1",
      "node:_http_incoming": [">= 14.18 && < 15", ">= 16"],
      _http_outgoing: ">= 0.11.1",
      "node:_http_outgoing": [">= 14.18 && < 15", ">= 16"],
      _http_server: ">= 0.11.1",
      "node:_http_server": [">= 14.18 && < 15", ">= 16"],
      http: true,
      "node:http": [">= 14.18 && < 15", ">= 16"],
      http2: ">= 8.8",
      "node:http2": [">= 14.18 && < 15", ">= 16"],
      https: true,
      "node:https": [">= 14.18 && < 15", ">= 16"],
      inspector: ">= 8",
      "node:inspector": [">= 14.18 && < 15", ">= 16"],
      "inspector/promises": [">= 19"],
      "node:inspector/promises": [">= 19"],
      _linklist: "< 8",
      module: true,
      "node:module": [">= 14.18 && < 15", ">= 16"],
      net: true,
      "node:net": [">= 14.18 && < 15", ">= 16"],
      "node-inspect/lib/_inspect": ">= 7.6 && < 12",
      "node-inspect/lib/internal/inspect_client": ">= 7.6 && < 12",
      "node-inspect/lib/internal/inspect_repl": ">= 7.6 && < 12",
      os: true,
      "node:os": [">= 14.18 && < 15", ">= 16"],
      path: true,
      "node:path": [">= 14.18 && < 15", ">= 16"],
      "path/posix": ">= 15.3",
      "node:path/posix": ">= 16",
      "path/win32": ">= 15.3",
      "node:path/win32": ">= 16",
      perf_hooks: ">= 8.5",
      "node:perf_hooks": [">= 14.18 && < 15", ">= 16"],
      process: ">= 1",
      "node:process": [">= 14.18 && < 15", ">= 16"],
      punycode: ">= 0.5",
      "node:punycode": [">= 14.18 && < 15", ">= 16"],
      querystring: true,
      "node:querystring": [">= 14.18 && < 15", ">= 16"],
      readline: true,
      "node:readline": [">= 14.18 && < 15", ">= 16"],
      "readline/promises": ">= 17",
      "node:readline/promises": ">= 17",
      repl: true,
      "node:repl": [">= 14.18 && < 15", ">= 16"],
      "node:sea": [">= 20.12 && < 21", ">= 21.7"],
      smalloc: ">= 0.11.5 && < 3",
      "node:sqlite": [">= 22.13 && < 23", ">= 23.4"],
      _stream_duplex: ">= 0.9.4",
      "node:_stream_duplex": [">= 14.18 && < 15", ">= 16"],
      _stream_transform: ">= 0.9.4",
      "node:_stream_transform": [">= 14.18 && < 15", ">= 16"],
      _stream_wrap: ">= 1.4.1",
      "node:_stream_wrap": [">= 14.18 && < 15", ">= 16"],
      _stream_passthrough: ">= 0.9.4",
      "node:_stream_passthrough": [">= 14.18 && < 15", ">= 16"],
      _stream_readable: ">= 0.9.4",
      "node:_stream_readable": [">= 14.18 && < 15", ">= 16"],
      _stream_writable: ">= 0.9.4",
      "node:_stream_writable": [">= 14.18 && < 15", ">= 16"],
      stream: true,
      "node:stream": [">= 14.18 && < 15", ">= 16"],
      "stream/consumers": ">= 16.7",
      "node:stream/consumers": ">= 16.7",
      "stream/promises": ">= 15",
      "node:stream/promises": ">= 16",
      "stream/web": ">= 16.5",
      "node:stream/web": ">= 16.5",
      string_decoder: true,
      "node:string_decoder": [">= 14.18 && < 15", ">= 16"],
      sys: [">= 0.4 && < 0.7", ">= 0.8"],
      "node:sys": [">= 14.18 && < 15", ">= 16"],
      "test/reporters": ">= 19.9 && < 20.2",
      "node:test/reporters": [">= 18.17 && < 19", ">= 19.9", ">= 20"],
      "test/mock_loader": ">= 22.3 && < 22.7",
      "node:test/mock_loader": ">= 22.3 && < 22.7",
      "node:test": [">= 16.17 && < 17", ">= 18"],
      timers: true,
      "node:timers": [">= 14.18 && < 15", ">= 16"],
      "timers/promises": ">= 15",
      "node:timers/promises": ">= 16",
      _tls_common: ">= 0.11.13",
      "node:_tls_common": [">= 14.18 && < 15", ">= 16"],
      _tls_legacy: ">= 0.11.3 && < 10",
      _tls_wrap: ">= 0.11.3",
      "node:_tls_wrap": [">= 14.18 && < 15", ">= 16"],
      tls: true,
      "node:tls": [">= 14.18 && < 15", ">= 16"],
      trace_events: ">= 10",
      "node:trace_events": [">= 14.18 && < 15", ">= 16"],
      tty: true,
      "node:tty": [">= 14.18 && < 15", ">= 16"],
      url: true,
      "node:url": [">= 14.18 && < 15", ">= 16"],
      util: true,
      "node:util": [">= 14.18 && < 15", ">= 16"],
      "util/types": ">= 15.3",
      "node:util/types": ">= 16",
      "v8/tools/arguments": ">= 10 && < 12",
      "v8/tools/codemap": [">= 4.4 && < 5", ">= 5.2 && < 12"],
      "v8/tools/consarray": [">= 4.4 && < 5", ">= 5.2 && < 12"],
      "v8/tools/csvparser": [">= 4.4 && < 5", ">= 5.2 && < 12"],
      "v8/tools/logreader": [">= 4.4 && < 5", ">= 5.2 && < 12"],
      "v8/tools/profile_view": [">= 4.4 && < 5", ">= 5.2 && < 12"],
      "v8/tools/splaytree": [">= 4.4 && < 5", ">= 5.2 && < 12"],
      v8: ">= 1",
      "node:v8": [">= 14.18 && < 15", ">= 16"],
      vm: true,
      "node:vm": [">= 14.18 && < 15", ">= 16"],
      wasi: [">= 13.4 && < 13.5", ">= 18.17 && < 19", ">= 20"],
      "node:wasi": [">= 18.17 && < 19", ">= 20"],
      worker_threads: ">= 11.7",
      "node:worker_threads": [">= 14.18 && < 15", ">= 16"],
      zlib: ">= 0.5",
      "node:zlib": [">= 14.18 && < 15", ">= 16"]
    };
  }
});

// ../../node_modules/.pnpm/is-core-module@2.16.1/node_modules/is-core-module/index.js
var require_is_core_module = __commonJS({
  "../../node_modules/.pnpm/is-core-module@2.16.1/node_modules/is-core-module/index.js"(exports2, module2) {
    "use strict";
    var hasOwn = require_hasown();
    function specifierIncluded(current, specifier) {
      var nodeParts = current.split(".");
      var parts = specifier.split(" ");
      var op = parts.length > 1 ? parts[0] : "=";
      var versionParts = (parts.length > 1 ? parts[1] : parts[0]).split(".");
      for (var i = 0; i < 3; ++i) {
        var cur = parseInt(nodeParts[i] || 0, 10);
        var ver = parseInt(versionParts[i] || 0, 10);
        if (cur === ver) {
          continue;
        }
        if (op === "<") {
          return cur < ver;
        }
        if (op === ">=") {
          return cur >= ver;
        }
        return false;
      }
      return op === ">=";
    }
    function matchesRange(current, range) {
      var specifiers = range.split(/ ?&& ?/);
      if (specifiers.length === 0) {
        return false;
      }
      for (var i = 0; i < specifiers.length; ++i) {
        if (!specifierIncluded(current, specifiers[i])) {
          return false;
        }
      }
      return true;
    }
    function versionIncluded(nodeVersion, specifierValue) {
      if (typeof specifierValue === "boolean") {
        return specifierValue;
      }
      var current = typeof nodeVersion === "undefined" ? process.versions && process.versions.node : nodeVersion;
      if (typeof current !== "string") {
        throw new TypeError(typeof nodeVersion === "undefined" ? "Unable to determine current node version" : "If provided, a valid node version is required");
      }
      if (specifierValue && typeof specifierValue === "object") {
        for (var i = 0; i < specifierValue.length; ++i) {
          if (matchesRange(current, specifierValue[i])) {
            return true;
          }
        }
        return false;
      }
      return matchesRange(current, specifierValue);
    }
    var data = require_core();
    module2.exports = function isCore(x, nodeVersion) {
      return hasOwn(data, x) && versionIncluded(nodeVersion, data[x]);
    };
  }
});

// ../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/async.js
var require_async = __commonJS({
  "../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/async.js"(exports2, module2) {
    "use strict";
    var fs4 = require("fs");
    var getHomedir = require_homedir();
    var path10 = require("path");
    var caller = require_caller();
    var nodeModulesPaths = require_node_modules_paths();
    var normalizeOptions = require_normalize_options();
    var isCore = require_is_core_module();
    var realpathFS = process.platform !== "win32" && fs4.realpath && typeof fs4.realpath.native === "function" ? fs4.realpath.native : fs4.realpath;
    var homedir = getHomedir();
    var defaultPaths = function() {
      return [
        path10.join(homedir, ".node_modules"),
        path10.join(homedir, ".node_libraries")
      ];
    };
    var defaultIsFile = function isFile(file2, cb) {
      fs4.stat(file2, function(err, stat) {
        if (!err) {
          return cb(null, stat.isFile() || stat.isFIFO());
        }
        if (err.code === "ENOENT" || err.code === "ENOTDIR") return cb(null, false);
        return cb(err);
      });
    };
    var defaultIsDir = function isDirectory(dir, cb) {
      fs4.stat(dir, function(err, stat) {
        if (!err) {
          return cb(null, stat.isDirectory());
        }
        if (err.code === "ENOENT" || err.code === "ENOTDIR") return cb(null, false);
        return cb(err);
      });
    };
    var defaultRealpath = function realpath(x, cb) {
      realpathFS(x, function(realpathErr, realPath) {
        if (realpathErr && realpathErr.code !== "ENOENT") cb(realpathErr);
        else cb(null, realpathErr ? x : realPath);
      });
    };
    var maybeRealpath = function maybeRealpath2(realpath, x, opts, cb) {
      if (opts && opts.preserveSymlinks === false) {
        realpath(x, cb);
      } else {
        cb(null, x);
      }
    };
    var defaultReadPackage = function defaultReadPackage2(readFile, pkgfile, cb) {
      readFile(pkgfile, function(readFileErr, body) {
        if (readFileErr) cb(readFileErr);
        else {
          try {
            var pkg = JSON.parse(body);
            cb(null, pkg);
          } catch (jsonErr) {
            cb(null);
          }
        }
      });
    };
    var getPackageCandidates = function getPackageCandidates2(x, start, opts) {
      var dirs = nodeModulesPaths(start, opts, x);
      for (var i = 0; i < dirs.length; i++) {
        dirs[i] = path10.join(dirs[i], x);
      }
      return dirs;
    };
    module2.exports = function resolve3(x, options2, callback) {
      var cb = callback;
      var opts = options2;
      if (typeof options2 === "function") {
        cb = opts;
        opts = {};
      }
      if (typeof x !== "string") {
        var err = new TypeError("Path must be a string.");
        return process.nextTick(function() {
          cb(err);
        });
      }
      opts = normalizeOptions(x, opts);
      var isFile = opts.isFile || defaultIsFile;
      var isDirectory = opts.isDirectory || defaultIsDir;
      var readFile = opts.readFile || fs4.readFile;
      var realpath = opts.realpath || defaultRealpath;
      var readPackage = opts.readPackage || defaultReadPackage;
      if (opts.readFile && opts.readPackage) {
        var conflictErr = new TypeError("`readFile` and `readPackage` are mutually exclusive.");
        return process.nextTick(function() {
          cb(conflictErr);
        });
      }
      var packageIterator = opts.packageIterator;
      var extensions = opts.extensions || [".js"];
      var includeCoreModules = opts.includeCoreModules !== false;
      var basedir = opts.basedir || path10.dirname(caller());
      var parent = opts.filename || basedir;
      opts.paths = opts.paths || defaultPaths();
      var absoluteStart = path10.resolve(basedir);
      maybeRealpath(
        realpath,
        absoluteStart,
        opts,
        function(err2, realStart) {
          if (err2) cb(err2);
          else init3(realStart);
        }
      );
      var res;
      function init3(basedir2) {
        if (/^(?:\.\.?(?:\/|$)|\/|([A-Za-z]:)?[/\\])/.test(x)) {
          res = path10.resolve(basedir2, x);
          if (x === "." || x === ".." || x.slice(-1) === "/") res += "/";
          if (/\/$/.test(x) && res === basedir2) {
            loadAsDirectory(res, opts.package, onfile);
          } else loadAsFile(res, opts.package, onfile);
        } else if (includeCoreModules && isCore(x)) {
          return cb(null, x);
        } else loadNodeModules(x, basedir2, function(err2, n, pkg) {
          if (err2) cb(err2);
          else if (n) {
            return maybeRealpath(realpath, n, opts, function(err3, realN) {
              if (err3) {
                cb(err3);
              } else {
                cb(null, realN, pkg);
              }
            });
          } else {
            var moduleError = new Error("Cannot find module '" + x + "' from '" + parent + "'");
            moduleError.code = "MODULE_NOT_FOUND";
            cb(moduleError);
          }
        });
      }
      function onfile(err2, m, pkg) {
        if (err2) cb(err2);
        else if (m) cb(null, m, pkg);
        else loadAsDirectory(res, function(err3, d, pkg2) {
          if (err3) cb(err3);
          else if (d) {
            maybeRealpath(realpath, d, opts, function(err4, realD) {
              if (err4) {
                cb(err4);
              } else {
                cb(null, realD, pkg2);
              }
            });
          } else {
            var moduleError = new Error("Cannot find module '" + x + "' from '" + parent + "'");
            moduleError.code = "MODULE_NOT_FOUND";
            cb(moduleError);
          }
        });
      }
      function loadAsFile(x2, thePackage, callback2) {
        var loadAsFilePackage = thePackage;
        var cb2 = callback2;
        if (typeof loadAsFilePackage === "function") {
          cb2 = loadAsFilePackage;
          loadAsFilePackage = void 0;
        }
        var exts = [""].concat(extensions);
        load2(exts, x2, loadAsFilePackage);
        function load2(exts2, x3, loadPackage) {
          if (exts2.length === 0) return cb2(null, void 0, loadPackage);
          var file2 = x3 + exts2[0];
          var pkg = loadPackage;
          if (pkg) onpkg(null, pkg);
          else loadpkg(path10.dirname(file2), onpkg);
          function onpkg(err2, pkg_, dir) {
            pkg = pkg_;
            if (err2) return cb2(err2);
            if (dir && pkg && opts.pathFilter) {
              var rfile = path10.relative(dir, file2);
              var rel = rfile.slice(0, rfile.length - exts2[0].length);
              var r = opts.pathFilter(pkg, x3, rel);
              if (r) return load2(
                [""].concat(extensions.slice()),
                path10.resolve(dir, r),
                pkg
              );
            }
            isFile(file2, onex);
          }
          function onex(err2, ex) {
            if (err2) return cb2(err2);
            if (ex) return cb2(null, file2, pkg);
            load2(exts2.slice(1), x3, pkg);
          }
        }
      }
      function loadpkg(dir, cb2) {
        if (dir === "" || dir === "/") return cb2(null);
        if (process.platform === "win32" && /^\w:[/\\]*$/.test(dir)) {
          return cb2(null);
        }
        if (/[/\\]node_modules[/\\]*$/.test(dir)) return cb2(null);
        maybeRealpath(realpath, dir, opts, function(unwrapErr, pkgdir) {
          if (unwrapErr) return loadpkg(path10.dirname(dir), cb2);
          var pkgfile = path10.join(pkgdir, "package.json");
          isFile(pkgfile, function(err2, ex) {
            if (!ex) return loadpkg(path10.dirname(dir), cb2);
            readPackage(readFile, pkgfile, function(err3, pkgParam) {
              if (err3) cb2(err3);
              var pkg = pkgParam;
              if (pkg && opts.packageFilter) {
                pkg = opts.packageFilter(pkg, pkgfile);
              }
              cb2(null, pkg, dir);
            });
          });
        });
      }
      function loadAsDirectory(x2, loadAsDirectoryPackage, callback2) {
        var cb2 = callback2;
        var fpkg = loadAsDirectoryPackage;
        if (typeof fpkg === "function") {
          cb2 = fpkg;
          fpkg = opts.package;
        }
        maybeRealpath(realpath, x2, opts, function(unwrapErr, pkgdir) {
          if (unwrapErr) return cb2(unwrapErr);
          var pkgfile = path10.join(pkgdir, "package.json");
          isFile(pkgfile, function(err2, ex) {
            if (err2) return cb2(err2);
            if (!ex) return loadAsFile(path10.join(x2, "index"), fpkg, cb2);
            readPackage(readFile, pkgfile, function(err3, pkgParam) {
              if (err3) return cb2(err3);
              var pkg = pkgParam;
              if (pkg && opts.packageFilter) {
                pkg = opts.packageFilter(pkg, pkgfile);
              }
              if (pkg && pkg.main) {
                if (typeof pkg.main !== "string") {
                  var mainError = new TypeError("package \u201C" + pkg.name + "\u201D `main` must be a string");
                  mainError.code = "INVALID_PACKAGE_MAIN";
                  return cb2(mainError);
                }
                if (pkg.main === "." || pkg.main === "./") {
                  pkg.main = "index";
                }
                loadAsFile(path10.resolve(x2, pkg.main), pkg, function(err4, m, pkg2) {
                  if (err4) return cb2(err4);
                  if (m) return cb2(null, m, pkg2);
                  if (!pkg2) return loadAsFile(path10.join(x2, "index"), pkg2, cb2);
                  var dir = path10.resolve(x2, pkg2.main);
                  loadAsDirectory(dir, pkg2, function(err5, n, pkg3) {
                    if (err5) return cb2(err5);
                    if (n) return cb2(null, n, pkg3);
                    loadAsFile(path10.join(x2, "index"), pkg3, cb2);
                  });
                });
                return;
              }
              loadAsFile(path10.join(x2, "/index"), pkg, cb2);
            });
          });
        });
      }
      function processDirs(cb2, dirs) {
        if (dirs.length === 0) return cb2(null, void 0);
        var dir = dirs[0];
        isDirectory(path10.dirname(dir), isdir);
        function isdir(err2, isdir2) {
          if (err2) return cb2(err2);
          if (!isdir2) return processDirs(cb2, dirs.slice(1));
          loadAsFile(dir, opts.package, onfile2);
        }
        function onfile2(err2, m, pkg) {
          if (err2) return cb2(err2);
          if (m) return cb2(null, m, pkg);
          loadAsDirectory(dir, opts.package, ondir);
        }
        function ondir(err2, n, pkg) {
          if (err2) return cb2(err2);
          if (n) return cb2(null, n, pkg);
          processDirs(cb2, dirs.slice(1));
        }
      }
      function loadNodeModules(x2, start, cb2) {
        var thunk = function() {
          return getPackageCandidates(x2, start, opts);
        };
        processDirs(
          cb2,
          packageIterator ? packageIterator(x2, start, thunk, opts) : thunk()
        );
      }
    };
  }
});

// ../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/core.json
var require_core2 = __commonJS({
  "../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/core.json"(exports2, module2) {
    module2.exports = {
      assert: true,
      "node:assert": [">= 14.18 && < 15", ">= 16"],
      "assert/strict": ">= 15",
      "node:assert/strict": ">= 16",
      async_hooks: ">= 8",
      "node:async_hooks": [">= 14.18 && < 15", ">= 16"],
      buffer_ieee754: ">= 0.5 && < 0.9.7",
      buffer: true,
      "node:buffer": [">= 14.18 && < 15", ">= 16"],
      child_process: true,
      "node:child_process": [">= 14.18 && < 15", ">= 16"],
      cluster: ">= 0.5",
      "node:cluster": [">= 14.18 && < 15", ">= 16"],
      console: true,
      "node:console": [">= 14.18 && < 15", ">= 16"],
      constants: true,
      "node:constants": [">= 14.18 && < 15", ">= 16"],
      crypto: true,
      "node:crypto": [">= 14.18 && < 15", ">= 16"],
      _debug_agent: ">= 1 && < 8",
      _debugger: "< 8",
      dgram: true,
      "node:dgram": [">= 14.18 && < 15", ">= 16"],
      diagnostics_channel: [">= 14.17 && < 15", ">= 15.1"],
      "node:diagnostics_channel": [">= 14.18 && < 15", ">= 16"],
      dns: true,
      "node:dns": [">= 14.18 && < 15", ">= 16"],
      "dns/promises": ">= 15",
      "node:dns/promises": ">= 16",
      domain: ">= 0.7.12",
      "node:domain": [">= 14.18 && < 15", ">= 16"],
      events: true,
      "node:events": [">= 14.18 && < 15", ">= 16"],
      freelist: "< 6",
      fs: true,
      "node:fs": [">= 14.18 && < 15", ">= 16"],
      "fs/promises": [">= 10 && < 10.1", ">= 14"],
      "node:fs/promises": [">= 14.18 && < 15", ">= 16"],
      _http_agent: ">= 0.11.1",
      "node:_http_agent": [">= 14.18 && < 15", ">= 16"],
      _http_client: ">= 0.11.1",
      "node:_http_client": [">= 14.18 && < 15", ">= 16"],
      _http_common: ">= 0.11.1",
      "node:_http_common": [">= 14.18 && < 15", ">= 16"],
      _http_incoming: ">= 0.11.1",
      "node:_http_incoming": [">= 14.18 && < 15", ">= 16"],
      _http_outgoing: ">= 0.11.1",
      "node:_http_outgoing": [">= 14.18 && < 15", ">= 16"],
      _http_server: ">= 0.11.1",
      "node:_http_server": [">= 14.18 && < 15", ">= 16"],
      http: true,
      "node:http": [">= 14.18 && < 15", ">= 16"],
      http2: ">= 8.8",
      "node:http2": [">= 14.18 && < 15", ">= 16"],
      https: true,
      "node:https": [">= 14.18 && < 15", ">= 16"],
      inspector: ">= 8",
      "node:inspector": [">= 14.18 && < 15", ">= 16"],
      "inspector/promises": [">= 19"],
      "node:inspector/promises": [">= 19"],
      _linklist: "< 8",
      module: true,
      "node:module": [">= 14.18 && < 15", ">= 16"],
      net: true,
      "node:net": [">= 14.18 && < 15", ">= 16"],
      "node-inspect/lib/_inspect": ">= 7.6 && < 12",
      "node-inspect/lib/internal/inspect_client": ">= 7.6 && < 12",
      "node-inspect/lib/internal/inspect_repl": ">= 7.6 && < 12",
      os: true,
      "node:os": [">= 14.18 && < 15", ">= 16"],
      path: true,
      "node:path": [">= 14.18 && < 15", ">= 16"],
      "path/posix": ">= 15.3",
      "node:path/posix": ">= 16",
      "path/win32": ">= 15.3",
      "node:path/win32": ">= 16",
      perf_hooks: ">= 8.5",
      "node:perf_hooks": [">= 14.18 && < 15", ">= 16"],
      process: ">= 1",
      "node:process": [">= 14.18 && < 15", ">= 16"],
      punycode: ">= 0.5",
      "node:punycode": [">= 14.18 && < 15", ">= 16"],
      querystring: true,
      "node:querystring": [">= 14.18 && < 15", ">= 16"],
      readline: true,
      "node:readline": [">= 14.18 && < 15", ">= 16"],
      "readline/promises": ">= 17",
      "node:readline/promises": ">= 17",
      repl: true,
      "node:repl": [">= 14.18 && < 15", ">= 16"],
      "node:sea": [">= 20.12 && < 21", ">= 21.7"],
      smalloc: ">= 0.11.5 && < 3",
      "node:sqlite": ">= 23.4",
      _stream_duplex: ">= 0.9.4",
      "node:_stream_duplex": [">= 14.18 && < 15", ">= 16"],
      _stream_transform: ">= 0.9.4",
      "node:_stream_transform": [">= 14.18 && < 15", ">= 16"],
      _stream_wrap: ">= 1.4.1",
      "node:_stream_wrap": [">= 14.18 && < 15", ">= 16"],
      _stream_passthrough: ">= 0.9.4",
      "node:_stream_passthrough": [">= 14.18 && < 15", ">= 16"],
      _stream_readable: ">= 0.9.4",
      "node:_stream_readable": [">= 14.18 && < 15", ">= 16"],
      _stream_writable: ">= 0.9.4",
      "node:_stream_writable": [">= 14.18 && < 15", ">= 16"],
      stream: true,
      "node:stream": [">= 14.18 && < 15", ">= 16"],
      "stream/consumers": ">= 16.7",
      "node:stream/consumers": ">= 16.7",
      "stream/promises": ">= 15",
      "node:stream/promises": ">= 16",
      "stream/web": ">= 16.5",
      "node:stream/web": ">= 16.5",
      string_decoder: true,
      "node:string_decoder": [">= 14.18 && < 15", ">= 16"],
      sys: [">= 0.4 && < 0.7", ">= 0.8"],
      "node:sys": [">= 14.18 && < 15", ">= 16"],
      "test/reporters": ">= 19.9 && < 20.2",
      "node:test/reporters": [">= 18.17 && < 19", ">= 19.9", ">= 20"],
      "test/mock_loader": ">= 22.3 && < 22.7",
      "node:test/mock_loader": ">= 22.3 && < 22.7",
      "node:test": [">= 16.17 && < 17", ">= 18"],
      timers: true,
      "node:timers": [">= 14.18 && < 15", ">= 16"],
      "timers/promises": ">= 15",
      "node:timers/promises": ">= 16",
      _tls_common: ">= 0.11.13",
      "node:_tls_common": [">= 14.18 && < 15", ">= 16"],
      _tls_legacy: ">= 0.11.3 && < 10",
      _tls_wrap: ">= 0.11.3",
      "node:_tls_wrap": [">= 14.18 && < 15", ">= 16"],
      tls: true,
      "node:tls": [">= 14.18 && < 15", ">= 16"],
      trace_events: ">= 10",
      "node:trace_events": [">= 14.18 && < 15", ">= 16"],
      tty: true,
      "node:tty": [">= 14.18 && < 15", ">= 16"],
      url: true,
      "node:url": [">= 14.18 && < 15", ">= 16"],
      util: true,
      "node:util": [">= 14.18 && < 15", ">= 16"],
      "util/types": ">= 15.3",
      "node:util/types": ">= 16",
      "v8/tools/arguments": ">= 10 && < 12",
      "v8/tools/codemap": [">= 4.4 && < 5", ">= 5.2 && < 12"],
      "v8/tools/consarray": [">= 4.4 && < 5", ">= 5.2 && < 12"],
      "v8/tools/csvparser": [">= 4.4 && < 5", ">= 5.2 && < 12"],
      "v8/tools/logreader": [">= 4.4 && < 5", ">= 5.2 && < 12"],
      "v8/tools/profile_view": [">= 4.4 && < 5", ">= 5.2 && < 12"],
      "v8/tools/splaytree": [">= 4.4 && < 5", ">= 5.2 && < 12"],
      v8: ">= 1",
      "node:v8": [">= 14.18 && < 15", ">= 16"],
      vm: true,
      "node:vm": [">= 14.18 && < 15", ">= 16"],
      wasi: [">= 13.4 && < 13.5", ">= 18.17 && < 19", ">= 20"],
      "node:wasi": [">= 18.17 && < 19", ">= 20"],
      worker_threads: ">= 11.7",
      "node:worker_threads": [">= 14.18 && < 15", ">= 16"],
      zlib: ">= 0.5",
      "node:zlib": [">= 14.18 && < 15", ">= 16"]
    };
  }
});

// ../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/core.js
var require_core3 = __commonJS({
  "../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/core.js"(exports2, module2) {
    "use strict";
    var isCoreModule = require_is_core_module();
    var data = require_core2();
    var core2 = {};
    for (mod in data) {
      if (Object.prototype.hasOwnProperty.call(data, mod)) {
        core2[mod] = isCoreModule(mod);
      }
    }
    var mod;
    module2.exports = core2;
  }
});

// ../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/is-core.js
var require_is_core = __commonJS({
  "../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/is-core.js"(exports2, module2) {
    "use strict";
    var isCoreModule = require_is_core_module();
    module2.exports = function isCore(x) {
      return isCoreModule(x);
    };
  }
});

// ../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/sync.js
var require_sync = __commonJS({
  "../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/lib/sync.js"(exports2, module2) {
    "use strict";
    var isCore = require_is_core_module();
    var fs4 = require("fs");
    var path10 = require("path");
    var getHomedir = require_homedir();
    var caller = require_caller();
    var nodeModulesPaths = require_node_modules_paths();
    var normalizeOptions = require_normalize_options();
    var realpathFS = process.platform !== "win32" && fs4.realpathSync && typeof fs4.realpathSync.native === "function" ? fs4.realpathSync.native : fs4.realpathSync;
    var homedir = getHomedir();
    var defaultPaths = function() {
      return [
        path10.join(homedir, ".node_modules"),
        path10.join(homedir, ".node_libraries")
      ];
    };
    var defaultIsFile = function isFile(file2) {
      try {
        var stat = fs4.statSync(file2, { throwIfNoEntry: false });
      } catch (e) {
        if (e && (e.code === "ENOENT" || e.code === "ENOTDIR")) return false;
        throw e;
      }
      return !!stat && (stat.isFile() || stat.isFIFO());
    };
    var defaultIsDir = function isDirectory(dir) {
      try {
        var stat = fs4.statSync(dir, { throwIfNoEntry: false });
      } catch (e) {
        if (e && (e.code === "ENOENT" || e.code === "ENOTDIR")) return false;
        throw e;
      }
      return !!stat && stat.isDirectory();
    };
    var defaultRealpathSync = function realpathSync(x) {
      try {
        return realpathFS(x);
      } catch (realpathErr) {
        if (realpathErr.code !== "ENOENT") {
          throw realpathErr;
        }
      }
      return x;
    };
    var maybeRealpathSync = function maybeRealpathSync2(realpathSync, x, opts) {
      if (opts && opts.preserveSymlinks === false) {
        return realpathSync(x);
      }
      return x;
    };
    var defaultReadPackageSync = function defaultReadPackageSync2(readFileSync, pkgfile) {
      var body = readFileSync(pkgfile);
      try {
        var pkg = JSON.parse(body);
        return pkg;
      } catch (jsonErr) {
      }
    };
    var getPackageCandidates = function getPackageCandidates2(x, start, opts) {
      var dirs = nodeModulesPaths(start, opts, x);
      for (var i = 0; i < dirs.length; i++) {
        dirs[i] = path10.join(dirs[i], x);
      }
      return dirs;
    };
    module2.exports = function resolveSync(x, options2) {
      if (typeof x !== "string") {
        throw new TypeError("Path must be a string.");
      }
      var opts = normalizeOptions(x, options2);
      var isFile = opts.isFile || defaultIsFile;
      var readFileSync = opts.readFileSync || fs4.readFileSync;
      var isDirectory = opts.isDirectory || defaultIsDir;
      var realpathSync = opts.realpathSync || defaultRealpathSync;
      var readPackageSync = opts.readPackageSync || defaultReadPackageSync;
      if (opts.readFileSync && opts.readPackageSync) {
        throw new TypeError("`readFileSync` and `readPackageSync` are mutually exclusive.");
      }
      var packageIterator = opts.packageIterator;
      var extensions = opts.extensions || [".js"];
      var includeCoreModules = opts.includeCoreModules !== false;
      var basedir = opts.basedir || path10.dirname(caller());
      var parent = opts.filename || basedir;
      opts.paths = opts.paths || defaultPaths();
      var absoluteStart = maybeRealpathSync(realpathSync, path10.resolve(basedir), opts);
      if (/^(?:\.\.?(?:\/|$)|\/|([A-Za-z]:)?[/\\])/.test(x)) {
        var res = path10.resolve(absoluteStart, x);
        if (x === "." || x === ".." || x.slice(-1) === "/") res += "/";
        var m = loadAsFileSync(res) || loadAsDirectorySync(res);
        if (m) return maybeRealpathSync(realpathSync, m, opts);
      } else if (includeCoreModules && isCore(x)) {
        return x;
      } else {
        var n = loadNodeModulesSync(x, absoluteStart);
        if (n) return maybeRealpathSync(realpathSync, n, opts);
      }
      var err = new Error("Cannot find module '" + x + "' from '" + parent + "'");
      err.code = "MODULE_NOT_FOUND";
      throw err;
      function loadAsFileSync(x2) {
        var pkg = loadpkg(path10.dirname(x2));
        if (pkg && pkg.dir && pkg.pkg && opts.pathFilter) {
          var rfile = path10.relative(pkg.dir, x2);
          var r = opts.pathFilter(pkg.pkg, x2, rfile);
          if (r) {
            x2 = path10.resolve(pkg.dir, r);
          }
        }
        if (isFile(x2)) {
          return x2;
        }
        for (var i = 0; i < extensions.length; i++) {
          var file2 = x2 + extensions[i];
          if (isFile(file2)) {
            return file2;
          }
        }
      }
      function loadpkg(dir) {
        if (dir === "" || dir === "/") return;
        if (process.platform === "win32" && /^\w:[/\\]*$/.test(dir)) {
          return;
        }
        if (/[/\\]node_modules[/\\]*$/.test(dir)) return;
        var pkgfile = path10.join(maybeRealpathSync(realpathSync, dir, opts), "package.json");
        if (!isFile(pkgfile)) {
          return loadpkg(path10.dirname(dir));
        }
        var pkg = readPackageSync(readFileSync, pkgfile);
        if (pkg && opts.packageFilter) {
          pkg = opts.packageFilter(
            pkg,
            /*pkgfile,*/
            dir
          );
        }
        return { pkg, dir };
      }
      function loadAsDirectorySync(x2) {
        var pkgfile = path10.join(maybeRealpathSync(realpathSync, x2, opts), "/package.json");
        if (isFile(pkgfile)) {
          try {
            var pkg = readPackageSync(readFileSync, pkgfile);
          } catch (e) {
          }
          if (pkg && opts.packageFilter) {
            pkg = opts.packageFilter(
              pkg,
              /*pkgfile,*/
              x2
            );
          }
          if (pkg && pkg.main) {
            if (typeof pkg.main !== "string") {
              var mainError = new TypeError("package \u201C" + pkg.name + "\u201D `main` must be a string");
              mainError.code = "INVALID_PACKAGE_MAIN";
              throw mainError;
            }
            if (pkg.main === "." || pkg.main === "./") {
              pkg.main = "index";
            }
            try {
              var m2 = loadAsFileSync(path10.resolve(x2, pkg.main));
              if (m2) return m2;
              var n2 = loadAsDirectorySync(path10.resolve(x2, pkg.main));
              if (n2) return n2;
            } catch (e) {
            }
          }
        }
        return loadAsFileSync(path10.join(x2, "/index"));
      }
      function loadNodeModulesSync(x2, start) {
        var thunk = function() {
          return getPackageCandidates(x2, start, opts);
        };
        var dirs = packageIterator ? packageIterator(x2, start, thunk, opts) : thunk();
        for (var i = 0; i < dirs.length; i++) {
          var dir = dirs[i];
          if (isDirectory(path10.dirname(dir))) {
            var m2 = loadAsFileSync(dir);
            if (m2) return m2;
            var n2 = loadAsDirectorySync(dir);
            if (n2) return n2;
          }
        }
      }
    };
  }
});

// ../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/index.js
var require_resolve = __commonJS({
  "../../node_modules/.pnpm/resolve@1.22.10/node_modules/resolve/index.js"(exports2, module2) {
    "use strict";
    var async = require_async();
    async.core = require_core3();
    async.isCore = require_is_core();
    async.sync = require_sync();
    module2.exports = async;
  }
});

// ../../node_modules/.pnpm/@babel+helper-validator-identifier@7.25.9/node_modules/@babel/helper-validator-identifier/lib/identifier.js
var require_identifier = __commonJS({
  "../../node_modules/.pnpm/@babel+helper-validator-identifier@7.25.9/node_modules/@babel/helper-validator-identifier/lib/identifier.js"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", {
      value: true
    });
    exports2.isIdentifierChar = isIdentifierChar;
    exports2.isIdentifierName = isIdentifierName2;
    exports2.isIdentifierStart = isIdentifierStart;
    var nonASCIIidentifierStartChars = "\xAA\xB5\xBA\xC0-\xD6\xD8-\xF6\xF8-\u02C1\u02C6-\u02D1\u02E0-\u02E4\u02EC\u02EE\u0370-\u0374\u0376\u0377\u037A-\u037D\u037F\u0386\u0388-\u038A\u038C\u038E-\u03A1\u03A3-\u03F5\u03F7-\u0481\u048A-\u052F\u0531-\u0556\u0559\u0560-\u0588\u05D0-\u05EA\u05EF-\u05F2\u0620-\u064A\u066E\u066F\u0671-\u06D3\u06D5\u06E5\u06E6\u06EE\u06EF\u06FA-\u06FC\u06FF\u0710\u0712-\u072F\u074D-\u07A5\u07B1\u07CA-\u07EA\u07F4\u07F5\u07FA\u0800-\u0815\u081A\u0824\u0828\u0840-\u0858\u0860-\u086A\u0870-\u0887\u0889-\u088E\u08A0-\u08C9\u0904-\u0939\u093D\u0950\u0958-\u0961\u0971-\u0980\u0985-\u098C\u098F\u0990\u0993-\u09A8\u09AA-\u09B0\u09B2\u09B6-\u09B9\u09BD\u09CE\u09DC\u09DD\u09DF-\u09E1\u09F0\u09F1\u09FC\u0A05-\u0A0A\u0A0F\u0A10\u0A13-\u0A28\u0A2A-\u0A30\u0A32\u0A33\u0A35\u0A36\u0A38\u0A39\u0A59-\u0A5C\u0A5E\u0A72-\u0A74\u0A85-\u0A8D\u0A8F-\u0A91\u0A93-\u0AA8\u0AAA-\u0AB0\u0AB2\u0AB3\u0AB5-\u0AB9\u0ABD\u0AD0\u0AE0\u0AE1\u0AF9\u0B05-\u0B0C\u0B0F\u0B10\u0B13-\u0B28\u0B2A-\u0B30\u0B32\u0B33\u0B35-\u0B39\u0B3D\u0B5C\u0B5D\u0B5F-\u0B61\u0B71\u0B83\u0B85-\u0B8A\u0B8E-\u0B90\u0B92-\u0B95\u0B99\u0B9A\u0B9C\u0B9E\u0B9F\u0BA3\u0BA4\u0BA8-\u0BAA\u0BAE-\u0BB9\u0BD0\u0C05-\u0C0C\u0C0E-\u0C10\u0C12-\u0C28\u0C2A-\u0C39\u0C3D\u0C58-\u0C5A\u0C5D\u0C60\u0C61\u0C80\u0C85-\u0C8C\u0C8E-\u0C90\u0C92-\u0CA8\u0CAA-\u0CB3\u0CB5-\u0CB9\u0CBD\u0CDD\u0CDE\u0CE0\u0CE1\u0CF1\u0CF2\u0D04-\u0D0C\u0D0E-\u0D10\u0D12-\u0D3A\u0D3D\u0D4E\u0D54-\u0D56\u0D5F-\u0D61\u0D7A-\u0D7F\u0D85-\u0D96\u0D9A-\u0DB1\u0DB3-\u0DBB\u0DBD\u0DC0-\u0DC6\u0E01-\u0E30\u0E32\u0E33\u0E40-\u0E46\u0E81\u0E82\u0E84\u0E86-\u0E8A\u0E8C-\u0EA3\u0EA5\u0EA7-\u0EB0\u0EB2\u0EB3\u0EBD\u0EC0-\u0EC4\u0EC6\u0EDC-\u0EDF\u0F00\u0F40-\u0F47\u0F49-\u0F6C\u0F88-\u0F8C\u1000-\u102A\u103F\u1050-\u1055\u105A-\u105D\u1061\u1065\u1066\u106E-\u1070\u1075-\u1081\u108E\u10A0-\u10C5\u10C7\u10CD\u10D0-\u10FA\u10FC-\u1248\u124A-\u124D\u1250-\u1256\u1258\u125A-\u125D\u1260-\u1288\u128A-\u128D\u1290-\u12B0\u12B2-\u12B5\u12B8-\u12BE\u12C0\u12C2-\u12C5\u12C8-\u12D6\u12D8-\u1310\u1312-\u1315\u1318-\u135A\u1380-\u138F\u13A0-\u13F5\u13F8-\u13FD\u1401-\u166C\u166F-\u167F\u1681-\u169A\u16A0-\u16EA\u16EE-\u16F8\u1700-\u1711\u171F-\u1731\u1740-\u1751\u1760-\u176C\u176E-\u1770\u1780-\u17B3\u17D7\u17DC\u1820-\u1878\u1880-\u18A8\u18AA\u18B0-\u18F5\u1900-\u191E\u1950-\u196D\u1970-\u1974\u1980-\u19AB\u19B0-\u19C9\u1A00-\u1A16\u1A20-\u1A54\u1AA7\u1B05-\u1B33\u1B45-\u1B4C\u1B83-\u1BA0\u1BAE\u1BAF\u1BBA-\u1BE5\u1C00-\u1C23\u1C4D-\u1C4F\u1C5A-\u1C7D\u1C80-\u1C8A\u1C90-\u1CBA\u1CBD-\u1CBF\u1CE9-\u1CEC\u1CEE-\u1CF3\u1CF5\u1CF6\u1CFA\u1D00-\u1DBF\u1E00-\u1F15\u1F18-\u1F1D\u1F20-\u1F45\u1F48-\u1F4D\u1F50-\u1F57\u1F59\u1F5B\u1F5D\u1F5F-\u1F7D\u1F80-\u1FB4\u1FB6-\u1FBC\u1FBE\u1FC2-\u1FC4\u1FC6-\u1FCC\u1FD0-\u1FD3\u1FD6-\u1FDB\u1FE0-\u1FEC\u1FF2-\u1FF4\u1FF6-\u1FFC\u2071\u207F\u2090-\u209C\u2102\u2107\u210A-\u2113\u2115\u2118-\u211D\u2124\u2126\u2128\u212A-\u2139\u213C-\u213F\u2145-\u2149\u214E\u2160-\u2188\u2C00-\u2CE4\u2CEB-\u2CEE\u2CF2\u2CF3\u2D00-\u2D25\u2D27\u2D2D\u2D30-\u2D67\u2D6F\u2D80-\u2D96\u2DA0-\u2DA6\u2DA8-\u2DAE\u2DB0-\u2DB6\u2DB8-\u2DBE\u2DC0-\u2DC6\u2DC8-\u2DCE\u2DD0-\u2DD6\u2DD8-\u2DDE\u3005-\u3007\u3021-\u3029\u3031-\u3035\u3038-\u303C\u3041-\u3096\u309B-\u309F\u30A1-\u30FA\u30FC-\u30FF\u3105-\u312F\u3131-\u318E\u31A0-\u31BF\u31F0-\u31FF\u3400-\u4DBF\u4E00-\uA48C\uA4D0-\uA4FD\uA500-\uA60C\uA610-\uA61F\uA62A\uA62B\uA640-\uA66E\uA67F-\uA69D\uA6A0-\uA6EF\uA717-\uA71F\uA722-\uA788\uA78B-\uA7CD\uA7D0\uA7D1\uA7D3\uA7D5-\uA7DC\uA7F2-\uA801\uA803-\uA805\uA807-\uA80A\uA80C-\uA822\uA840-\uA873\uA882-\uA8B3\uA8F2-\uA8F7\uA8FB\uA8FD\uA8FE\uA90A-\uA925\uA930-\uA946\uA960-\uA97C\uA984-\uA9B2\uA9CF\uA9E0-\uA9E4\uA9E6-\uA9EF\uA9FA-\uA9FE\uAA00-\uAA28\uAA40-\uAA42\uAA44-\uAA4B\uAA60-\uAA76\uAA7A\uAA7E-\uAAAF\uAAB1\uAAB5\uAAB6\uAAB9-\uAABD\uAAC0\uAAC2\uAADB-\uAADD\uAAE0-\uAAEA\uAAF2-\uAAF4\uAB01-\uAB06\uAB09-\uAB0E\uAB11-\uAB16\uAB20-\uAB26\uAB28-\uAB2E\uAB30-\uAB5A\uAB5C-\uAB69\uAB70-\uABE2\uAC00-\uD7A3\uD7B0-\uD7C6\uD7CB-\uD7FB\uF900-\uFA6D\uFA70-\uFAD9\uFB00-\uFB06\uFB13-\uFB17\uFB1D\uFB1F-\uFB28\uFB2A-\uFB36\uFB38-\uFB3C\uFB3E\uFB40\uFB41\uFB43\uFB44\uFB46-\uFBB1\uFBD3-\uFD3D\uFD50-\uFD8F\uFD92-\uFDC7\uFDF0-\uFDFB\uFE70-\uFE74\uFE76-\uFEFC\uFF21-\uFF3A\uFF41-\uFF5A\uFF66-\uFFBE\uFFC2-\uFFC7\uFFCA-\uFFCF\uFFD2-\uFFD7\uFFDA-\uFFDC";
    var nonASCIIidentifierChars = "\xB7\u0300-\u036F\u0387\u0483-\u0487\u0591-\u05BD\u05BF\u05C1\u05C2\u05C4\u05C5\u05C7\u0610-\u061A\u064B-\u0669\u0670\u06D6-\u06DC\u06DF-\u06E4\u06E7\u06E8\u06EA-\u06ED\u06F0-\u06F9\u0711\u0730-\u074A\u07A6-\u07B0\u07C0-\u07C9\u07EB-\u07F3\u07FD\u0816-\u0819\u081B-\u0823\u0825-\u0827\u0829-\u082D\u0859-\u085B\u0897-\u089F\u08CA-\u08E1\u08E3-\u0903\u093A-\u093C\u093E-\u094F\u0951-\u0957\u0962\u0963\u0966-\u096F\u0981-\u0983\u09BC\u09BE-\u09C4\u09C7\u09C8\u09CB-\u09CD\u09D7\u09E2\u09E3\u09E6-\u09EF\u09FE\u0A01-\u0A03\u0A3C\u0A3E-\u0A42\u0A47\u0A48\u0A4B-\u0A4D\u0A51\u0A66-\u0A71\u0A75\u0A81-\u0A83\u0ABC\u0ABE-\u0AC5\u0AC7-\u0AC9\u0ACB-\u0ACD\u0AE2\u0AE3\u0AE6-\u0AEF\u0AFA-\u0AFF\u0B01-\u0B03\u0B3C\u0B3E-\u0B44\u0B47\u0B48\u0B4B-\u0B4D\u0B55-\u0B57\u0B62\u0B63\u0B66-\u0B6F\u0B82\u0BBE-\u0BC2\u0BC6-\u0BC8\u0BCA-\u0BCD\u0BD7\u0BE6-\u0BEF\u0C00-\u0C04\u0C3C\u0C3E-\u0C44\u0C46-\u0C48\u0C4A-\u0C4D\u0C55\u0C56\u0C62\u0C63\u0C66-\u0C6F\u0C81-\u0C83\u0CBC\u0CBE-\u0CC4\u0CC6-\u0CC8\u0CCA-\u0CCD\u0CD5\u0CD6\u0CE2\u0CE3\u0CE6-\u0CEF\u0CF3\u0D00-\u0D03\u0D3B\u0D3C\u0D3E-\u0D44\u0D46-\u0D48\u0D4A-\u0D4D\u0D57\u0D62\u0D63\u0D66-\u0D6F\u0D81-\u0D83\u0DCA\u0DCF-\u0DD4\u0DD6\u0DD8-\u0DDF\u0DE6-\u0DEF\u0DF2\u0DF3\u0E31\u0E34-\u0E3A\u0E47-\u0E4E\u0E50-\u0E59\u0EB1\u0EB4-\u0EBC\u0EC8-\u0ECE\u0ED0-\u0ED9\u0F18\u0F19\u0F20-\u0F29\u0F35\u0F37\u0F39\u0F3E\u0F3F\u0F71-\u0F84\u0F86\u0F87\u0F8D-\u0F97\u0F99-\u0FBC\u0FC6\u102B-\u103E\u1040-\u1049\u1056-\u1059\u105E-\u1060\u1062-\u1064\u1067-\u106D\u1071-\u1074\u1082-\u108D\u108F-\u109D\u135D-\u135F\u1369-\u1371\u1712-\u1715\u1732-\u1734\u1752\u1753\u1772\u1773\u17B4-\u17D3\u17DD\u17E0-\u17E9\u180B-\u180D\u180F-\u1819\u18A9\u1920-\u192B\u1930-\u193B\u1946-\u194F\u19D0-\u19DA\u1A17-\u1A1B\u1A55-\u1A5E\u1A60-\u1A7C\u1A7F-\u1A89\u1A90-\u1A99\u1AB0-\u1ABD\u1ABF-\u1ACE\u1B00-\u1B04\u1B34-\u1B44\u1B50-\u1B59\u1B6B-\u1B73\u1B80-\u1B82\u1BA1-\u1BAD\u1BB0-\u1BB9\u1BE6-\u1BF3\u1C24-\u1C37\u1C40-\u1C49\u1C50-\u1C59\u1CD0-\u1CD2\u1CD4-\u1CE8\u1CED\u1CF4\u1CF7-\u1CF9\u1DC0-\u1DFF\u200C\u200D\u203F\u2040\u2054\u20D0-\u20DC\u20E1\u20E5-\u20F0\u2CEF-\u2CF1\u2D7F\u2DE0-\u2DFF\u302A-\u302F\u3099\u309A\u30FB\uA620-\uA629\uA66F\uA674-\uA67D\uA69E\uA69F\uA6F0\uA6F1\uA802\uA806\uA80B\uA823-\uA827\uA82C\uA880\uA881\uA8B4-\uA8C5\uA8D0-\uA8D9\uA8E0-\uA8F1\uA8FF-\uA909\uA926-\uA92D\uA947-\uA953\uA980-\uA983\uA9B3-\uA9C0\uA9D0-\uA9D9\uA9E5\uA9F0-\uA9F9\uAA29-\uAA36\uAA43\uAA4C\uAA4D\uAA50-\uAA59\uAA7B-\uAA7D\uAAB0\uAAB2-\uAAB4\uAAB7\uAAB8\uAABE\uAABF\uAAC1\uAAEB-\uAAEF\uAAF5\uAAF6\uABE3-\uABEA\uABEC\uABED\uABF0-\uABF9\uFB1E\uFE00-\uFE0F\uFE20-\uFE2F\uFE33\uFE34\uFE4D-\uFE4F\uFF10-\uFF19\uFF3F\uFF65";
    var nonASCIIidentifierStart = new RegExp("[" + nonASCIIidentifierStartChars + "]");
    var nonASCIIidentifier = new RegExp("[" + nonASCIIidentifierStartChars + nonASCIIidentifierChars + "]");
    nonASCIIidentifierStartChars = nonASCIIidentifierChars = null;
    var astralIdentifierStartCodes = [0, 11, 2, 25, 2, 18, 2, 1, 2, 14, 3, 13, 35, 122, 70, 52, 268, 28, 4, 48, 48, 31, 14, 29, 6, 37, 11, 29, 3, 35, 5, 7, 2, 4, 43, 157, 19, 35, 5, 35, 5, 39, 9, 51, 13, 10, 2, 14, 2, 6, 2, 1, 2, 10, 2, 14, 2, 6, 2, 1, 4, 51, 13, 310, 10, 21, 11, 7, 25, 5, 2, 41, 2, 8, 70, 5, 3, 0, 2, 43, 2, 1, 4, 0, 3, 22, 11, 22, 10, 30, 66, 18, 2, 1, 11, 21, 11, 25, 71, 55, 7, 1, 65, 0, 16, 3, 2, 2, 2, 28, 43, 28, 4, 28, 36, 7, 2, 27, 28, 53, 11, 21, 11, 18, 14, 17, 111, 72, 56, 50, 14, 50, 14, 35, 39, 27, 10, 22, 251, 41, 7, 1, 17, 2, 60, 28, 11, 0, 9, 21, 43, 17, 47, 20, 28, 22, 13, 52, 58, 1, 3, 0, 14, 44, 33, 24, 27, 35, 30, 0, 3, 0, 9, 34, 4, 0, 13, 47, 15, 3, 22, 0, 2, 0, 36, 17, 2, 24, 20, 1, 64, 6, 2, 0, 2, 3, 2, 14, 2, 9, 8, 46, 39, 7, 3, 1, 3, 21, 2, 6, 2, 1, 2, 4, 4, 0, 19, 0, 13, 4, 31, 9, 2, 0, 3, 0, 2, 37, 2, 0, 26, 0, 2, 0, 45, 52, 19, 3, 21, 2, 31, 47, 21, 1, 2, 0, 185, 46, 42, 3, 37, 47, 21, 0, 60, 42, 14, 0, 72, 26, 38, 6, 186, 43, 117, 63, 32, 7, 3, 0, 3, 7, 2, 1, 2, 23, 16, 0, 2, 0, 95, 7, 3, 38, 17, 0, 2, 0, 29, 0, 11, 39, 8, 0, 22, 0, 12, 45, 20, 0, 19, 72, 200, 32, 32, 8, 2, 36, 18, 0, 50, 29, 113, 6, 2, 1, 2, 37, 22, 0, 26, 5, 2, 1, 2, 31, 15, 0, 328, 18, 16, 0, 2, 12, 2, 33, 125, 0, 80, 921, 103, 110, 18, 195, 2637, 96, 16, 1071, 18, 5, 26, 3994, 6, 582, 6842, 29, 1763, 568, 8, 30, 18, 78, 18, 29, 19, 47, 17, 3, 32, 20, 6, 18, 433, 44, 212, 63, 129, 74, 6, 0, 67, 12, 65, 1, 2, 0, 29, 6135, 9, 1237, 42, 9, 8936, 3, 2, 6, 2, 1, 2, 290, 16, 0, 30, 2, 3, 0, 15, 3, 9, 395, 2309, 106, 6, 12, 4, 8, 8, 9, 5991, 84, 2, 70, 2, 1, 3, 0, 3, 1, 3, 3, 2, 11, 2, 0, 2, 6, 2, 64, 2, 3, 3, 7, 2, 6, 2, 27, 2, 3, 2, 4, 2, 0, 4, 6, 2, 339, 3, 24, 2, 24, 2, 30, 2, 24, 2, 30, 2, 24, 2, 30, 2, 24, 2, 30, 2, 24, 2, 7, 1845, 30, 7, 5, 262, 61, 147, 44, 11, 6, 17, 0, 322, 29, 19, 43, 485, 27, 229, 29, 3, 0, 496, 6, 2, 3, 2, 1, 2, 14, 2, 196, 60, 67, 8, 0, 1205, 3, 2, 26, 2, 1, 2, 0, 3, 0, 2, 9, 2, 3, 2, 0, 2, 0, 7, 0, 5, 0, 2, 0, 2, 0, 2, 2, 2, 1, 2, 0, 3, 0, 2, 0, 2, 0, 2, 0, 2, 0, 2, 1, 2, 0, 3, 3, 2, 6, 2, 3, 2, 3, 2, 0, 2, 9, 2, 16, 6, 2, 2, 4, 2, 16, 4421, 42719, 33, 4153, 7, 221, 3, 5761, 15, 7472, 16, 621, 2467, 541, 1507, 4938, 6, 4191];
    var astralIdentifierCodes = [509, 0, 227, 0, 150, 4, 294, 9, 1368, 2, 2, 1, 6, 3, 41, 2, 5, 0, 166, 1, 574, 3, 9, 9, 7, 9, 32, 4, 318, 1, 80, 3, 71, 10, 50, 3, 123, 2, 54, 14, 32, 10, 3, 1, 11, 3, 46, 10, 8, 0, 46, 9, 7, 2, 37, 13, 2, 9, 6, 1, 45, 0, 13, 2, 49, 13, 9, 3, 2, 11, 83, 11, 7, 0, 3, 0, 158, 11, 6, 9, 7, 3, 56, 1, 2, 6, 3, 1, 3, 2, 10, 0, 11, 1, 3, 6, 4, 4, 68, 8, 2, 0, 3, 0, 2, 3, 2, 4, 2, 0, 15, 1, 83, 17, 10, 9, 5, 0, 82, 19, 13, 9, 214, 6, 3, 8, 28, 1, 83, 16, 16, 9, 82, 12, 9, 9, 7, 19, 58, 14, 5, 9, 243, 14, 166, 9, 71, 5, 2, 1, 3, 3, 2, 0, 2, 1, 13, 9, 120, 6, 3, 6, 4, 0, 29, 9, 41, 6, 2, 3, 9, 0, 10, 10, 47, 15, 343, 9, 54, 7, 2, 7, 17, 9, 57, 21, 2, 13, 123, 5, 4, 0, 2, 1, 2, 6, 2, 0, 9, 9, 49, 4, 2, 1, 2, 4, 9, 9, 330, 3, 10, 1, 2, 0, 49, 6, 4, 4, 14, 10, 5350, 0, 7, 14, 11465, 27, 2343, 9, 87, 9, 39, 4, 60, 6, 26, 9, 535, 9, 470, 0, 2, 54, 8, 3, 82, 0, 12, 1, 19628, 1, 4178, 9, 519, 45, 3, 22, 543, 4, 4, 5, 9, 7, 3, 6, 31, 3, 149, 2, 1418, 49, 513, 54, 5, 49, 9, 0, 15, 0, 23, 4, 2, 14, 1361, 6, 2, 16, 3, 6, 2, 1, 2, 4, 101, 0, 161, 6, 10, 9, 357, 0, 62, 13, 499, 13, 245, 1, 2, 9, 726, 6, 110, 6, 6, 9, 4759, 9, 787719, 239];
    function isInAstralSet(code, set) {
      let pos2 = 65536;
      for (let i = 0, length = set.length; i < length; i += 2) {
        pos2 += set[i];
        if (pos2 > code) return false;
        pos2 += set[i + 1];
        if (pos2 >= code) return true;
      }
      return false;
    }
    function isIdentifierStart(code) {
      if (code < 65) return code === 36;
      if (code <= 90) return true;
      if (code < 97) return code === 95;
      if (code <= 122) return true;
      if (code <= 65535) {
        return code >= 170 && nonASCIIidentifierStart.test(String.fromCharCode(code));
      }
      return isInAstralSet(code, astralIdentifierStartCodes);
    }
    function isIdentifierChar(code) {
      if (code < 48) return code === 36;
      if (code < 58) return true;
      if (code < 65) return false;
      if (code <= 90) return true;
      if (code < 97) return code === 95;
      if (code <= 122) return true;
      if (code <= 65535) {
        return code >= 170 && nonASCIIidentifier.test(String.fromCharCode(code));
      }
      return isInAstralSet(code, astralIdentifierStartCodes) || isInAstralSet(code, astralIdentifierCodes);
    }
    function isIdentifierName2(name) {
      let isFirst = true;
      for (let i = 0; i < name.length; i++) {
        let cp2 = name.charCodeAt(i);
        if ((cp2 & 64512) === 55296 && i + 1 < name.length) {
          const trail = name.charCodeAt(++i);
          if ((trail & 64512) === 56320) {
            cp2 = 65536 + ((cp2 & 1023) << 10) + (trail & 1023);
          }
        }
        if (isFirst) {
          isFirst = false;
          if (!isIdentifierStart(cp2)) {
            return false;
          }
        } else if (!isIdentifierChar(cp2)) {
          return false;
        }
      }
      return !isFirst;
    }
  }
});

// ../../node_modules/.pnpm/@babel+helper-validator-identifier@7.25.9/node_modules/@babel/helper-validator-identifier/lib/keyword.js
var require_keyword = __commonJS({
  "../../node_modules/.pnpm/@babel+helper-validator-identifier@7.25.9/node_modules/@babel/helper-validator-identifier/lib/keyword.js"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", {
      value: true
    });
    exports2.isKeyword = isKeyword;
    exports2.isReservedWord = isReservedWord;
    exports2.isStrictBindOnlyReservedWord = isStrictBindOnlyReservedWord;
    exports2.isStrictBindReservedWord = isStrictBindReservedWord;
    exports2.isStrictReservedWord = isStrictReservedWord;
    var reservedWords = {
      keyword: ["break", "case", "catch", "continue", "debugger", "default", "do", "else", "finally", "for", "function", "if", "return", "switch", "throw", "try", "var", "const", "while", "with", "new", "this", "super", "class", "extends", "export", "import", "null", "true", "false", "in", "instanceof", "typeof", "void", "delete"],
      strict: ["implements", "interface", "let", "package", "private", "protected", "public", "static", "yield"],
      strictBind: ["eval", "arguments"]
    };
    var keywords = new Set(reservedWords.keyword);
    var reservedWordsStrictSet = new Set(reservedWords.strict);
    var reservedWordsStrictBindSet = new Set(reservedWords.strictBind);
    function isReservedWord(word, inModule) {
      return inModule && word === "await" || word === "enum";
    }
    function isStrictReservedWord(word, inModule) {
      return isReservedWord(word, inModule) || reservedWordsStrictSet.has(word);
    }
    function isStrictBindOnlyReservedWord(word) {
      return reservedWordsStrictBindSet.has(word);
    }
    function isStrictBindReservedWord(word, inModule) {
      return isStrictReservedWord(word, inModule) || isStrictBindOnlyReservedWord(word);
    }
    function isKeyword(word) {
      return keywords.has(word);
    }
  }
});

// ../../node_modules/.pnpm/@babel+helper-validator-identifier@7.25.9/node_modules/@babel/helper-validator-identifier/lib/index.js
var require_lib2 = __commonJS({
  "../../node_modules/.pnpm/@babel+helper-validator-identifier@7.25.9/node_modules/@babel/helper-validator-identifier/lib/index.js"(exports2) {
    "use strict";
    Object.defineProperty(exports2, "__esModule", {
      value: true
    });
    Object.defineProperty(exports2, "isIdentifierChar", {
      enumerable: true,
      get: function() {
        return _identifier.isIdentifierChar;
      }
    });
    Object.defineProperty(exports2, "isIdentifierName", {
      enumerable: true,
      get: function() {
        return _identifier.isIdentifierName;
      }
    });
    Object.defineProperty(exports2, "isIdentifierStart", {
      enumerable: true,
      get: function() {
        return _identifier.isIdentifierStart;
      }
    });
    Object.defineProperty(exports2, "isKeyword", {
      enumerable: true,
      get: function() {
        return _keyword.isKeyword;
      }
    });
    Object.defineProperty(exports2, "isReservedWord", {
      enumerable: true,
      get: function() {
        return _keyword.isReservedWord;
      }
    });
    Object.defineProperty(exports2, "isStrictBindOnlyReservedWord", {
      enumerable: true,
      get: function() {
        return _keyword.isStrictBindOnlyReservedWord;
      }
    });
    Object.defineProperty(exports2, "isStrictBindReservedWord", {
      enumerable: true,
      get: function() {
        return _keyword.isStrictBindReservedWord;
      }
    });
    Object.defineProperty(exports2, "isStrictReservedWord", {
      enumerable: true,
      get: function() {
        return _keyword.isStrictReservedWord;
      }
    });
    var _identifier = require_identifier();
    var _keyword = require_keyword();
  }
});

// ../../node_modules/.pnpm/env-paths@2.2.1/node_modules/env-paths/index.js
var require_env_paths = __commonJS({
  "../../node_modules/.pnpm/env-paths@2.2.1/node_modules/env-paths/index.js"(exports2, module2) {
    "use strict";
    var path10 = require("path");
    var os2 = require("os");
    var homedir = os2.homedir();
    var tmpdir = os2.tmpdir();
    var { env: env2 } = process;
    var macos = (name) => {
      const library = path10.join(homedir, "Library");
      return {
        data: path10.join(library, "Application Support", name),
        config: path10.join(library, "Preferences", name),
        cache: path10.join(library, "Caches", name),
        log: path10.join(library, "Logs", name),
        temp: path10.join(tmpdir, name)
      };
    };
    var windows2 = (name) => {
      const appData = env2.APPDATA || path10.join(homedir, "AppData", "Roaming");
      const localAppData = env2.LOCALAPPDATA || path10.join(homedir, "AppData", "Local");
      return {
        // Data/config/cache/log are invented by me as Windows isn't opinionated about this
        data: path10.join(localAppData, name, "Data"),
        config: path10.join(appData, name, "Config"),
        cache: path10.join(localAppData, name, "Cache"),
        log: path10.join(localAppData, name, "Log"),
        temp: path10.join(tmpdir, name)
      };
    };
    var linux = (name) => {
      const username = path10.basename(homedir);
      return {
        data: path10.join(env2.XDG_DATA_HOME || path10.join(homedir, ".local", "share"), name),
        config: path10.join(env2.XDG_CONFIG_HOME || path10.join(homedir, ".config"), name),
        cache: path10.join(env2.XDG_CACHE_HOME || path10.join(homedir, ".cache"), name),
        // https://wiki.debian.org/XDGBaseDirectorySpecification#state
        log: path10.join(env2.XDG_STATE_HOME || path10.join(homedir, ".local", "state"), name),
        temp: path10.join(tmpdir, username, name)
      };
    };
    var envPaths = (name, options2) => {
      if (typeof name !== "string") {
        throw new TypeError(`Expected string, got ${typeof name}`);
      }
      options2 = Object.assign({ suffix: "nodejs" }, options2);
      if (options2.suffix) {
        name += `-${options2.suffix}`;
      }
      if (process.platform === "darwin") {
        return macos(name);
      }
      if (process.platform === "win32") {
        return windows2(name);
      }
      return linux(name);
    };
    module2.exports = envPaths;
    module2.exports.default = envPaths;
  }
});

// ../ts-builders/src/KeyType.ts
var KeyType_exports = {};
__export(KeyType_exports, {
  KeyType: () => KeyType,
  keyType: () => keyType
});
function keyType(baseType, key) {
  return new KeyType(baseType, key);
}
var KeyType;
var init_KeyType = __esm({
  "../ts-builders/src/KeyType.ts"() {
    "use strict";
    init_TypeBuilder();
    KeyType = class extends TypeBuilder {
      constructor(baseType, key) {
        super();
        this.baseType = baseType;
        this.key = key;
      }
      write(writer) {
        this.baseType.writeIndexed(writer);
        writer.write("[").write(`"${this.key}"`).write("]");
      }
    };
  }
});

// ../ts-builders/src/TypeBuilder.ts
var TypeBuilder;
var init_TypeBuilder = __esm({
  "../ts-builders/src/TypeBuilder.ts"() {
    "use strict";
    TypeBuilder = class {
      // TODO(@SevInf): this should be replaced with precedence system that would
      // automatically add parenthesis where they are needed
      needsParenthesisWhenIndexed = false;
      needsParenthesisInKeyof = false;
      needsParenthesisInUnion = false;
      needsParenthesisInIntersection = false;
      subKey(key) {
        const { KeyType: KeyType2 } = (init_KeyType(), __toCommonJS(KeyType_exports));
        return new KeyType2(this, key);
      }
      writeIndexed(writer) {
        if (this.needsParenthesisWhenIndexed) {
          writer.write("(");
        }
        writer.write(this);
        if (this.needsParenthesisWhenIndexed) {
          writer.write(")");
        }
      }
    };
  }
});

// src/generation/generator.ts
var generator_exports = {};
__export(generator_exports, {
  dmmfToTypes: () => dmmfToTypes,
  externalToInternalDmmf: () => externalToInternalDmmf
});
module.exports = __toCommonJS(generator_exports);
var import_node_path6 = __toESM(require("node:path"));

// ../client-common/src/Cache.ts
var Cache = class {
  _map = /* @__PURE__ */ new Map();
  get(key) {
    return this._map.get(key)?.value;
  }
  set(key, value) {
    this._map.set(key, { value });
  }
  getOrCreate(key, create) {
    const cached = this._map.get(key);
    if (cached) {
      return cached.value;
    }
    const value = create();
    this.set(key, value);
    return value;
  }
};

// ../client-common/src/casing.ts
function capitalize(self2) {
  if (self2.length === 0) return self2;
  return self2[0].toUpperCase() + self2.slice(1);
}
function uncapitalize(self2) {
  return self2.substring(0, 1).toLowerCase() + self2.substring(1);
}

// ../client-common/src/Dictionary.ts
function keyBy(collection, prop) {
  const acc = {};
  for (const obj of collection) {
    const key = obj[prop];
    acc[key] = obj;
  }
  return acc;
}

// ../client-common/src/enums.ts
var strictEnumNames = ["TransactionIsolationLevel"];
var objectEnumNames = ["JsonNullValueInput", "NullableJsonNullValueInput", "JsonNullValueFilter"];

// ../client-common/src/runtimeDataModel.ts
function dmmfToRuntimeDataModel(dmmfDataModel) {
  return {
    models: buildMapForRuntime(dmmfDataModel.models),
    enums: buildMapForRuntime(dmmfDataModel.enums),
    types: buildMapForRuntime(dmmfDataModel.types)
  };
}
function pruneRuntimeDataModel({ models }) {
  const prunedModels = {};
  for (const modelName of Object.keys(models)) {
    prunedModels[modelName] = { fields: [], dbName: models[modelName].dbName };
    for (const { name, kind, type, relationName, dbName } of models[modelName].fields) {
      prunedModels[modelName].fields.push({ name, kind, type, relationName, dbName });
    }
  }
  return { models: prunedModels, enums: {}, types: {} };
}
function buildMapForRuntime(list) {
  const result = {};
  for (const { name, ...rest } of list) {
    result[name] = rest;
  }
  return result;
}

// ../client-common/src/uniqueBy.ts
function uniqueBy(arr, callee) {
  const result = {};
  for (const value of arr) {
    const hash = callee(value);
    if (!result[hash]) {
      result[hash] = value;
    }
  }
  return Object.values(result);
}

// ../client-generator-js/src/externalToInternalDmmf.ts
var import_pluralize = __toESM(require_pluralize());
function externalToInternalDmmf(document) {
  return {
    ...document,
    mappings: getMappings(document.mappings, document.datamodel)
  };
}
function getMappings(mappings, datamodel) {
  const modelOperations = mappings.modelOperations.filter((mapping) => {
    const model = datamodel.models.find((m) => m.name === mapping.model);
    if (!model) {
      throw new Error(`Mapping without model ${mapping.model}`);
    }
    return model.fields.some((f) => f.kind !== "object");
  }).map((mapping) => ({
    model: mapping.model,
    plural: (0, import_pluralize.default)(uncapitalize(mapping.model)),
    // TODO not needed anymore
    findUnique: mapping.findUnique || mapping.findSingle,
    findUniqueOrThrow: mapping.findUniqueOrThrow,
    findFirst: mapping.findFirst,
    findFirstOrThrow: mapping.findFirstOrThrow,
    findMany: mapping.findMany,
    create: mapping.createOne || mapping.createSingle || mapping.create,
    createMany: mapping.createMany,
    createManyAndReturn: mapping.createManyAndReturn,
    delete: mapping.deleteOne || mapping.deleteSingle || mapping.delete,
    update: mapping.updateOne || mapping.updateSingle || mapping.update,
    deleteMany: mapping.deleteMany,
    updateMany: mapping.updateMany,
    updateManyAndReturn: mapping.updateManyAndReturn,
    upsert: mapping.upsertOne || mapping.upsertSingle || mapping.upsert,
    aggregate: mapping.aggregate,
    groupBy: mapping.groupBy,
    findRaw: mapping.findRaw,
    aggregateRaw: mapping.aggregateRaw
  }));
  return {
    modelOperations,
    otherOperations: mappings.otherOperations
  };
}

// ../../node_modules/.pnpm/kleur@4.1.5/node_modules/kleur/colors.mjs
var colors_exports = {};
__export(colors_exports, {
  $: () => $,
  bgBlack: () => bgBlack,
  bgBlue: () => bgBlue,
  bgCyan: () => bgCyan,
  bgGreen: () => bgGreen,
  bgMagenta: () => bgMagenta,
  bgRed: () => bgRed,
  bgWhite: () => bgWhite,
  bgYellow: () => bgYellow,
  black: () => black,
  blue: () => blue,
  bold: () => bold,
  cyan: () => cyan,
  dim: () => dim,
  gray: () => gray,
  green: () => green,
  grey: () => grey,
  hidden: () => hidden,
  inverse: () => inverse,
  italic: () => italic,
  magenta: () => magenta,
  red: () => red,
  reset: () => reset,
  strikethrough: () => strikethrough,
  underline: () => underline,
  white: () => white,
  yellow: () => yellow
});
var FORCE_COLOR;
var NODE_DISABLE_COLORS;
var NO_COLOR;
var TERM;
var isTTY = true;
if (typeof process !== "undefined") {
  ({ FORCE_COLOR, NODE_DISABLE_COLORS, NO_COLOR, TERM } = process.env || {});
  isTTY = process.stdout && process.stdout.isTTY;
}
var $ = {
  enabled: !NODE_DISABLE_COLORS && NO_COLOR == null && TERM !== "dumb" && (FORCE_COLOR != null && FORCE_COLOR !== "0" || isTTY)
};
function init(x, y) {
  let rgx = new RegExp(`\\x1b\\[${y}m`, "g");
  let open = `\x1B[${x}m`, close = `\x1B[${y}m`;
  return function(txt) {
    if (!$.enabled || txt == null) return txt;
    return open + (!!~("" + txt).indexOf(close) ? txt.replace(rgx, close + open) : txt) + close;
  };
}
var reset = init(0, 0);
var bold = init(1, 22);
var dim = init(2, 22);
var italic = init(3, 23);
var underline = init(4, 24);
var inverse = init(7, 27);
var hidden = init(8, 28);
var strikethrough = init(9, 29);
var black = init(30, 39);
var red = init(31, 39);
var green = init(32, 39);
var yellow = init(33, 39);
var blue = init(34, 39);
var magenta = init(35, 39);
var cyan = init(36, 39);
var white = init(37, 39);
var gray = init(90, 39);
var grey = init(90, 39);
var bgBlack = init(40, 49);
var bgRed = init(41, 49);
var bgGreen = init(42, 49);
var bgYellow = init(43, 49);
var bgBlue = init(44, 49);
var bgMagenta = init(45, 49);
var bgCyan = init(46, 49);
var bgWhite = init(47, 49);

// ../internals/src/utils/parseEnvValue.ts
function parseEnvValue(object) {
  if (object.fromEnvVar && object.fromEnvVar != "null") {
    const value = process.env[object.fromEnvVar];
    if (!value) {
      throw new Error(
        `Attempted to load provider value using \`env(${object.fromEnvVar})\` but it was not present. Please ensure that ${dim(
          object.fromEnvVar
        )} is present in your Environment Variables`
      );
    }
    return value;
  }
  return object.value;
}

// ../debug/src/index.ts
var MAX_ARGS_HISTORY = 100;
var COLORS = ["green", "yellow", "blue", "magenta", "cyan", "red"];
var argsHistory = [];
var lastTimestamp = Date.now();
var lastColor = 0;
var processEnv = typeof process !== "undefined" ? process.env : {};
globalThis.DEBUG ??= processEnv.DEBUG ?? "";
globalThis.DEBUG_COLORS ??= processEnv.DEBUG_COLORS ? processEnv.DEBUG_COLORS === "true" : true;
var topProps = {
  enable(namespace2) {
    if (typeof namespace2 === "string") {
      globalThis.DEBUG = namespace2;
    }
  },
  disable() {
    const prev = globalThis.DEBUG;
    globalThis.DEBUG = "";
    return prev;
  },
  // this is the core logic to check if logging should happen or not
  enabled(namespace2) {
    const listenedNamespaces = globalThis.DEBUG.split(",").map((s) => {
      return s.replace(/[.+?^${}()|[\]\\]/g, "\\$&");
    });
    const isListened = listenedNamespaces.some((listenedNamespace) => {
      if (listenedNamespace === "" || listenedNamespace[0] === "-") return false;
      return namespace2.match(RegExp(listenedNamespace.split("*").join(".*") + "$"));
    });
    const isExcluded = listenedNamespaces.some((listenedNamespace) => {
      if (listenedNamespace === "" || listenedNamespace[0] !== "-") return false;
      return namespace2.match(RegExp(listenedNamespace.slice(1).split("*").join(".*") + "$"));
    });
    return isListened && !isExcluded;
  },
  log: (...args) => {
    const [namespace2, format, ...rest] = args;
    const logWithFormatting = console.warn ?? console.log;
    logWithFormatting(`${namespace2} ${format}`, ...rest);
  },
  formatters: {}
  // not implemented
};
function debugCreate(namespace2) {
  const instanceProps = {
    color: COLORS[lastColor++ % COLORS.length],
    enabled: topProps.enabled(namespace2),
    namespace: namespace2,
    log: topProps.log,
    extend: () => {
    }
    // not implemented
  };
  const debugCall = (...args) => {
    const { enabled, namespace: namespace3, color: color2, log } = instanceProps;
    if (args.length !== 0) {
      argsHistory.push([namespace3, ...args]);
    }
    if (argsHistory.length > MAX_ARGS_HISTORY) {
      argsHistory.shift();
    }
    if (topProps.enabled(namespace3) || enabled) {
      const stringArgs = args.map((arg) => {
        if (typeof arg === "string") {
          return arg;
        }
        return safeStringify(arg);
      });
      const ms = `+${Date.now() - lastTimestamp}ms`;
      lastTimestamp = Date.now();
      if (globalThis.DEBUG_COLORS) {
        log(colors_exports[color2](bold(namespace3)), ...stringArgs, colors_exports[color2](ms));
      } else {
        log(namespace3, ...stringArgs, ms);
      }
    }
  };
  return new Proxy(debugCall, {
    get: (_, prop) => instanceProps[prop],
    set: (_, prop, value) => instanceProps[prop] = value
  });
}
var Debug = new Proxy(debugCreate, {
  get: (_, prop) => topProps[prop],
  set: (_, prop, value) => topProps[prop] = value
});
function safeStringify(value, indent8 = 2) {
  const cache = /* @__PURE__ */ new Set();
  return JSON.stringify(
    value,
    (key, value2) => {
      if (typeof value2 === "object" && value2 !== null) {
        if (cache.has(value2)) {
          return `[Circular *]`;
        }
        cache.add(value2);
      } else if (typeof value2 === "bigint") {
        return value2.toString();
      }
      return value2;
    },
    indent8
  );
}

// ../generator-helper/src/generatorHandler.ts
var import_node_readline = __toESM(require("node:readline"));
function generatorHandler(handler) {
  const stdinInterface = import_node_readline.default.createInterface({
    input: process.stdin,
    crlfDelay: Infinity
  });
  stdinInterface.on("line", async (line) => {
    const json = JSON.parse(line);
    if (json.method === "generate" && json.params) {
      try {
        const result = await handler.onGenerate(json.params);
        respond({
          jsonrpc: "2.0",
          result,
          id: json.id
        });
      } catch (_e) {
        const e = _e;
        respond({
          jsonrpc: "2.0",
          error: {
            code: -32e3,
            message: e.message,
            data: {
              stack: e.stack
            }
          },
          id: json.id
        });
      }
    }
    if (json.method === "getManifest") {
      if (handler.onManifest) {
        try {
          const manifest = await handler.onManifest(json.params);
          respond({
            jsonrpc: "2.0",
            result: {
              manifest
            },
            id: json.id
          });
        } catch (_e) {
          const e = _e;
          respond({
            jsonrpc: "2.0",
            error: {
              code: -32e3,
              message: e.message,
              data: {
                stack: e.stack
              }
            },
            id: json.id
          });
        }
      } else {
        respond({
          jsonrpc: "2.0",
          result: {
            manifest: null
          },
          id: json.id
        });
      }
    }
  });
  process.stdin.resume();
}
function respond(response) {
  process.stderr.write(JSON.stringify(response) + "\n");
}

// ../internals/src/resolvePkg.ts
var import_path = __toESM(require("path"));
var import_resolve = __toESM(require_resolve());
async function resolveOrUndefined(id, options2) {
  const _options = { preserveSymlinks: false, ...options2 };
  return new Promise((res) => {
    (0, import_resolve.default)(id, _options, (e, v) => {
      if (e) res(void 0);
      res(v);
    });
  });
}
async function resolvePkg(id, options2) {
  const resolvedPath = await resolveOrUndefined(`${id}/package.json`, options2);
  return resolvedPath && import_path.default.dirname(resolvedPath);
}

// ../internals/src/utils/assertNever.ts
function assertNever(arg, errorMessage) {
  throw new Error(errorMessage);
}

// ../internals/src/utils/path.ts
var import_path2 = __toESM(require("path"));
function pathToPosix(filePath) {
  if (import_path2.default.sep === import_path2.default.posix.sep) {
    return filePath;
  }
  return filePath.split(import_path2.default.sep).join(import_path2.default.posix.sep);
}

// ../internals/src/utils/hasOwnProperty.ts
function hasOwnProperty(object, key) {
  return Object.prototype.hasOwnProperty.call(object, key);
}

// ../internals/src/utils/isValidJsIdentifier.ts
var import_helper_validator_identifier = __toESM(require_lib2());
function isValidJsIdentifier(str) {
  return (0, import_helper_validator_identifier.isIdentifierName)(str);
}

// ../internals/src/utils/setClassName.ts
function setClassName(classObject, name) {
  Object.defineProperty(classObject, "name", {
    value: name,
    configurable: true
  });
}

// ../client-generator-js/src/generateClient.ts
var import_crypto = require("crypto");
var import_env_paths = __toESM(require_env_paths());
var import_promises2 = __toESM(require("fs/promises"));
var import_fs_extra = __toESM(require_lib());

// ../../node_modules/.pnpm/package-up@5.0.0/node_modules/package-up/index.js
var import_node_process2 = __toESM(require("node:process"), 1);

// ../../node_modules/.pnpm/find-up-simple@1.0.1/node_modules/find-up-simple/index.js
var import_node_process = __toESM(require("node:process"), 1);
var import_promises = __toESM(require("node:fs/promises"), 1);
var import_node_url = require("node:url");
var import_node_path = __toESM(require("node:path"), 1);
var toPath = (urlOrPath) => urlOrPath instanceof URL ? (0, import_node_url.fileURLToPath)(urlOrPath) : urlOrPath;
async function findUp(name, {
  cwd = import_node_process.default.cwd(),
  type = "file",
  stopAt
} = {}) {
  let directory = import_node_path.default.resolve(toPath(cwd) ?? "");
  const { root } = import_node_path.default.parse(directory);
  stopAt = import_node_path.default.resolve(directory, toPath(stopAt ?? root));
  const isAbsoluteName = import_node_path.default.isAbsolute(name);
  while (directory) {
    const filePath = isAbsoluteName ? name : import_node_path.default.join(directory, name);
    try {
      const stats = await import_promises.default.stat(filePath);
      if (type === "file" && stats.isFile() || type === "directory" && stats.isDirectory()) {
        return filePath;
      }
    } catch {
    }
    if (directory === stopAt || directory === root) {
      break;
    }
    directory = import_node_path.default.dirname(directory);
  }
}

// ../../node_modules/.pnpm/package-up@5.0.0/node_modules/package-up/index.js
async function packageUp({ cwd = import_node_process2.default.cwd() } = {}) {
  return findUp("package.json", { cwd });
}

// ../client-generator-js/src/generateClient.ts
var import_path4 = __toESM(require("path"));

// package.json
var package_default = {
  name: "@prisma/client",
  version: "7.4.2",
  description: "Prisma Client is an auto-generated, type-safe and modern JavaScript/TypeScript ORM for Node.js that's tailored to your data. Supports PostgreSQL, CockroachDB, MySQL, MariaDB, SQL Server, SQLite & MongoDB databases.",
  keywords: [
    "ORM",
    "Prisma",
    "prisma2",
    "Prisma Client",
    "client",
    "query",
    "query-builder",
    "database",
    "db",
    "JavaScript",
    "JS",
    "TypeScript",
    "TS",
    "SQL",
    "SQLite",
    "pg",
    "Postgres",
    "PostgreSQL",
    "CockroachDB",
    "MySQL",
    "MariaDB",
    "MSSQL",
    "SQL Server",
    "SQLServer",
    "MongoDB"
  ],
  main: "default.js",
  types: "default.d.ts",
  browser: "index-browser.js",
  exports: {
    "./package.json": "./package.json",
    ".": {
      require: {
        types: "./default.d.ts",
        node: "./default.js",
        "edge-light": "./default.js",
        workerd: "./default.js",
        worker: "./default.js",
        browser: "./index-browser.js"
      },
      import: {
        types: "./default.d.ts",
        node: "./default.js",
        "edge-light": "./default.js",
        workerd: "./default.js",
        worker: "./default.js",
        browser: "./index-browser.js"
      },
      default: "./default.js"
    },
    "./extension": {
      types: "./extension.d.ts",
      require: "./extension.js",
      import: "./extension.js",
      default: "./extension.js"
    },
    "./index-browser": {
      types: "./index.d.ts",
      require: "./index-browser.js",
      import: "./index-browser.js",
      default: "./index-browser.js"
    },
    "./index": {
      types: "./index.d.ts",
      require: "./index.js",
      import: "./index.js",
      default: "./index.js"
    },
    "./edge": {
      types: "./edge.d.ts",
      require: "./edge.js",
      import: "./edge.js",
      default: "./edge.js"
    },
    "./runtime/client": {
      types: "./runtime/client.d.ts",
      node: {
        require: "./runtime/client.js",
        default: "./runtime/client.js"
      },
      require: "./runtime/client.js",
      import: "./runtime/client.mjs",
      default: "./runtime/client.mjs"
    },
    "./runtime/wasm-compiler-edge": {
      types: "./runtime/wasm-compiler-edge.d.ts",
      require: "./runtime/wasm-compiler-edge.js",
      import: "./runtime/wasm-compiler-edge.mjs",
      default: "./runtime/wasm-compiler-edge.mjs"
    },
    "./runtime/index-browser": {
      types: "./runtime/index-browser.d.ts",
      require: "./runtime/index-browser.js",
      import: "./runtime/index-browser.mjs",
      default: "./runtime/index-browser.mjs"
    },
    "./generator-build": {
      require: "./generator-build/index.js",
      import: "./generator-build/index.js",
      default: "./generator-build/index.js"
    },
    "./sql": {
      require: {
        types: "./sql.d.ts",
        node: "./sql.js",
        default: "./sql.js"
      },
      import: {
        types: "./sql.d.ts",
        node: "./sql.mjs",
        default: "./sql.mjs"
      },
      default: "./sql.js"
    },
    "./*": "./*"
  },
  license: "Apache-2.0",
  engines: {
    node: "^20.19 || ^22.12 || >=24.0"
  },
  homepage: "https://www.prisma.io",
  repository: {
    type: "git",
    url: "https://github.com/prisma/prisma.git",
    directory: "packages/client"
  },
  author: "Tim Suchanek <suchanek@prisma.io>",
  bugs: "https://github.com/prisma/prisma/issues",
  scripts: {
    dev: "DEV=true tsx helpers/build.ts",
    build: "tsx helpers/build.ts",
    test: "dotenv -e ../../.db.env -- jest --silent",
    "test:e2e": "dotenv -e ../../.db.env -- tsx tests/e2e/_utils/run.ts",
    "test:functional": "dotenv -e ../../.db.env -- tsx helpers/functional-test/run-tests.ts",
    "test:functional:client": "pnpm run test:functional --client-runtime client",
    "test:functional:code": "dotenv -e ../../.db.env -- tsx helpers/functional-test/run-tests.ts --no-types",
    "test:functional:types": "dotenv -e ../../.db.env -- tsx helpers/functional-test/run-tests.ts --types-only",
    "test-notypes": "dotenv -e ../../.db.env -- jest --testPathIgnorePatterns src/__tests__/types/types.test.ts",
    prepublishOnly: "pnpm run build",
    "new-test": "tsx ./helpers/new-test/new-test.ts"
  },
  files: [
    "README.md",
    "runtime",
    "scripts",
    "generator-build",
    "edge.js",
    "edge.d.ts",
    "index.js",
    "index.d.ts",
    "default.js",
    "default.d.ts",
    "index-browser.js",
    "extension.js",
    "extension.d.ts",
    "sql.d.ts",
    "sql.js",
    "sql.mjs"
  ],
  devDependencies: {
    "@cloudflare/workers-types": "^4.20251014.0",
    "@codspeed/benchmark.js-plugin": "4.0.0",
    "@faker-js/faker": "9.6.0",
    "@fast-check/jest": "2.0.3",
    "@hono/node-server": "1.19.0",
    "@inquirer/prompts": "7.3.3",
    "@jest/create-cache-key-function": "29.7.0",
    "@jest/globals": "29.7.0",
    "@jest/test-sequencer": "29.7.0",
    "@libsql/client": "0.8.1",
    "@neondatabase/serverless": "0.10.2",
    "@opentelemetry/api": "1.9.0",
    "@opentelemetry/context-async-hooks": "2.1.0",
    "@opentelemetry/instrumentation": "0.206.0",
    "@opentelemetry/resources": "2.1.0",
    "@opentelemetry/sdk-trace-base": "2.1.0",
    "@opentelemetry/semantic-conventions": "1.37.0",
    "@planetscale/database": "1.19.0",
    "@prisma/adapter-better-sqlite3": "workspace:*",
    "@prisma/adapter-d1": "workspace:*",
    "@prisma/adapter-libsql": "workspace:*",
    "@prisma/adapter-mariadb": "workspace:*",
    "@prisma/adapter-mssql": "workspace:*",
    "@prisma/adapter-neon": "workspace:*",
    "@prisma/adapter-pg": "workspace:*",
    "@prisma/adapter-planetscale": "workspace:*",
    "@prisma/client-common": "workspace:*",
    "@prisma/client-engine-runtime": "workspace:*",
    "@prisma/client-generator-js": "workspace:*",
    "@prisma/client-generator-ts": "workspace:*",
    "@prisma/config": "workspace:*",
    "@prisma/debug": "workspace:*",
    "@prisma/dmmf": "workspace:*",
    "@prisma/driver-adapter-utils": "workspace:*",
    "@prisma/engines": "workspace:*",
    "@prisma/engines-version": "7.5.0-10.94a226be1cf2967af2541cca5529f0f7ba866919",
    "@prisma/fetch-engine": "workspace:*",
    "@prisma/generator": "workspace:*",
    "@prisma/generator-helper": "workspace:*",
    "@prisma/get-platform": "workspace:*",
    "@prisma/instrumentation": "workspace:*",
    "@prisma/instrumentation-contract": "workspace:*",
    "@prisma/internals": "workspace:*",
    "@prisma/json-protocol": "workspace:*",
    "@prisma/migrate": "workspace:*",
    "@prisma/param-graph": "workspace:*",
    "@prisma/param-graph-builder": "workspace:*",
    "@prisma/query-compiler-wasm": "7.5.0-10.94a226be1cf2967af2541cca5529f0f7ba866919",
    "@prisma/query-plan-executor": "workspace:*",
    "@prisma/sqlcommenter": "workspace:*",
    "@prisma/sqlcommenter-trace-context": "workspace:*",
    "@prisma/ts-builders": "workspace:*",
    "@snaplet/copycat": "6.0.0",
    "@swc-node/register": "1.10.9",
    "@swc/core": "1.11.5",
    "@swc/jest": "0.2.37",
    "@timsuchanek/copy": "1.4.5",
    "@types/debug": "4.1.12",
    "@types/fs-extra": "11.0.4",
    "@types/jest": "29.5.14",
    "@types/js-levenshtein": "1.1.3",
    "@types/mssql": "9.1.8",
    "@types/node": "~20.19.24",
    "@types/pg": "8.11.11",
    arg: "5.0.2",
    benchmark: "2.1.4",
    "cookie-es": "2.0.0",
    execa: "8.0.1",
    "expect-type": "1.2.2",
    "fs-extra": "11.3.0",
    "get-stream": "6.0.1",
    globby: "11.1.0",
    "indent-string": "4.0.0",
    jest: "29.7.0",
    "jest-extended": "4.0.2",
    "jest-junit": "16.0.0",
    "jest-serializer-ansi-escapes": "4.0.0",
    "jest-snapshot": "29.7.0",
    "js-levenshtein": "1.1.6",
    kleur: "4.1.5",
    klona: "2.0.6",
    mariadb: "3.4.5",
    memfs: "4.17.2",
    mssql: "11.0.1",
    "new-github-issue-url": "0.2.1",
    "p-retry": "4.6.2",
    pg: "8.14.1",
    resolve: "1.22.10",
    "simple-statistics": "7.8.8",
    "sort-keys": "5.1.0",
    "source-map-support": "0.5.21",
    "stacktrace-parser": "0.1.11",
    "strip-ansi": "7.1.0",
    "strip-indent": "4.0.0",
    tempy: "3.0.0",
    "ts-pattern": "5.6.2",
    tsd: "0.31.2",
    typescript: "5.4.5",
    undici: "7.4.0",
    zx: "8.4.1"
  },
  dependencies: {
    "@prisma/client-runtime-utils": "workspace:*"
  },
  peerDependencies: {
    prisma: "*",
    typescript: ">=5.4.0"
  },
  peerDependenciesMeta: {
    prisma: {
      optional: true
    },
    typescript: {
      optional: true
    }
  },
  sideEffects: false
};

// ../client-generator-js/src/getDMMF.ts
function getPrismaClientDMMF(dmmf) {
  return externalToInternalDmmf(dmmf);
}

// ../client-generator-js/src/TSClient/Enum.ts
var import_indent_string = __toESM(require_indent_string());

// ../client-generator-js/src/TSClient/constants.ts
var TAB_SIZE = 2;

// ../client-generator-js/src/TSClient/Enum.ts
var Enum = class {
  constructor(type, useNamespace) {
    this.type = type;
    this.useNamespace = useNamespace;
  }
  isObjectEnum() {
    return this.useNamespace && objectEnumNames.includes(this.type.name);
  }
  isStrictEnum() {
    return this.useNamespace && strictEnumNames.includes(this.type.name);
  }
  toJS() {
    const { type } = this;
    const enumVariants = `{
${(0, import_indent_string.default)(type.values.map((v) => `${v}: ${this.getValueJS(v)}`).join(",\n"), TAB_SIZE)}
}`;
    const enumBody = this.isStrictEnum() ? `makeStrictEnum(${enumVariants})` : enumVariants;
    return this.useNamespace ? `exports.Prisma.${type.name} = ${enumBody};` : `exports.${type.name} = exports.$Enums.${type.name} = ${enumBody};`;
  }
  getValueJS(value) {
    return this.isObjectEnum() ? `Prisma.${value}` : `'${value}'`;
  }
  toTS() {
    const { type } = this;
    return `export const ${type.name}: {
${(0, import_indent_string.default)(type.values.map((v) => `${v}: ${this.getValueTS(v)}`).join(",\n"), TAB_SIZE)}
};

export type ${type.name} = (typeof ${type.name})[keyof typeof ${type.name}]
`;
  }
  getValueTS(value) {
    return this.isObjectEnum() ? `typeof ${value}` : `'${value}'`;
  }
};

// ../client-generator-js/src/TSClient/Generable.ts
function JS(gen) {
  return gen.toJS?.() ?? "";
}
function BrowserJS(gen) {
  return gen.toBrowserJS?.() ?? "";
}
function TS(gen) {
  return gen.toTS();
}

// ../ts-builders/src/ArraySpread.ts
init_TypeBuilder();
var ArraySpread = class extends TypeBuilder {
  constructor(innerType) {
    super();
    this.innerType = innerType;
  }
  write(writer) {
    writer.write("[...").write(this.innerType).write("]");
  }
};
function arraySpread(innerType) {
  return new ArraySpread(innerType);
}

// ../ts-builders/src/ArrayType.ts
init_TypeBuilder();
var ArrayType = class extends TypeBuilder {
  constructor(elementType) {
    super();
    this.elementType = elementType;
  }
  write(writer) {
    this.elementType.writeIndexed(writer);
    writer.write("[]");
  }
};
function array(elementType) {
  return new ArrayType(elementType);
}

// ../ts-builders/src/ConditionalType.ts
init_TypeBuilder();
var ConditionalType = class extends TypeBuilder {
  needsParenthesisInUnion = true;
  needsParenthesisInIntersection = true;
  #checkType;
  #extendsType;
  #trueType;
  #falseType;
  constructor(checkType2, extendsType, trueType, falseType) {
    super();
    this.#checkType = checkType2;
    this.#extendsType = extendsType;
    this.#trueType = trueType;
    this.#falseType = falseType;
  }
  write(writer) {
    writer.write(this.#checkType);
    writer.write(" extends ");
    writer.write(this.#extendsType);
    writer.write(" ? ");
    writer.write(this.#trueType);
    writer.write(" : ");
    writer.write(this.#falseType);
  }
};
var ConditionalTypeBuilder = class {
  check(checkType2) {
    return new ConditionalTypeBuilderWithCheckType(checkType2);
  }
};
var ConditionalTypeBuilderWithCheckType = class {
  #checkType;
  constructor(checkType2) {
    this.#checkType = checkType2;
  }
  extends(extendsType) {
    return new ConditionalTypeBuilderWithExtendsType(this.#checkType, extendsType);
  }
};
var ConditionalTypeBuilderWithExtendsType = class {
  #checkType;
  #extendsType;
  constructor(checkType2, extendsType) {
    this.#checkType = checkType2;
    this.#extendsType = extendsType;
  }
  then(trueType) {
    return new ConditionalTypeBuilderWithTrueType(this.#checkType, this.#extendsType, trueType);
  }
};
var ConditionalTypeBuilderWithTrueType = class {
  #checkType;
  #extendsType;
  #trueType;
  constructor(checkType2, extendsType, trueType) {
    this.#checkType = checkType2;
    this.#extendsType = extendsType;
    this.#trueType = trueType;
  }
  else(falseType) {
    return new ConditionalType(this.#checkType, this.#extendsType, this.#trueType, falseType);
  }
};
function conditionalType() {
  return new ConditionalTypeBuilder();
}

// ../ts-builders/src/ConstDeclaration.ts
var ConstDeclaration = class {
  constructor(name, type) {
    this.name = name;
    this.type = type;
  }
  docComment;
  value;
  setDocComment(docComment2) {
    this.docComment = docComment2;
    return this;
  }
  setValue(value) {
    this.value = value;
    return this;
  }
  write(writer) {
    if (this.docComment) {
      writer.write(this.docComment);
    }
    writer.write("const ").write(this.name);
    if (this.type) {
      writer.write(": ").write(this.type);
    }
    if (this.value) {
      writer.write(" = ").write(this.value);
    }
  }
};
function constDeclaration(name, type) {
  return new ConstDeclaration(name, type);
}

// ../ts-builders/src/DocComment.ts
var DocComment = class {
  lines = [];
  constructor(startingText) {
    if (startingText) {
      this.addText(startingText);
    }
  }
  addText(text2) {
    this.lines.push(...text2.split("\n"));
    return this;
  }
  write(writer) {
    writer.writeLine("/**");
    for (const line of this.lines) {
      writer.writeLine(` * ${line}`);
    }
    writer.writeLine(" */");
    return writer;
  }
};
function docComment(firstParameter, ...args) {
  if (typeof firstParameter === "string" || typeof firstParameter === "undefined") {
    return new DocComment(firstParameter);
  }
  return docCommentTag(firstParameter, args);
}
function docCommentTag(strings, args) {
  const docComment2 = new DocComment();
  const fullText = strings.flatMap((str, i) => {
    if (i < args.length) {
      return [str, args[i]];
    }
    return [str];
  }).join("");
  const lines2 = trimEmptyLines(fullText.split("\n"));
  if (lines2.length === 0) {
    return docComment2;
  }
  const indent8 = getIndent(lines2[0]);
  for (const line of lines2) {
    docComment2.addText(line.slice(indent8));
  }
  return docComment2;
}
function trimEmptyLines(lines2) {
  const firstLine = findFirstNonEmptyLine(lines2);
  const lastLine = findLastNonEmptyLine(lines2);
  if (firstLine === -1 || lastLine === -1) {
    return [];
  }
  return lines2.slice(firstLine, lastLine + 1);
}
function findFirstNonEmptyLine(lines2) {
  return lines2.findIndex((line) => !isEmptyLine(line));
}
function findLastNonEmptyLine(lines2) {
  let i = lines2.length - 1;
  while (i > 0 && isEmptyLine(lines2[i])) {
    i--;
  }
  return i;
}
function isEmptyLine(line) {
  return line.trim().length === 0;
}
function getIndent(line) {
  let indent8 = 0;
  while (line[indent8] === " ") {
    indent8++;
  }
  return indent8;
}

// ../ts-builders/src/Export.ts
var Export = class {
  constructor(declaration) {
    this.declaration = declaration;
  }
  docComment;
  setDocComment(docComment2) {
    this.docComment = docComment2;
    return this;
  }
  write(writer) {
    if (this.docComment) {
      writer.write(this.docComment);
    }
    writer.write("export ").write(this.declaration);
  }
};
function moduleExport(declaration) {
  return new Export(declaration);
}

// ../ts-builders/src/ExportFrom.ts
var NamespaceExport = class {
  constructor(from, namespace2) {
    this.from = from;
    this.namespace = namespace2;
  }
  write(writer) {
    writer.write(`export * as ${this.namespace} from '${this.from}'`);
  }
};
var BindingsExport = class {
  constructor(from) {
    this.from = from;
  }
  namedExports = [];
  named(namedExport) {
    if (typeof namedExport === "string") {
      namedExport = new NamedExport(namedExport);
    }
    this.namedExports.push(namedExport);
    return this;
  }
  write(writer) {
    writer.write("export ").write("{ ").writeJoined(", ", this.namedExports).write(" }").write(` from "${this.from}"`);
  }
};
var NamedExport = class {
  constructor(name) {
    this.name = name;
  }
  alias;
  type = false;
  as(alias) {
    this.alias = alias;
    return this;
  }
  typeOnly() {
    this.type = true;
    return this;
  }
  write(writer) {
    if (this.type) {
      writer.write("type ");
    }
    writer.write(this.name);
    if (this.alias) {
      writer.write(" as ").write(this.alias);
    }
  }
};
var ExportAllFrom = class {
  constructor(from) {
    this.from = from;
  }
  asNamespace(namespace2) {
    return new NamespaceExport(this.from, namespace2);
  }
  named(binding) {
    return new BindingsExport(this.from).named(binding);
  }
  write(writer) {
    writer.write(`export * from "${this.from}"`);
  }
};
function moduleExportFrom(from) {
  return new ExportAllFrom(from);
}

// ../ts-builders/src/File.ts
var File = class {
  imports = [];
  declarations = [];
  addImport(moduleImport2) {
    this.imports.push(moduleImport2);
    return this;
  }
  add(declaration) {
    this.declarations.push(declaration);
  }
  write(writer) {
    for (const moduleImport2 of this.imports) {
      writer.writeLine(moduleImport2);
    }
    if (this.imports.length > 0) {
      writer.newLine();
    }
    for (const [i, declaration] of this.declarations.entries()) {
      writer.writeLine(declaration);
      if (i < this.declarations.length - 1) {
        writer.newLine();
      }
    }
  }
};
function file() {
  return new File();
}

// ../ts-builders/src/ValueBuilder.ts
var ValueBuilder = class {
  as(type) {
    return new TypeAssertion(this, type);
  }
};
var TypeAssertion = class extends ValueBuilder {
  #value;
  #type;
  constructor(value, type) {
    super();
    this.#value = value;
    this.#type = type;
  }
  write(writer) {
    writer.write(this.#value).write(" as ").write(this.#type);
  }
};

// ../ts-builders/src/PrimitiveType.ts
init_TypeBuilder();
var PrimitiveType = class extends TypeBuilder {
  constructor(name) {
    super();
    this.name = name;
  }
  write(writer) {
    writer.write(this.name);
  }
};
var stringType = new PrimitiveType("string");
var numberType = new PrimitiveType("number");
var booleanType = new PrimitiveType("boolean");
var nullType = new PrimitiveType("null");
var undefinedType = new PrimitiveType("undefined");
var bigintType = new PrimitiveType("bigint");
var unknownType = new PrimitiveType("unknown");
var anyType = new PrimitiveType("any");
var voidType = new PrimitiveType("void");
var thisType = new PrimitiveType("this");
var neverType = new PrimitiveType("never");

// ../ts-builders/src/FunctionType.ts
init_TypeBuilder();
var FunctionType = class extends TypeBuilder {
  needsParenthesisWhenIndexed = true;
  needsParenthesisInKeyof = true;
  needsParenthesisInUnion = true;
  needsParenthesisInIntersection = true;
  returnType = voidType;
  parameters = [];
  genericParameters = [];
  setReturnType(returnType) {
    this.returnType = returnType;
    return this;
  }
  addParameter(param) {
    this.parameters.push(param);
    return this;
  }
  addGenericParameter(param) {
    this.genericParameters.push(param);
    return this;
  }
  write(writer) {
    if (this.genericParameters.length > 0) {
      writer.write("<").writeJoined(", ", this.genericParameters).write(">");
    }
    writer.write("(").writeJoined(", ", this.parameters).write(") => ").write(this.returnType);
  }
};
function functionType() {
  return new FunctionType();
}

// ../ts-builders/src/NamedType.ts
init_TypeBuilder();
var NamedType = class extends TypeBuilder {
  constructor(name) {
    super();
    this.name = name;
  }
  genericArguments = [];
  addGenericArgument(type) {
    this.genericArguments.push(type);
    return this;
  }
  write(writer) {
    writer.write(this.name);
    if (this.genericArguments.length > 0) {
      writer.write("<").writeJoined(", ", this.genericArguments).write(">");
    }
  }
};
function namedType(name) {
  return new NamedType(name);
}

// ../ts-builders/src/GenericParameter.ts
var GenericParameter = class {
  constructor(name) {
    this.name = name;
  }
  extendedType;
  defaultType;
  extends(type) {
    this.extendedType = type;
    return this;
  }
  default(type) {
    this.defaultType = type;
    return this;
  }
  toArgument() {
    return new NamedType(this.name);
  }
  write(writer) {
    writer.write(this.name);
    if (this.extendedType) {
      writer.write(" extends ").write(this.extendedType);
    }
    if (this.defaultType) {
      writer.write(" = ").write(this.defaultType);
    }
  }
};
function genericParameter(name) {
  return new GenericParameter(name);
}

// ../ts-builders/src/helpers.ts
function omit(type, keyType2) {
  return namedType("Omit").addGenericArgument(type).addGenericArgument(keyType2);
}

// ../ts-builders/src/Import.ts
var NamespaceImport = class {
  constructor(alias, from) {
    this.alias = alias;
    this.from = from;
  }
  type = false;
  typeOnly() {
    this.type = true;
    return this;
  }
  write(writer) {
    writer.write("import ");
    if (this.type) {
      writer.write("type ");
    }
    writer.write("* as ").write(this.alias).write(` from "${this.from}"`);
  }
};
var BindingsImport = class {
  constructor(from) {
    this.from = from;
  }
  defaultImport;
  namedImports = [];
  default(name) {
    this.defaultImport = name;
    return this;
  }
  named(namedImport) {
    if (typeof namedImport === "string") {
      namedImport = new NamedImport(namedImport);
    }
    this.namedImports.push(namedImport);
    return this;
  }
  write(writer) {
    writer.write("import ");
    if (this.defaultImport) {
      writer.write(this.defaultImport);
      if (this.hasNamedImports()) {
        writer.write(", ");
      }
    }
    if (this.hasNamedImports()) {
      writer.write("{ ").writeJoined(", ", this.namedImports).write(" }");
    }
    writer.write(` from "${this.from}"`);
  }
  hasNamedImports() {
    return this.namedImports.length > 0;
  }
};
var NamedImport = class {
  constructor(name) {
    this.name = name;
  }
  alias;
  type = false;
  as(alias) {
    this.alias = alias;
    return this;
  }
  typeOnly() {
    this.type = true;
    return this;
  }
  write(writer) {
    if (this.type) {
      writer.write("type ");
    }
    writer.write(this.name);
    if (this.alias) {
      writer.write(" as ").write(this.alias);
    }
  }
};
var ModuleImport = class {
  constructor(from) {
    this.from = from;
  }
  asNamespace(alias) {
    return new NamespaceImport(alias, this.from);
  }
  default(alias) {
    return new BindingsImport(this.from).default(alias);
  }
  named(namedImport) {
    return new BindingsImport(this.from).named(namedImport);
  }
  write(writer) {
    writer.write("import ").write(`"${this.from}"`);
  }
};
function moduleImport(from) {
  return new ModuleImport(from);
}

// ../ts-builders/src/Interface.ts
init_TypeBuilder();
var InterfaceDeclaration = class extends TypeBuilder {
  constructor(name) {
    super();
    this.name = name;
  }
  needsParenthesisWhenIndexed = true;
  items = [];
  genericParameters = [];
  extendedTypes = [];
  add(item2) {
    this.items.push(item2);
    return this;
  }
  addMultiple(items) {
    for (const item2 of items) {
      this.add(item2);
    }
    return this;
  }
  addGenericParameter(param) {
    this.genericParameters.push(param);
    return this;
  }
  extends(type) {
    this.extendedTypes.push(type);
    return this;
  }
  write(writer) {
    writer.write("interface ").write(this.name);
    if (this.genericParameters.length > 0) {
      writer.write("<").writeJoined(", ", this.genericParameters).write(">");
    }
    if (this.extendedTypes.length > 0) {
      writer.write(" extends ").writeJoined(", ", this.extendedTypes);
    }
    if (this.items.length === 0) {
      writer.writeLine(" {}");
      return;
    }
    writer.writeLine(" {").withIndent(() => {
      for (const item2 of this.items) {
        writer.writeLine(item2);
      }
    }).write("}");
  }
};
function interfaceDeclaration(name) {
  return new InterfaceDeclaration(name);
}

// ../ts-builders/src/IntersectionType.ts
init_TypeBuilder();
var IntersectionType = class extends TypeBuilder {
  needsParenthesisWhenIndexed = true;
  needsParenthesisInKeyof = true;
  members;
  constructor(firstType) {
    super();
    this.members = [firstType];
  }
  addType(type) {
    this.members.push(type);
    return this;
  }
  addTypes(types) {
    for (const type of types) {
      this.addType(type);
    }
    return this;
  }
  write(writer) {
    writer.writeJoined(" & ", this.members, (member, writer2) => {
      if (member.needsParenthesisInIntersection) {
        writer2.write("(").write(member).write(")");
      } else {
        writer2.write(member);
      }
    });
  }
  mapTypes(callback) {
    return intersectionType(this.members.map((m) => callback(m)));
  }
};
function intersectionType(types) {
  if (Array.isArray(types)) {
    if (types.length === 0) {
      throw new TypeError("Intersection types array can not be empty");
    }
    const intersection = new IntersectionType(types[0]);
    for (let i = 1; i < types.length; i++) {
      intersection.addType(types[i]);
    }
    return intersection;
  }
  return new IntersectionType(types);
}

// ../ts-builders/src/index.ts
init_KeyType();

// ../ts-builders/src/Method.ts
var Method = class {
  constructor(name) {
    this.name = name;
  }
  docComment;
  returnType = voidType;
  parameters = [];
  genericParameters = [];
  setDocComment(docComment2) {
    this.docComment = docComment2;
    return this;
  }
  setReturnType(returnType) {
    this.returnType = returnType;
    return this;
  }
  addParameter(param) {
    this.parameters.push(param);
    return this;
  }
  addGenericParameter(param) {
    this.genericParameters.push(param);
    return this;
  }
  write(writer) {
    if (this.docComment) {
      writer.write(this.docComment);
    }
    writer.write(this.name);
    if (this.genericParameters.length > 0) {
      writer.write("<").writeJoined(", ", this.genericParameters).write(">");
    }
    writer.write("(");
    if (this.parameters.length > 0) {
      writer.writeJoined(", ", this.parameters);
    }
    writer.write(")");
    if (this.name !== "constructor") {
      writer.write(": ").write(this.returnType);
    }
  }
};
function method(name) {
  return new Method(name);
}

// ../ts-builders/src/NamespaceDeclaration.ts
var NamespaceDeclaration = class {
  constructor(name) {
    this.name = name;
  }
  items = [];
  add(declaration) {
    this.items.push(declaration);
  }
  write(writer) {
    writer.writeLine(`namespace ${this.name} {`).withIndent(() => {
      for (const item2 of this.items) {
        writer.writeLine(item2);
      }
    }).write("}");
  }
};
function namespace(name) {
  return new NamespaceDeclaration(name);
}

// ../ts-builders/src/ObjectType.ts
init_TypeBuilder();
var ObjectType = class extends TypeBuilder {
  needsParenthesisWhenIndexed = true;
  items = [];
  inline = false;
  add(item2) {
    this.items.push(item2);
    return this;
  }
  addMultiple(items) {
    for (const item2 of items) {
      this.add(item2);
    }
    return this;
  }
  formatInline() {
    this.inline = true;
    return this;
  }
  write(writer) {
    if (this.items.length === 0) {
      writer.write("{}");
    } else if (this.inline) {
      this.writeInline(writer);
    } else {
      this.writeMultiline(writer);
    }
  }
  writeMultiline(writer) {
    writer.writeLine("{").withIndent(() => {
      for (const item2 of this.items) {
        writer.writeLine(item2);
      }
    }).write("}");
  }
  writeInline(writer) {
    writer.write("{ ").writeJoined(", ", this.items).write(" }");
  }
};
function objectType() {
  return new ObjectType();
}

// ../ts-builders/src/Parameter.ts
var Parameter = class {
  constructor(name, type) {
    this.name = name;
    this.type = type;
  }
  isOptional = false;
  optional() {
    this.isOptional = true;
    return this;
  }
  write(writer) {
    writer.write(this.name);
    if (this.isOptional) {
      writer.write("?");
    }
    writer.write(": ").write(this.type);
  }
};
function parameter(name, type) {
  return new Parameter(name, type);
}

// ../ts-builders/src/Property.ts
var Property = class {
  constructor(name, type) {
    this.name = name;
    this.type = type;
  }
  isOptional = false;
  isReadonly = false;
  docComment;
  optional() {
    this.isOptional = true;
    return this;
  }
  readonly() {
    this.isReadonly = true;
    return this;
  }
  setDocComment(docComment2) {
    this.docComment = docComment2;
    return this;
  }
  write(writer) {
    if (this.docComment) {
      writer.write(this.docComment);
    }
    if (this.isReadonly) {
      writer.write("readonly ");
    }
    if (typeof this.name === "string") {
      if (isValidJsIdentifier(this.name)) {
        writer.write(this.name);
      } else {
        writer.write("[").write(JSON.stringify(this.name)).write("]");
      }
    } else {
      writer.write("[").write(this.name).write("]");
    }
    if (this.isOptional) {
      writer.write("?");
    }
    writer.write(": ").write(this.type);
  }
};
function property(name, type) {
  return new Property(name, type);
}

// ../ts-builders/src/Writer.ts
var INDENT_SIZE = 2;
var Writer = class {
  constructor(startingIndent = 0, context) {
    this.context = context;
    this.currentIndent = startingIndent;
  }
  lines = [];
  currentLine = "";
  currentIndent = 0;
  marginSymbol;
  afterNextNewLineCallback;
  /**
   * Adds provided value to the current line. Does not end the line.
   *
   * @param value
   * @returns
   */
  write(value) {
    if (typeof value === "string") {
      this.currentLine += value;
    } else {
      value.write(this);
    }
    return this;
  }
  /**
   * Adds several `values` to the current line, separated by `separator`. Both values and separator
   * can also be `Builder` instances for more advanced formatting.
   *
   * @param separator
   * @param values
   * @param writeItem allow to customize how individual item is written
   * @returns
   */
  writeJoined(separator, values, writeItem = (item2, w) => w.write(item2)) {
    const last = values.length - 1;
    for (let i = 0; i < values.length; i++) {
      writeItem(values[i], this);
      if (i !== last) {
        this.write(separator);
      }
    }
    return this;
  }
  /**
   * Adds a string to current line, flushes current line and starts a new line.
   * @param line
   * @returns
   */
  writeLine(line) {
    return this.write(line).newLine();
  }
  /**
   * Flushes current line and starts a new line. New line starts at previously configured indentation level
   * @returns
   */
  newLine() {
    this.lines.push(this.indentedCurrentLine());
    this.currentLine = "";
    this.marginSymbol = void 0;
    const afterNextNewLineCallback = this.afterNextNewLineCallback;
    this.afterNextNewLineCallback = void 0;
    afterNextNewLineCallback?.();
    return this;
  }
  /**
   * Increases indentation level by 1, calls provided callback and then decreases indentation again.
   * Could be used for writing indented blocks of text:
   *
   * @example
   * ```ts
   * writer
   *   .writeLine('{')
   *   .withIndent(() => {
   *      writer.writeLine('foo: 123');
   *      writer.writeLine('bar: 456');
   *   })
   *   .writeLine('}')
   * ```
   * @param callback
   * @returns
   */
  withIndent(callback) {
    this.indent();
    callback(this);
    this.unindent();
    return this;
  }
  /**
   * Calls provided callback next time when new line is started.
   * Callback is called after old line have already been flushed and a new
   * line have been started. Can be used for adding "between the lines" decorations,
   * such as underlines.
   *
   * @param callback
   * @returns
   */
  afterNextNewline(callback) {
    this.afterNextNewLineCallback = callback;
    return this;
  }
  /**
   * Increases indentation level of the current line by 1
   * @returns
   */
  indent() {
    this.currentIndent++;
    return this;
  }
  /**
   * Decreases indentation level of the current line by 1, if it is possible
   * @returns
   */
  unindent() {
    if (this.currentIndent > 0) {
      this.currentIndent--;
    }
    return this;
  }
  /**
   * Adds a symbol, that will replace the first character of the current line (including indentation)
   * when it is flushed. Can be used for adding markers to the line.
   *
   * Note: if indentation level of the line is 0, it will replace the first actually printed character
   * of the line. Use with caution.
   * @param symbol
   * @returns
   */
  addMarginSymbol(symbol2) {
    this.marginSymbol = symbol2;
    return this;
  }
  toString() {
    return this.lines.concat(this.indentedCurrentLine()).join("\n");
  }
  getCurrentLineLength() {
    return this.currentLine.length;
  }
  indentedCurrentLine() {
    const line = this.currentLine.padStart(this.currentLine.length + INDENT_SIZE * this.currentIndent);
    if (this.marginSymbol) {
      return this.marginSymbol + line.slice(1);
    }
    return line;
  }
};

// ../ts-builders/src/stringify.ts
function stringify(builder, { indentLevel = 0, newLine = "none" } = {}) {
  const str = new Writer(indentLevel, void 0).write(builder).toString();
  switch (newLine) {
    case "none":
      return str;
    case "leading":
      return "\n" + str;
    case "trailing":
      return str + "\n";
    case "both":
      return "\n" + str + "\n";
    default:
      assertNever(newLine, "Unexpected value");
  }
}

// ../ts-builders/src/StringLiteralType.ts
init_TypeBuilder();
var StringLiteralType = class extends TypeBuilder {
  constructor(content) {
    super();
    this.content = content;
  }
  write(writer) {
    writer.write(JSON.stringify(this.content));
  }
  asValue() {
    return new StringLiteralValue(this);
  }
};
var StringLiteralValue = class extends ValueBuilder {
  #type;
  constructor(type) {
    super();
    this.#type = type;
  }
  write(writer) {
    writer.write(this.#type);
  }
};
function stringLiteral(content) {
  return new StringLiteralType(content);
}

// ../ts-builders/src/TupleType.ts
init_TypeBuilder();
var TupleItem = class {
  constructor(type) {
    this.type = type;
  }
  name;
  setName(name) {
    this.name = name;
    return this;
  }
  write(writer) {
    if (this.name) {
      writer.write(this.name).write(": ");
    }
    writer.write(this.type);
  }
};
var TupleType = class extends TypeBuilder {
  items = [];
  add(item2) {
    if (item2 instanceof TypeBuilder) {
      item2 = new TupleItem(item2);
    }
    this.items.push(item2);
    return this;
  }
  write(writer) {
    writer.write("[").writeJoined(", ", this.items).write("]");
  }
};
function tupleType() {
  return new TupleType();
}
function tupleItem(type) {
  return new TupleItem(type);
}

// ../ts-builders/src/index.ts
init_TypeBuilder();

// ../ts-builders/src/TypeDeclaration.ts
var TypeDeclaration = class {
  constructor(name, type) {
    this.name = name;
    this.type = type;
  }
  genericParameters = [];
  docComment;
  addGenericParameter(param) {
    this.genericParameters.push(param);
    return this;
  }
  setName(name) {
    this.name = name;
    return this;
  }
  setDocComment(docComment2) {
    this.docComment = docComment2;
    return this;
  }
  write(writer) {
    if (this.docComment) {
      writer.write(this.docComment);
    }
    writer.write("type ").write(this.name);
    if (this.genericParameters.length > 0) {
      writer.write("<").writeJoined(", ", this.genericParameters).write(">");
    }
    writer.write(" = ").write(this.type);
  }
};
function typeDeclaration(name, type) {
  return new TypeDeclaration(name, type);
}

// ../ts-builders/src/UnionType.ts
init_TypeBuilder();
var UnionType = class extends TypeBuilder {
  needsParenthesisWhenIndexed = true;
  needsParenthesisInKeyof = true;
  needsParenthesisInIntersection = true;
  variants;
  constructor(firstType) {
    super();
    this.variants = [firstType];
  }
  addVariant(variant) {
    this.variants.push(variant);
    return this;
  }
  addVariants(variants) {
    for (const variant of variants) {
      this.addVariant(variant);
    }
    return this;
  }
  write(writer) {
    writer.writeJoined(" | ", this.variants, (variant, writer2) => {
      if (variant.needsParenthesisInUnion) {
        writer2.write("(").write(variant).write(")");
      } else {
        writer2.write(variant);
      }
    });
  }
  mapVariants(callback) {
    return unionType(this.variants.map((v) => callback(v)));
  }
};
function unionType(types) {
  if (Array.isArray(types)) {
    if (types.length === 0) {
      throw new TypeError("Union types array can not be empty");
    }
    const union = new UnionType(types[0]);
    for (let i = 1; i < types.length; i++) {
      union.addVariant(types[i]);
    }
    return union;
  }
  return new UnionType(types);
}

// ../ts-builders/src/WellKnownSymbol.ts
var WellKnownSymbol = class {
  constructor(name) {
    this.name = name;
  }
  write(writer) {
    writer.write("Symbol.").write(this.name);
  }
};
function wellKnownSymbol(name) {
  return new WellKnownSymbol(name);
}
var toStringTag = wellKnownSymbol("toStringTag");

// ../client-generator-js/src/TSClient/Input.ts
var import_indent_string2 = __toESM(require_indent_string());

// ../dmmf/src/convert.ts
function datamodelEnumToSchemaEnum(datamodelEnum) {
  return {
    name: datamodelEnum.name,
    values: datamodelEnum.values.map((v) => v.name)
  };
}

// ../dmmf/src/dmmf.ts
var ModelAction = /* @__PURE__ */ ((ModelAction2) => {
  ModelAction2["findUnique"] = "findUnique";
  ModelAction2["findUniqueOrThrow"] = "findUniqueOrThrow";
  ModelAction2["findFirst"] = "findFirst";
  ModelAction2["findFirstOrThrow"] = "findFirstOrThrow";
  ModelAction2["findMany"] = "findMany";
  ModelAction2["create"] = "create";
  ModelAction2["createMany"] = "createMany";
  ModelAction2["createManyAndReturn"] = "createManyAndReturn";
  ModelAction2["update"] = "update";
  ModelAction2["updateMany"] = "updateMany";
  ModelAction2["updateManyAndReturn"] = "updateManyAndReturn";
  ModelAction2["upsert"] = "upsert";
  ModelAction2["delete"] = "delete";
  ModelAction2["deleteMany"] = "deleteMany";
  ModelAction2["groupBy"] = "groupBy";
  ModelAction2["count"] = "count";
  ModelAction2["aggregate"] = "aggregate";
  ModelAction2["findRaw"] = "findRaw";
  ModelAction2["aggregateRaw"] = "aggregateRaw";
  return ModelAction2;
})(ModelAction || {});

// ../client-generator-js/src/utils.ts
function getSelectName(modelName) {
  return `${modelName}Select`;
}
function getSelectCreateManyAndReturnName(modelName) {
  return `${modelName}SelectCreateManyAndReturn`;
}
function getSelectUpdateManyAndReturnName(modelName) {
  return `${modelName}SelectUpdateManyAndReturn`;
}
function getIncludeName(modelName) {
  return `${modelName}Include`;
}
function getIncludeCreateManyAndReturnName(modelName) {
  return `${modelName}IncludeCreateManyAndReturn`;
}
function getIncludeUpdateManyAndReturnName(modelName) {
  return `${modelName}IncludeUpdateManyAndReturn`;
}
function getCreateManyAndReturnOutputType(modelName) {
  return `CreateMany${modelName}AndReturnOutputType`;
}
function getUpdateManyAndReturnOutputType(modelName) {
  return `UpdateMany${modelName}AndReturnOutputType`;
}
function getOmitName(modelName) {
  return `${modelName}Omit`;
}
function getAggregateName(modelName) {
  return `Aggregate${capitalize(modelName)}`;
}
function getGroupByName(modelName) {
  return `${capitalize(modelName)}GroupByOutputType`;
}
function getAvgAggregateName(modelName) {
  return `${capitalize(modelName)}AvgAggregateOutputType`;
}
function getSumAggregateName(modelName) {
  return `${capitalize(modelName)}SumAggregateOutputType`;
}
function getMinAggregateName(modelName) {
  return `${capitalize(modelName)}MinAggregateOutputType`;
}
function getMaxAggregateName(modelName) {
  return `${capitalize(modelName)}MaxAggregateOutputType`;
}
function getCountAggregateInputName(modelName) {
  return `${capitalize(modelName)}CountAggregateInputType`;
}
function getCountAggregateOutputName(modelName) {
  return `${capitalize(modelName)}CountAggregateOutputType`;
}
function getAggregateInputType(aggregateOutputType) {
  return aggregateOutputType.replace(/OutputType$/, "InputType");
}
function getGroupByArgsName(modelName) {
  return `${modelName}GroupByArgs`;
}
function getGroupByPayloadName(modelName) {
  return `Get${capitalize(modelName)}GroupByPayload`;
}
function getAggregateArgsName(modelName) {
  return `${capitalize(modelName)}AggregateArgs`;
}
function getAggregateGetName(modelName) {
  return `Get${capitalize(modelName)}AggregateType`;
}
function getFieldArgName(field, modelName) {
  if (field.args.length) {
    return getModelFieldArgsName(field, modelName);
  }
  return getModelArgName(field.outputType.type);
}
function getModelFieldArgsName(field, modelName) {
  return `${modelName}$${field.name}Args`;
}
function getModelArgName(modelName, action2) {
  if (!action2) {
    return `${modelName}DefaultArgs`;
  }
  switch (action2) {
    case "findMany" /* findMany */:
      return `${modelName}FindManyArgs`;
    case "findUnique" /* findUnique */:
      return `${modelName}FindUniqueArgs`;
    case "findUniqueOrThrow" /* findUniqueOrThrow */:
      return `${modelName}FindUniqueOrThrowArgs`;
    case "findFirst" /* findFirst */:
      return `${modelName}FindFirstArgs`;
    case "findFirstOrThrow" /* findFirstOrThrow */:
      return `${modelName}FindFirstOrThrowArgs`;
    case "upsert" /* upsert */:
      return `${modelName}UpsertArgs`;
    case "update" /* update */:
      return `${modelName}UpdateArgs`;
    case "updateMany" /* updateMany */:
      return `${modelName}UpdateManyArgs`;
    case "updateManyAndReturn" /* updateManyAndReturn */:
      return `${modelName}UpdateManyAndReturnArgs`;
    case "delete" /* delete */:
      return `${modelName}DeleteArgs`;
    case "create" /* create */:
      return `${modelName}CreateArgs`;
    case "createMany" /* createMany */:
      return `${modelName}CreateManyArgs`;
    case "createManyAndReturn" /* createManyAndReturn */:
      return `${modelName}CreateManyAndReturnArgs`;
    case "deleteMany" /* deleteMany */:
      return `${modelName}DeleteManyArgs`;
    case "groupBy" /* groupBy */:
      return getGroupByArgsName(modelName);
    case "aggregate" /* aggregate */:
      return getAggregateArgsName(modelName);
    case "count" /* count */:
      return `${modelName}CountArgs`;
    case "findRaw" /* findRaw */:
      return `${modelName}FindRawArgs`;
    case "aggregateRaw" /* aggregateRaw */:
      return `${modelName}AggregateRawArgs`;
    default:
      assertNever(action2, `Unknown action: ${action2}`);
  }
}
function getPayloadName(modelName, namespace2 = true) {
  if (namespace2) {
    return `Prisma.${getPayloadName(modelName, false)}`;
  }
  return `$${modelName}Payload`;
}
function getFieldRefsTypeName(name) {
  return `${name}FieldRefs`;
}
function getRefAllowedTypeName(type) {
  let typeName = type.type;
  if (type.isList) {
    typeName += "[]";
  }
  return `'${typeName}'`;
}
function appendSkipType(context, type) {
  if (context.isPreviewFeatureOn("strictUndefinedChecks")) {
    return unionType([type, namedType("$Types.Skip")]);
  }
  return type;
}
var extArgsParam = genericParameter("ExtArgs").extends(namedType("$Extensions.InternalArgs")).default(namedType("$Extensions.DefaultArgs"));

// ../client-generator-js/src/utils/common.ts
function needsNamespace(field) {
  if (field.kind === "object") {
    return true;
  }
  if (field.kind === "scalar") {
    return field.type === "Json" || field.type === "Decimal" || field.type === "Bytes";
  }
  return false;
}
var GraphQLScalarToJSTypeTable = {
  String: "string",
  Int: "number",
  Float: "number",
  Boolean: "boolean",
  Long: "number",
  DateTime: ["Date", "string"],
  ID: "string",
  UUID: "string",
  Json: "JsonValue",
  Bytes: "Bytes",
  Decimal: ["Decimal", "DecimalJsLike", "number", "string"],
  BigInt: ["bigint", "number"]
};
var JSOutputTypeToInputType = {
  JsonValue: "InputJsonValue"
};

// ../client-generator-js/src/TSClient/Input.ts
var InputField = class {
  constructor(field, context, source) {
    this.field = field;
    this.context = context;
    this.source = source;
  }
  toTS() {
    const property2 = buildInputField(this.field, this.context, this.source);
    return stringify(property2);
  }
};
function buildInputField(field, context, source) {
  const tsType = buildAllFieldTypes(field.inputTypes, context, source);
  const tsProperty = property(field.name, field.isRequired ? tsType : appendSkipType(context, tsType));
  if (!field.isRequired) {
    tsProperty.optional();
  }
  const docComment2 = docComment();
  if (field.comment) {
    docComment2.addText(field.comment);
  }
  if (field.deprecation) {
    docComment2.addText(`@deprecated since ${field.deprecation.sinceVersion}: ${field.deprecation.reason}`);
  }
  if (docComment2.lines.length > 0) {
    tsProperty.setDocComment(docComment2);
  }
  return tsProperty;
}
function buildSingleFieldType(t, genericsInfo, source) {
  let type;
  const scalarType = GraphQLScalarToJSTypeTable[t.type];
  if (t.location === "enumTypes" && t.namespace === "model") {
    type = namedType(`$Enums.${t.type}`);
  } else if (t.type === "Null") {
    return nullType;
  } else if (Array.isArray(scalarType)) {
    const union = unionType(scalarType.map(namedInputType));
    if (t.isList) {
      return union.mapVariants((variant) => array(variant));
    }
    return union;
  } else {
    type = namedInputType(scalarType ?? t.type);
  }
  if (genericsInfo.typeRefNeedsGenericModelArg(t)) {
    if (source) {
      type.addGenericArgument(stringLiteral(source));
    } else {
      type.addGenericArgument(namedType("$PrismaModel"));
    }
  }
  if (t.isList) {
    return array(type);
  }
  return type;
}
function namedInputType(typeName) {
  return namedType(JSOutputTypeToInputType[typeName] ?? typeName);
}
function buildAllFieldTypes(inputTypes, context, source) {
  const inputObjectTypes = inputTypes.filter((t) => t.location === "inputObjectTypes" && !t.isList);
  const otherTypes = inputTypes.filter((t) => t.location !== "inputObjectTypes" || t.isList);
  const tsInputObjectTypes = inputObjectTypes.map((type) => buildSingleFieldType(type, context.genericArgsInfo, source));
  const tsOtherTypes = otherTypes.map((type) => buildSingleFieldType(type, context.genericArgsInfo, source));
  if (tsOtherTypes.length === 0) {
    return xorTypes(tsInputObjectTypes);
  }
  if (tsInputObjectTypes.length === 0) {
    return unionType(tsOtherTypes);
  }
  return unionType(xorTypes(tsInputObjectTypes)).addVariants(tsOtherTypes);
}
function xorTypes(types) {
  return types.reduce((prev, curr) => namedType("XOR").addGenericArgument(prev).addGenericArgument(curr));
}
var InputType = class {
  constructor(type, context) {
    this.type = type;
    this.context = context;
    this.generatedName = type.name;
  }
  generatedName;
  toTS() {
    const { type } = this;
    const source = type.meta?.source;
    const fields = uniqueBy(type.fields, (f) => f.name);
    const body = `{
${(0, import_indent_string2.default)(
      fields.map((arg) => {
        return new InputField(arg, this.context, source).toTS();
      }).join("\n"),
      TAB_SIZE
    )}
}`;
    return `
export type ${this.getTypeName()} = ${wrapWithAtLeast(body, type)}`;
  }
  overrideName(name) {
    this.generatedName = name;
    return this;
  }
  getTypeName() {
    if (this.context.genericArgsInfo.typeNeedsGenericModelArg(this.type)) {
      return `${this.generatedName}<$PrismaModel = never>`;
    }
    return this.generatedName;
  }
};
function wrapWithAtLeast(body, input) {
  if (input.constraints?.fields && input.constraints.fields.length > 0) {
    const fields = input.constraints.fields.map((f) => `"${f}"`).join(" | ");
    return `Prisma.AtLeast<${body}, ${fields}>`;
  }
  return body;
}

// ../client-generator-js/src/TSClient/Model.ts
var import_indent_string3 = __toESM(require_indent_string());

// ../../node_modules/.pnpm/klona@2.0.6/node_modules/klona/dist/index.mjs
function klona(x) {
  if (typeof x !== "object") return x;
  var k, tmp, str = Object.prototype.toString.call(x);
  if (str === "[object Object]") {
    if (x.constructor !== Object && typeof x.constructor === "function") {
      tmp = new x.constructor();
      for (k in x) {
        if (x.hasOwnProperty(k) && tmp[k] !== x[k]) {
          tmp[k] = klona(x[k]);
        }
      }
    } else {
      tmp = {};
      for (k in x) {
        if (k === "__proto__") {
          Object.defineProperty(tmp, k, {
            value: klona(x[k]),
            configurable: true,
            enumerable: true,
            writable: true
          });
        } else {
          tmp[k] = klona(x[k]);
        }
      }
    }
    return tmp;
  }
  if (str === "[object Array]") {
    k = x.length;
    for (tmp = Array(k); k--; ) {
      tmp[k] = klona(x[k]);
    }
    return tmp;
  }
  if (str === "[object Set]") {
    tmp = /* @__PURE__ */ new Set();
    x.forEach(function(val) {
      tmp.add(klona(val));
    });
    return tmp;
  }
  if (str === "[object Map]") {
    tmp = /* @__PURE__ */ new Map();
    x.forEach(function(val, key) {
      tmp.set(klona(key), klona(val));
    });
    return tmp;
  }
  if (str === "[object Date]") {
    return /* @__PURE__ */ new Date(+x);
  }
  if (str === "[object RegExp]") {
    tmp = new RegExp(x.source, x.flags);
    tmp.lastIndex = x.lastIndex;
    return tmp;
  }
  if (str === "[object DataView]") {
    return new x.constructor(klona(x.buffer));
  }
  if (str === "[object ArrayBuffer]") {
    return x.slice(0);
  }
  if (str.slice(-6) === "Array]") {
    return new x.constructor(x);
  }
  return x;
}

// ../client-generator-js/src/TSClient/helpers.ts
var import_pluralize2 = __toESM(require_pluralize());

// ../client-generator-js/src/TSClient/jsdoc.ts
var Docs = {
  cursor: `{@link https://www.prisma.io/docs/concepts/components/prisma-client/pagination#cursor-based-pagination Cursor Docs}`,
  pagination: `{@link https://www.prisma.io/docs/concepts/components/prisma-client/pagination Pagination Docs}`,
  aggregations: `{@link https://www.prisma.io/docs/concepts/components/prisma-client/aggregations Aggregation Docs}`,
  distinct: `{@link https://www.prisma.io/docs/concepts/components/prisma-client/distinct Distinct Docs}`,
  sorting: `{@link https://www.prisma.io/docs/concepts/components/prisma-client/sorting Sorting Docs}`
};
function addLinkToDocs(comment, docs) {
  return `${Docs[docs]}

${comment}`;
}
function getDeprecationString(since, replacement) {
  return `@deprecated since ${since} please use \`${replacement}\``;
}
var undefinedNote = `Note, that providing \`undefined\` is treated as the value not being there.
Read more here: https://pris.ly/d/null-undefined`;
var JSDocFields = {
  take: (singular, plural) => addLinkToDocs(`Take \`\xB1n\` ${plural} from the position of the cursor.`, "pagination"),
  skip: (singular, plural) => addLinkToDocs(`Skip the first \`n\` ${plural}.`, "pagination"),
  _count: (singular, plural) => addLinkToDocs(`Count returned ${plural}`, "aggregations"),
  _avg: () => addLinkToDocs(`Select which fields to average`, "aggregations"),
  _sum: () => addLinkToDocs(`Select which fields to sum`, "aggregations"),
  _min: () => addLinkToDocs(`Select which fields to find the minimum value`, "aggregations"),
  _max: () => addLinkToDocs(`Select which fields to find the maximum value`, "aggregations"),
  count: () => getDeprecationString("2.23.0", "_count"),
  avg: () => getDeprecationString("2.23.0", "_avg"),
  sum: () => getDeprecationString("2.23.0", "_sum"),
  min: () => getDeprecationString("2.23.0", "_min"),
  max: () => getDeprecationString("2.23.0", "_max"),
  distinct: (singular, plural) => addLinkToDocs(`Filter by unique combinations of ${plural}.`, "distinct"),
  orderBy: (singular, plural) => addLinkToDocs(`Determine the order of ${plural} to fetch.`, "sorting")
};
var JSDocs = {
  groupBy: {
    body: (ctx) => `Group by ${ctx.singular}.
${undefinedNote}
@param {${getGroupByArgsName(ctx.model.name)}} args - Group by arguments.
@example
// Group by city, order by createdAt, get count
const result = await prisma.user.groupBy({
  by: ['city', 'createdAt'],
  orderBy: {
    createdAt: true
  },
  _count: {
    _all: true
  },
})
`,
    fields: {}
  },
  create: {
    body: (ctx) => `Create a ${ctx.singular}.
@param {${getModelArgName(ctx.model.name, ctx.action)}} args - Arguments to create a ${ctx.singular}.
@example
// Create one ${ctx.singular}
const ${ctx.singular} = await ${ctx.method}({
  data: {
    // ... data to create a ${ctx.singular}
  }
})
`,
    fields: {
      data: (singular) => `The data needed to create a ${singular}.`
    }
  },
  createMany: {
    body: (ctx) => `Create many ${ctx.plural}.
@param {${getModelArgName(ctx.model.name, ctx.action)}} args - Arguments to create many ${ctx.plural}.
@example
// Create many ${ctx.plural}
const ${uncapitalize(ctx.mapping.model)} = await ${ctx.method}({
  data: [
    // ... provide data here
  ]
})
    `,
    fields: {
      data: (singular, plural) => `The data used to create many ${plural}.`
    }
  },
  createManyAndReturn: {
    body: (ctx) => {
      const onlySelect = ctx.firstScalar ? `
// Create many ${ctx.plural} and only return the \`${ctx.firstScalar.name}\`
const ${uncapitalize(ctx.mapping.model)}With${capitalize(ctx.firstScalar.name)}Only = await ${ctx.method}({
  select: { ${ctx.firstScalar.name}: true },
  data: [
    // ... provide data here
  ]
})` : "";
      return `Create many ${ctx.plural} and returns the data saved in the database.
@param {${getModelArgName(ctx.model.name, ctx.action)}} args - Arguments to create many ${ctx.plural}.
@example
// Create many ${ctx.plural}
const ${uncapitalize(ctx.mapping.model)} = await ${ctx.method}({
  data: [
    // ... provide data here
  ]
})
${onlySelect}
${undefinedNote}
`;
    },
    fields: {
      data: (singular, plural) => `The data used to create many ${plural}.`
    }
  },
  findUnique: {
    body: (ctx) => `Find zero or one ${ctx.singular} that matches the filter.
@param {${getModelArgName(ctx.model.name, ctx.action)}} args - Arguments to find a ${ctx.singular}
@example
// Get one ${ctx.singular}
const ${uncapitalize(ctx.mapping.model)} = await ${ctx.method}({
  where: {
    // ... provide filter here
  }
})`,
    fields: {
      where: (singular) => `Filter, which ${singular} to fetch.`
    }
  },
  findUniqueOrThrow: {
    body: (ctx) => `Find one ${ctx.singular} that matches the filter or throw an error with \`error.code='P2025'\`
if no matches were found.
@param {${getModelArgName(ctx.model.name, ctx.action)}} args - Arguments to find a ${ctx.singular}
@example
// Get one ${ctx.singular}
const ${uncapitalize(ctx.mapping.model)} = await ${ctx.method}({
  where: {
    // ... provide filter here
  }
})`,
    fields: {
      where: (singular) => `Filter, which ${singular} to fetch.`
    }
  },
  findFirst: {
    body: (ctx) => `Find the first ${ctx.singular} that matches the filter.
${undefinedNote}
@param {${getModelArgName(ctx.model.name, ctx.action)}} args - Arguments to find a ${ctx.singular}
@example
// Get one ${ctx.singular}
const ${uncapitalize(ctx.mapping.model)} = await ${ctx.method}({
  where: {
    // ... provide filter here
  }
})`,
    fields: {
      where: (singular) => `Filter, which ${singular} to fetch.`,
      orderBy: JSDocFields.orderBy,
      cursor: (singular, plural) => addLinkToDocs(`Sets the position for searching for ${plural}.`, "cursor"),
      take: JSDocFields.take,
      skip: JSDocFields.skip,
      distinct: JSDocFields.distinct
    }
  },
  findFirstOrThrow: {
    body: (ctx) => `Find the first ${ctx.singular} that matches the filter or
throw \`PrismaKnownClientError\` with \`P2025\` code if no matches were found.
${undefinedNote}
@param {${getModelArgName(ctx.model.name, ctx.action)}} args - Arguments to find a ${ctx.singular}
@example
// Get one ${ctx.singular}
const ${uncapitalize(ctx.mapping.model)} = await ${ctx.method}({
  where: {
    // ... provide filter here
  }
})`,
    fields: {
      where: (singular) => `Filter, which ${singular} to fetch.`,
      orderBy: JSDocFields.orderBy,
      cursor: (singular, plural) => addLinkToDocs(`Sets the position for searching for ${plural}.`, "cursor"),
      take: JSDocFields.take,
      skip: JSDocFields.skip,
      distinct: JSDocFields.distinct
    }
  },
  findMany: {
    body: (ctx) => {
      const onlySelect = ctx.firstScalar ? `
// Only select the \`${ctx.firstScalar.name}\`
const ${uncapitalize(ctx.mapping.model)}With${capitalize(ctx.firstScalar.name)}Only = await ${ctx.method}({ select: { ${ctx.firstScalar.name}: true } })` : "";
      return `Find zero or more ${ctx.plural} that matches the filter.
${undefinedNote}
@param {${getModelArgName(ctx.model.name, ctx.action)}} args - Arguments to filter and select certain fields only.
@example
// Get all ${ctx.plural}
const ${ctx.mapping.plural} = await ${ctx.method}()

// Get first 10 ${ctx.plural}
const ${ctx.mapping.plural} = await ${ctx.method}({ take: 10 })
${onlySelect}
`;
    },
    fields: {
      where: (singular, plural) => `Filter, which ${plural} to fetch.`,
      orderBy: JSDocFields.orderBy,
      skip: JSDocFields.skip,
      cursor: (singular, plural) => addLinkToDocs(`Sets the position for listing ${plural}.`, "cursor"),
      take: JSDocFields.take
    }
  },
  update: {
    body: (ctx) => `Update one ${ctx.singular}.
@param {${getModelArgName(ctx.model.name, ctx.action)}} args - Arguments to update one ${ctx.singular}.
@example
// Update one ${ctx.singular}
const ${uncapitalize(ctx.mapping.model)} = await ${ctx.method}({
  where: {
    // ... provide filter here
  },
  data: {
    // ... provide data here
  }
})
`,
    fields: {
      data: (singular) => `The data needed to update a ${singular}.`,
      where: (singular) => `Choose, which ${singular} to update.`
    }
  },
  upsert: {
    body: (ctx) => `Create or update one ${ctx.singular}.
@param {${getModelArgName(ctx.model.name, ctx.action)}} args - Arguments to update or create a ${ctx.singular}.
@example
// Update or create a ${ctx.singular}
const ${uncapitalize(ctx.mapping.model)} = await ${ctx.method}({
  create: {
    // ... data to create a ${ctx.singular}
  },
  update: {
    // ... in case it already exists, update
  },
  where: {
    // ... the filter for the ${ctx.singular} we want to update
  }
})`,
    fields: {
      where: (singular) => `The filter to search for the ${singular} to update in case it exists.`,
      create: (singular) => `In case the ${singular} found by the \`where\` argument doesn't exist, create a new ${singular} with this data.`,
      update: (singular) => `In case the ${singular} was found with the provided \`where\` argument, update it with this data.`
    }
  },
  delete: {
    body: (ctx) => `Delete a ${ctx.singular}.
@param {${getModelArgName(ctx.model.name, ctx.action)}} args - Arguments to delete one ${ctx.singular}.
@example
// Delete one ${ctx.singular}
const ${ctx.singular} = await ${ctx.method}({
  where: {
    // ... filter to delete one ${ctx.singular}
  }
})
`,
    fields: {
      where: (singular) => `Filter which ${singular} to delete.`
    }
  },
  aggregate: {
    body: (ctx) => `Allows you to perform aggregations operations on a ${ctx.singular}.
${undefinedNote}
@param {${getModelArgName(
      ctx.model.name,
      ctx.action
    )}} args - Select which aggregations you would like to apply and on what fields.
@example
// Ordered by age ascending
// Where email contains prisma.io
// Limited to the 10 users
const aggregations = await prisma.user.aggregate({
  _avg: {
    age: true,
  },
  where: {
    email: {
      contains: "prisma.io",
    },
  },
  orderBy: {
    age: "asc",
  },
  take: 10,
})`,
    fields: {
      where: (singular) => `Filter which ${singular} to aggregate.`,
      orderBy: JSDocFields.orderBy,
      cursor: () => addLinkToDocs(`Sets the start position`, "cursor"),
      take: JSDocFields.take,
      skip: JSDocFields.skip,
      _count: JSDocFields._count,
      _avg: JSDocFields._avg,
      _sum: JSDocFields._sum,
      _min: JSDocFields._min,
      _max: JSDocFields._max,
      count: JSDocFields.count,
      avg: JSDocFields.avg,
      sum: JSDocFields.sum,
      min: JSDocFields.min,
      max: JSDocFields.max
    }
  },
  count: {
    body: (ctx) => `Count the number of ${ctx.plural}.
${undefinedNote}
@param {${getModelArgName(ctx.model.name, ctx.action)}} args - Arguments to filter ${ctx.plural} to count.
@example
// Count the number of ${ctx.plural}
const count = await ${ctx.method}({
  where: {
    // ... the filter for the ${ctx.plural} we want to count
  }
})`,
    fields: {}
  },
  updateMany: {
    body: (ctx) => `Update zero or more ${ctx.plural}.
${undefinedNote}
@param {${getModelArgName(ctx.model.name, ctx.action)}} args - Arguments to update one or more rows.
@example
// Update many ${ctx.plural}
const ${uncapitalize(ctx.mapping.model)} = await ${ctx.method}({
  where: {
    // ... provide filter here
  },
  data: {
    // ... provide data here
  }
})
`,
    fields: {
      data: (singular, plural) => `The data used to update ${plural}.`,
      where: (singular, plural) => `Filter which ${plural} to update`,
      limit: (singular, plural) => `Limit how many ${plural} to update.`
    }
  },
  updateManyAndReturn: {
    body: (ctx) => {
      const onlySelect = ctx.firstScalar ? `
// Update zero or more ${ctx.plural} and only return the \`${ctx.firstScalar.name}\`
const ${uncapitalize(ctx.mapping.model)}With${capitalize(ctx.firstScalar.name)}Only = await ${ctx.method}({
  select: { ${ctx.firstScalar.name}: true },
  where: {
    // ... provide filter here
  },
  data: [
    // ... provide data here
  ]
})` : "";
      return `Update zero or more ${ctx.plural} and returns the data updated in the database.
@param {${getModelArgName(ctx.model.name, ctx.action)}} args - Arguments to update many ${ctx.plural}.
@example
// Update many ${ctx.plural}
const ${uncapitalize(ctx.mapping.model)} = await ${ctx.method}({
  where: {
    // ... provide filter here
  },
  data: [
    // ... provide data here
  ]
})
${onlySelect}
${undefinedNote}
`;
    },
    fields: {
      data: (singular, plural) => `The data used to update ${plural}.`,
      where: (singular, plural) => `Filter which ${plural} to update`,
      limit: (singular, plural) => `Limit how many ${plural} to update.`
    }
  },
  deleteMany: {
    body: (ctx) => `Delete zero or more ${ctx.plural}.
@param {${getModelArgName(ctx.model.name, ctx.action)}} args - Arguments to filter ${ctx.plural} to delete.
@example
// Delete a few ${ctx.plural}
const { count } = await ${ctx.method}({
  where: {
    // ... provide filter here
  }
})
`,
    fields: {
      where: (singular, plural) => `Filter which ${plural} to delete`,
      limit: (singular, plural) => `Limit how many ${plural} to delete.`
    }
  },
  aggregateRaw: {
    body: (ctx) => `Perform aggregation operations on a ${ctx.singular}.
@param {${getModelArgName(ctx.model.name, ctx.action)}} args - Select which aggregations you would like to apply.
@example
const ${uncapitalize(ctx.mapping.model)} = await ${ctx.method}({
  pipeline: [
    { $match: { status: "registered" } },
    { $group: { _id: "$country", total: { $sum: 1 } } }
  ]
})`,
    fields: {
      pipeline: () => "An array of aggregation stages to process and transform the document stream via the aggregation pipeline. ${@link https://docs.mongodb.com/manual/reference/operator/aggregation-pipeline MongoDB Docs}.",
      options: () => "Additional options to pass to the `aggregate` command ${@link https://docs.mongodb.com/manual/reference/command/aggregate/#command-fields MongoDB Docs}."
    }
  },
  findRaw: {
    body: (ctx) => `Find zero or more ${ctx.plural} that matches the filter.
@param {${getModelArgName(ctx.model.name, ctx.action)}} args - Select which filters you would like to apply.
@example
const ${uncapitalize(ctx.mapping.model)} = await ${ctx.method}({
  filter: { age: { $gt: 25 } }
})`,
    fields: {
      filter: () => "The query predicate filter. If unspecified, then all documents in the collection will match the predicate. ${@link https://docs.mongodb.com/manual/reference/operator/query MongoDB Docs}.",
      options: () => "Additional options to pass to the `find` command ${@link https://docs.mongodb.com/manual/reference/command/find/#command-fields MongoDB Docs}."
    }
  }
};

// ../client-generator-js/src/TSClient/helpers.ts
function getMethodJSDocBody(action2, mapping, model) {
  const ctx = {
    singular: capitalize(mapping.model),
    plural: capitalize(mapping.plural),
    firstScalar: model.fields.find((f) => f.kind === "scalar"),
    method: `prisma.${uncapitalize(mapping.model)}.${action2}`,
    action: action2,
    mapping,
    model
  };
  const jsdoc = JSDocs[action2]?.body(ctx);
  return jsdoc ? jsdoc : "";
}
function getMethodJSDoc(action2, mapping, model) {
  return wrapComment(getMethodJSDocBody(action2, mapping, model));
}
function wrapComment(str) {
  return `/**
${str.split("\n").map((l) => " * " + l).join("\n")}
**/`;
}
function getArgFieldJSDoc(type, action2, field) {
  if (!field || !action2 || !type) return;
  const fieldName = typeof field === "string" ? field : field.name;
  if (JSDocs[action2] && JSDocs[action2]?.fields[fieldName]) {
    const singular = type.name;
    const plural = (0, import_pluralize2.default)(type.name);
    const comment = JSDocs[action2]?.fields[fieldName](singular, plural);
    return comment;
  }
  return void 0;
}
function escapeJson(str) {
  return str.replace(/\\n/g, "\\\\n").replace(/\\r/g, "\\\\r").replace(/\\t/g, "\\\\t");
}

// ../client-generator-js/src/TSClient/Args.ts
var ArgsTypeBuilder = class {
  constructor(type, context, action2) {
    this.type = type;
    this.context = context;
    this.action = action2;
    this.moduleExport = moduleExport(
      typeDeclaration(getModelArgName(type.name, action2), objectType()).addGenericParameter(extArgsParam)
    ).setDocComment(docComment(`${type.name} ${action2 ?? "without action"}`));
  }
  moduleExport;
  hasDefaultName = true;
  addProperty(prop) {
    this.moduleExport.declaration.type.add(prop);
  }
  addSchemaArgs(args) {
    for (const arg of args) {
      const inputField = buildInputField(arg, this.context);
      const docComment2 = getArgFieldJSDoc(this.type, this.action, arg);
      if (docComment2) {
        inputField.setDocComment(docComment(docComment2));
      }
      this.addProperty(inputField);
    }
    return this;
  }
  addSelectArg(selectTypeName = getSelectName(this.type.name)) {
    this.addProperty(
      property(
        "select",
        unionType([namedType(selectTypeName).addGenericArgument(extArgsParam.toArgument()), nullType])
      ).optional().setDocComment(docComment(`Select specific fields to fetch from the ${this.type.name}`))
    );
    return this;
  }
  addIncludeArgIfHasRelations(includeTypeName = getIncludeName(this.type.name), type = this.type) {
    const hasRelationField = type.fields.some((f) => f.outputType.location === "outputObjectTypes");
    if (!hasRelationField) {
      return this;
    }
    this.addProperty(
      property(
        "include",
        unionType([namedType(includeTypeName).addGenericArgument(extArgsParam.toArgument()), nullType])
      ).optional().setDocComment(docComment("Choose, which related nodes to fetch as well"))
    );
    return this;
  }
  addOmitArg() {
    this.addProperty(
      property(
        "omit",
        unionType([
          namedType(getOmitName(this.type.name)).addGenericArgument(extArgsParam.toArgument()),
          nullType
        ])
      ).optional().setDocComment(docComment(`Omit specific fields from the ${this.type.name}`))
    );
    return this;
  }
  setGeneratedName(name) {
    this.hasDefaultName = false;
    this.moduleExport.declaration.setName(name);
    return this;
  }
  setComment(comment) {
    this.moduleExport.setDocComment(docComment(comment));
    return this;
  }
  createExport() {
    return this.moduleExport;
  }
};

// ../client-generator-js/src/TSClient/ModelFieldRefs.ts
var ModelFieldRefs = class {
  constructor(outputType) {
    this.outputType = outputType;
  }
  toTS() {
    const { name } = this.outputType;
    return `

/**
 * Fields of the ${name} model
 */
interface ${getFieldRefsTypeName(name)} {
${this.stringifyFields()}
}
    `;
  }
  stringifyFields() {
    const { name } = this.outputType;
    return this.outputType.fields.filter((field) => field.outputType.location !== "outputObjectTypes").map((field) => {
      const fieldOutput = field.outputType;
      const refTypeName = getRefAllowedTypeName(fieldOutput);
      return `  readonly ${field.name}: FieldRef<"${name}", ${refTypeName}>`;
    }).join("\n");
  }
};

// ../client-generator-js/src/TSClient/Output.ts
function buildModelOutputProperty(field, dmmf) {
  let fieldTypeName = hasOwnProperty(GraphQLScalarToJSTypeTable, field.type) ? GraphQLScalarToJSTypeTable[field.type] : field.type;
  if (Array.isArray(fieldTypeName)) {
    fieldTypeName = fieldTypeName[0];
  }
  if (needsNamespace(field)) {
    fieldTypeName = `Prisma.${fieldTypeName}`;
  }
  let fieldType;
  if (field.kind === "object") {
    const payloadType = namedType(getPayloadName(field.type));
    if (!dmmf.isComposite(field.type)) {
      payloadType.addGenericArgument(namedType("ExtArgs"));
    }
    fieldType = payloadType;
  } else if (field.kind === "enum") {
    fieldType = namedType(`$Enums.${fieldTypeName}`);
  } else {
    fieldType = namedType(fieldTypeName);
  }
  if (field.isList) {
    fieldType = array(fieldType);
  } else if (!field.isRequired) {
    fieldType = unionType(fieldType).addVariant(nullType);
  }
  const property2 = property(field.name, fieldType);
  if (field.documentation) {
    property2.setDocComment(docComment(field.documentation));
  }
  return property2;
}
function buildOutputType(type) {
  return moduleExport(typeDeclaration(type.name, objectType().addMultiple(type.fields.map(buildOutputField))));
}
function buildOutputField(field) {
  let fieldType;
  if (field.outputType.location === "enumTypes" && field.outputType.namespace === "model") {
    fieldType = namedType(enumTypeName(field.outputType));
  } else {
    const typeNames = GraphQLScalarToJSTypeTable[field.outputType.type] ?? field.outputType.type;
    fieldType = Array.isArray(typeNames) ? namedType(typeNames[0]) : namedType(typeNames);
  }
  if (field.outputType.isList) {
    fieldType = array(fieldType);
  } else if (field.isNullable) {
    fieldType = unionType(fieldType).addVariant(nullType);
  }
  const property2 = property(field.name, fieldType);
  if (field.deprecation) {
    property2.setDocComment(
      docComment(`@deprecated since ${field.deprecation.sinceVersion} because ${field.deprecation.reason}`)
    );
  }
  return property2;
}
function enumTypeName(ref) {
  const name = ref.type;
  const namespace2 = ref.namespace === "model" ? "$Enums" : "Prisma";
  return `${namespace2}.${name}`;
}

// ../client-generator-js/src/TSClient/Payload.ts
function buildModelPayload(model, context) {
  const isComposite = context.dmmf.isComposite(model.name);
  const objects = objectType();
  const scalars = objectType();
  const composites = objectType();
  for (const field of model.fields) {
    if (field.kind === "object") {
      if (context.dmmf.isComposite(field.type)) {
        composites.add(buildModelOutputProperty(field, context.dmmf));
      } else {
        objects.add(buildModelOutputProperty(field, context.dmmf));
      }
    } else if (field.kind === "enum" || field.kind === "scalar") {
      scalars.add(buildModelOutputProperty(field, context.dmmf));
    }
  }
  const scalarsType = isComposite ? scalars : namedType("$Extensions.GetPayloadResult").addGenericArgument(scalars).addGenericArgument(namedType("ExtArgs").subKey("result").subKey(uncapitalize(model.name)));
  const payloadTypeDeclaration = typeDeclaration(
    getPayloadName(model.name, false),
    objectType().add(property("name", stringLiteral(model.name))).add(property("objects", objects)).add(property("scalars", scalarsType)).add(property("composites", composites))
  );
  if (!isComposite) {
    payloadTypeDeclaration.addGenericParameter(extArgsParam);
  }
  return moduleExport(payloadTypeDeclaration);
}

// ../client-generator-js/src/TSClient/SelectIncludeOmit.ts
function buildIncludeType({
  modelName,
  typeName = getIncludeName(modelName),
  context,
  fields
}) {
  const type = buildSelectOrIncludeObject(modelName, getIncludeFields(fields, context.dmmf), context);
  return buildExport(typeName, type);
}
function buildOmitType({ modelName, fields, context }) {
  const keysType = unionType(
    fields.filter(
      (field) => field.outputType.location === "scalar" || field.outputType.location === "enumTypes" || context.dmmf.isComposite(field.outputType.type)
    ).map((field) => stringLiteral(field.name))
  );
  const omitType = namedType("$Extensions.GetOmit").addGenericArgument(keysType).addGenericArgument(modelResultExtensionsType(modelName));
  if (context.isPreviewFeatureOn("strictUndefinedChecks")) {
    omitType.addGenericArgument(namedType("$Types.Skip"));
  }
  return buildExport(getOmitName(modelName), omitType);
}
function buildSelectType({
  modelName,
  typeName = getSelectName(modelName),
  fields,
  context
}) {
  const objectType2 = buildSelectOrIncludeObject(modelName, fields, context);
  const selectType = namedType("$Extensions.GetSelect").addGenericArgument(objectType2).addGenericArgument(modelResultExtensionsType(modelName));
  return buildExport(typeName, selectType);
}
function modelResultExtensionsType(modelName) {
  return extArgsParam.toArgument().subKey("result").subKey(uncapitalize(modelName));
}
function buildScalarSelectType({ modelName, fields, context }) {
  const object = buildSelectOrIncludeObject(
    modelName,
    fields.filter((field) => field.outputType.location === "scalar" || field.outputType.location === "enumTypes"),
    context
  );
  return moduleExport(typeDeclaration(`${getSelectName(modelName)}Scalar`, object));
}
function buildSelectOrIncludeObject(modelName, fields, context) {
  const objectType2 = objectType();
  for (const field of fields) {
    const fieldType = unionType(booleanType);
    if (field.outputType.location === "outputObjectTypes") {
      const subSelectType = namedType(getFieldArgName(field, modelName));
      subSelectType.addGenericArgument(extArgsParam.toArgument());
      fieldType.addVariant(subSelectType);
    }
    objectType2.add(property(field.name, appendSkipType(context, fieldType)).optional());
  }
  return objectType2;
}
function buildExport(typeName, type) {
  const declaration = typeDeclaration(typeName, type);
  return moduleExport(declaration.addGenericParameter(extArgsParam));
}
function getIncludeFields(fields, dmmf) {
  return fields.filter((field) => {
    if (field.outputType.location !== "outputObjectTypes") {
      return false;
    }
    return !dmmf.isComposite(field.outputType.type);
  });
}

// ../client-generator-js/src/TSClient/utils/getModelActions.ts
function getModelActions(dmmf, name) {
  const mapping = dmmf.mappingsMap[name] ?? { model: name, plural: `${name}s` };
  const mappingKeys = Object.keys(mapping).filter(
    (key) => key !== "model" && key !== "plural" && mapping[key]
  );
  if ("aggregate" in mapping) {
    mappingKeys.push("count");
  }
  return mappingKeys;
}

// ../client-generator-js/src/TSClient/utils/type-builders.ts
function promise(resultType) {
  return new NamedType("$Utils.JsPromise").addGenericArgument(resultType);
}
function prismaPromise(resultType) {
  return new NamedType("Prisma.PrismaPromise").addGenericArgument(resultType);
}
function optional(innerType) {
  return new NamedType("$Utils.Optional").addGenericArgument(innerType);
}

// ../client-generator-js/src/TSClient/Model.ts
var Model = class {
  constructor(model, context) {
    this.model = model;
    this.context = context;
    this.dmmf = context.dmmf;
    this.type = this.context.dmmf.outputTypeMap.model[model.name];
    this.createManyAndReturnType = this.context.dmmf.outputTypeMap.model[getCreateManyAndReturnOutputType(model.name)];
    this.updateManyAndReturnType = this.context.dmmf.outputTypeMap.model[getUpdateManyAndReturnOutputType(model.name)];
    this.mapping = this.context.dmmf.mappings.modelOperations.find((m) => m.model === model.name);
  }
  type;
  createManyAndReturnType;
  updateManyAndReturnType;
  mapping;
  dmmf;
  get argsTypes() {
    const argsTypes = [];
    for (const action2 of Object.keys(ModelAction)) {
      const fieldName = this.rootFieldNameForAction(action2);
      if (!fieldName) {
        continue;
      }
      const field = this.dmmf.rootFieldMap[fieldName];
      if (!field) {
        throw new Error(`Oops this must not happen. Could not find field ${fieldName} on either Query or Mutation`);
      }
      if (action2 === "updateMany" || action2 === "deleteMany" || action2 === "createMany" || action2 === "findRaw" || action2 === "aggregateRaw") {
        argsTypes.push(
          new ArgsTypeBuilder(this.type, this.context, action2).addSchemaArgs(field.args).createExport()
        );
      } else if (action2 === "createManyAndReturn") {
        const args = new ArgsTypeBuilder(this.type, this.context, action2).addSelectArg(getSelectCreateManyAndReturnName(this.type.name)).addOmitArg().addSchemaArgs(field.args);
        if (this.createManyAndReturnType) {
          args.addIncludeArgIfHasRelations(
            getIncludeCreateManyAndReturnName(this.model.name),
            this.createManyAndReturnType
          );
        }
        argsTypes.push(args.createExport());
      } else if (action2 === "updateManyAndReturn") {
        const args = new ArgsTypeBuilder(this.type, this.context, action2).addSelectArg(getSelectUpdateManyAndReturnName(this.type.name)).addOmitArg().addSchemaArgs(field.args);
        if (this.updateManyAndReturnType) {
          args.addIncludeArgIfHasRelations(
            getIncludeUpdateManyAndReturnName(this.model.name),
            this.updateManyAndReturnType
          );
        }
        argsTypes.push(args.createExport());
      } else if (action2 !== "groupBy" && action2 !== "aggregate") {
        argsTypes.push(
          new ArgsTypeBuilder(this.type, this.context, action2).addSelectArg().addOmitArg().addIncludeArgIfHasRelations().addSchemaArgs(field.args).createExport()
        );
      }
    }
    for (const field of this.type.fields) {
      if (!field.args.length) {
        continue;
      }
      const fieldOutput = this.dmmf.resolveOutputObjectType(field.outputType);
      if (!fieldOutput) {
        continue;
      }
      argsTypes.push(
        new ArgsTypeBuilder(fieldOutput, this.context).addSelectArg().addOmitArg().addIncludeArgIfHasRelations().addSchemaArgs(field.args).setGeneratedName(getModelFieldArgsName(field, this.model.name)).setComment(`${this.model.name}.${field.name}`).createExport()
      );
    }
    argsTypes.push(
      new ArgsTypeBuilder(this.type, this.context).addSelectArg().addOmitArg().addIncludeArgIfHasRelations().createExport()
    );
    return argsTypes;
  }
  rootFieldNameForAction(action2) {
    return this.mapping?.[action2];
  }
  getGroupByTypes() {
    const { model, mapping } = this;
    const groupByType = this.dmmf.outputTypeMap.prisma[getGroupByName(model.name)];
    if (!groupByType) {
      throw new Error(`Could not get group by type for model ${model.name}`);
    }
    const groupByRootField = this.dmmf.rootFieldMap[mapping.groupBy];
    if (!groupByRootField) {
      throw new Error(`Could not find groupBy root field for model ${model.name}. Mapping: ${mapping?.groupBy}`);
    }
    const groupByArgsName = getGroupByArgsName(model.name);
    return `


export type ${groupByArgsName}<ExtArgs extends $Extensions.InternalArgs = $Extensions.DefaultArgs> = {
${(0, import_indent_string3.default)(
      groupByRootField.args.map((arg) => {
        const updatedArg = { ...arg, comment: getArgFieldJSDoc(this.type, "groupBy" /* groupBy */, arg) };
        return new InputField(updatedArg, this.context).toTS();
      }).concat(
        groupByType.fields.filter((f) => f.outputType.location === "outputObjectTypes").map((f) => {
          if (f.outputType.location === "outputObjectTypes") {
            return `${f.name}?: ${getAggregateInputType(f.outputType.type)}${f.name === "_count" ? " | true" : ""}`;
          }
          return "";
        })
      ).join("\n"),
      TAB_SIZE
    )}
}

${stringify(buildOutputType(groupByType))}

type ${getGroupByPayloadName(model.name)}<T extends ${groupByArgsName}> = Prisma.PrismaPromise<
  Array<
    PickEnumerable<${groupByType.name}, T['by']> &
      {
        [P in ((keyof T) & (keyof ${groupByType.name}))]: P extends '_count'
          ? T[P] extends boolean
            ? number
            : GetScalarType<T[P], ${groupByType.name}[P]>
          : GetScalarType<T[P], ${groupByType.name}[P]>
      }
    >
  >
`;
  }
  getAggregationTypes() {
    const { model, mapping } = this;
    let aggregateType = this.dmmf.outputTypeMap.prisma[getAggregateName(model.name)];
    if (!aggregateType) {
      throw new Error(`Could not get aggregate type "${getAggregateName(model.name)}" for "${model.name}"`);
    }
    aggregateType = klona(aggregateType);
    const aggregateRootField = this.dmmf.rootFieldMap[mapping.aggregate];
    if (!aggregateRootField) {
      throw new Error(`Could not find aggregate root field for model ${model.name}. Mapping: ${mapping?.aggregate}`);
    }
    const aggregateTypes = [aggregateType];
    const avgType = this.dmmf.outputTypeMap.prisma[getAvgAggregateName(model.name)];
    const sumType = this.dmmf.outputTypeMap.prisma[getSumAggregateName(model.name)];
    const minType = this.dmmf.outputTypeMap.prisma[getMinAggregateName(model.name)];
    const maxType = this.dmmf.outputTypeMap.prisma[getMaxAggregateName(model.name)];
    const countType = this.dmmf.outputTypeMap.prisma[getCountAggregateOutputName(model.name)];
    if (avgType) {
      aggregateTypes.push(avgType);
    }
    if (sumType) {
      aggregateTypes.push(sumType);
    }
    if (minType) {
      aggregateTypes.push(minType);
    }
    if (maxType) {
      aggregateTypes.push(maxType);
    }
    if (countType) {
      aggregateTypes.push(countType);
    }
    const aggregateArgsName = getAggregateArgsName(model.name);
    const aggregateName = getAggregateName(model.name);
    return `${aggregateTypes.map(buildOutputType).map((type) => stringify(type)).join("\n\n")}

${aggregateTypes.length > 1 ? aggregateTypes.slice(1).map((type) => {
      const newType = {
        name: getAggregateInputType(type.name),
        constraints: {
          maxNumFields: null,
          minNumFields: null
        },
        fields: type.fields.map((field) => ({
          ...field,
          name: field.name,
          isNullable: false,
          isRequired: false,
          isParameterizable: false,
          inputTypes: [
            {
              isList: false,
              location: "scalar",
              type: "true"
            }
          ]
        }))
      };
      return new InputType(newType, this.context).toTS();
    }).join("\n") : ""}

export type ${aggregateArgsName}<ExtArgs extends $Extensions.InternalArgs = $Extensions.DefaultArgs> = {
${(0, import_indent_string3.default)(
      aggregateRootField.args.map((arg) => {
        const updatedArg = { ...arg, comment: getArgFieldJSDoc(this.type, "aggregate" /* aggregate */, arg) };
        return new InputField(updatedArg, this.context).toTS();
      }).concat(
        aggregateType.fields.map((f) => {
          let data = "";
          const comment = getArgFieldJSDoc(this.type, "aggregate" /* aggregate */, f.name);
          data += comment ? wrapComment(comment) + "\n" : "";
          if (f.name === "_count" || f.name === "count") {
            data += `${f.name}?: true | ${getCountAggregateInputName(model.name)}`;
          } else {
            data += `${f.name}?: ${getAggregateInputType(f.outputType.type)}`;
          }
          return data;
        })
      ).join("\n"),
      TAB_SIZE
    )}
}

export type ${getAggregateGetName(model.name)}<T extends ${getAggregateArgsName(model.name)}> = {
      [P in keyof T & keyof ${aggregateName}]: P extends '_count' | 'count'
    ? T[P] extends true
      ? number
      : GetScalarType<T[P], ${aggregateName}[P]>
    : GetScalarType<T[P], ${aggregateName}[P]>
}`;
  }
  toTSWithoutNamespace() {
    const { model } = this;
    const docLines = model.documentation ?? "";
    const modelLine = `Model ${model.name}
`;
    const docs = `${modelLine}${docLines}`;
    const modelTypeExport = moduleExport(
      typeDeclaration(
        model.name,
        namedType(`$Result.DefaultSelection`).addGenericArgument(namedType(getPayloadName(model.name)))
      )
    ).setDocComment(docComment(docs));
    return stringify(modelTypeExport);
  }
  toTS() {
    const { model } = this;
    const isComposite = this.dmmf.isComposite(model.name);
    const omitType = stringify(
      buildOmitType({ modelName: this.model.name, context: this.context, fields: this.type.fields }),
      {
        newLine: "leading"
      }
    );
    const hasRelationField = model.fields.some((f) => f.kind === "object");
    const includeType = hasRelationField ? stringify(
      buildIncludeType({ modelName: this.model.name, context: this.context, fields: this.type.fields }),
      {
        newLine: "leading"
      }
    ) : "";
    const createManyAndReturnIncludeType = hasRelationField && this.createManyAndReturnType ? stringify(
      buildIncludeType({
        typeName: getIncludeCreateManyAndReturnName(this.model.name),
        modelName: this.model.name,
        context: this.context,
        fields: this.createManyAndReturnType.fields
      }),
      {
        newLine: "leading"
      }
    ) : "";
    const updateManyAndReturnIncludeType = hasRelationField && this.updateManyAndReturnType ? stringify(
      buildIncludeType({
        typeName: getIncludeUpdateManyAndReturnName(this.model.name),
        modelName: this.model.name,
        context: this.context,
        fields: this.updateManyAndReturnType.fields
      }),
      {
        newLine: "leading"
      }
    ) : "";
    return `
/**
 * Model ${model.name}
 */

${!isComposite ? this.getAggregationTypes() : ""}

${!isComposite ? this.getGroupByTypes() : ""}

${stringify(buildSelectType({ modelName: this.model.name, fields: this.type.fields, context: this.context }))}
${this.createManyAndReturnType ? stringify(
      buildSelectType({
        modelName: this.model.name,
        fields: this.createManyAndReturnType.fields,
        context: this.context,
        typeName: getSelectCreateManyAndReturnName(this.model.name)
      }),
      { newLine: "leading" }
    ) : ""}
${this.updateManyAndReturnType ? stringify(
      buildSelectType({
        modelName: this.model.name,
        fields: this.updateManyAndReturnType.fields,
        context: this.context,
        typeName: getSelectUpdateManyAndReturnName(this.model.name)
      }),
      { newLine: "leading" }
    ) : ""}
${stringify(buildScalarSelectType({ modelName: this.model.name, fields: this.type.fields, context: this.context }), {
      newLine: "leading"
    })}
${omitType}${includeType}${createManyAndReturnIncludeType}${updateManyAndReturnIncludeType}

${stringify(buildModelPayload(this.model, this.context), { newLine: "none" })}

type ${model.name}GetPayload<S extends boolean | null | undefined | ${getModelArgName(
      model.name
    )}> = $Result.GetResult<${getPayloadName(model.name)}, S>

${isComposite ? "" : new ModelDelegate(this.type, this.context).toTS()}

${new ModelFieldRefs(this.type).toTS()}

// Custom InputTypes
${this.argsTypes.map((type) => stringify(type)).join("\n\n")}
`;
  }
};
var ModelDelegate = class {
  constructor(outputType, context) {
    this.outputType = outputType;
    this.context = context;
  }
  /**
   * Returns all available non-aggregate or group actions
   * Includes both dmmf and client-only actions
   *
   * @param availableActions
   * @returns
   */
  getNonAggregateActions(availableActions) {
    const actions = availableActions.filter(
      (key) => key !== "aggregate" /* aggregate */ && key !== "groupBy" /* groupBy */ && key !== "count" /* count */
    );
    return actions;
  }
  toTS() {
    const { name } = this.outputType;
    const { dmmf } = this.context;
    const mapping = dmmf.mappingsMap[name] ?? { model: name, plural: `${name}s` };
    const modelOrType = dmmf.typeAndModelMap[name];
    const availableActions = getModelActions(dmmf, name);
    const nonAggregateActions = this.getNonAggregateActions(availableActions);
    const groupByArgsName = getGroupByArgsName(name);
    const countArgsName = getModelArgName(name, "count" /* count */);
    const genericDelegateParams = [extArgsParam, genericParameter("GlobalOmitOptions").default(objectType())];
    const excludedArgsForCount = ["select", "include", "distinct", "omit"];
    if (this.context.isPreviewFeatureOn("relationJoins")) {
      excludedArgsForCount.push("relationLoadStrategy");
    }
    const excludedArgsForCountType = excludedArgsForCount.map((name2) => `'${name2}'`).join(" | ");
    return `${availableActions.includes("aggregate" /* aggregate */) ? `type ${countArgsName}<ExtArgs extends $Extensions.InternalArgs = $Extensions.DefaultArgs> =
  Omit<${getModelArgName(name, "findMany" /* findMany */)}, ${excludedArgsForCountType}> & {
    select?: ${getCountAggregateInputName(name)} | true
  }
` : ""}
export interface ${name}Delegate<${genericDelegateParams.map((param) => stringify(param)).join(", ")}> {
${(0, import_indent_string3.default)(`[K: symbol]: { types: Prisma.TypeMap<ExtArgs>['model']['${name}'], meta: { name: '${name}' } }`, TAB_SIZE)}
${nonAggregateActions.map((action2) => {
      const method2 = buildModelDelegateMethod(name, action2, this.context);
      return stringify(method2, { indentLevel: 1, newLine: "trailing" });
    }).join("\n")}

${availableActions.includes("aggregate" /* aggregate */) ? `${(0, import_indent_string3.default)(getMethodJSDoc("count" /* count */, mapping, modelOrType), TAB_SIZE)}
  count<T extends ${countArgsName}>(
    args?: Subset<T, ${countArgsName}>,
  ): Prisma.PrismaPromise<
    T extends $Utils.Record<'select', any>
      ? T['select'] extends true
        ? number
        : GetScalarType<T['select'], ${getCountAggregateOutputName(name)}>
      : number
  >
` : ""}
${availableActions.includes("aggregate" /* aggregate */) ? `${(0, import_indent_string3.default)(getMethodJSDoc("aggregate" /* aggregate */, mapping, modelOrType), TAB_SIZE)}
  aggregate<T extends ${getAggregateArgsName(name)}>(args: Subset<T, ${getAggregateArgsName(
      name
    )}>): Prisma.PrismaPromise<${getAggregateGetName(name)}<T>>
` : ""}
${availableActions.includes("groupBy" /* groupBy */) ? `${(0, import_indent_string3.default)(getMethodJSDoc("groupBy" /* groupBy */, mapping, modelOrType), TAB_SIZE)}
  groupBy<
    T extends ${groupByArgsName},
    HasSelectOrTake extends Or<
      Extends<'skip', Keys<T>>,
      Extends<'take', Keys<T>>
    >,
    OrderByArg extends True extends HasSelectOrTake
      ? { orderBy: ${groupByArgsName}['orderBy'] }
      : { orderBy?: ${groupByArgsName}['orderBy'] },
    OrderFields extends ExcludeUnderscoreKeys<Keys<MaybeTupleToUnion<T['orderBy']>>>,
    ByFields extends MaybeTupleToUnion<T['by']>,
    ByValid extends Has<ByFields, OrderFields>,
    HavingFields extends GetHavingFields<T['having']>,
    HavingValid extends Has<ByFields, HavingFields>,
    ByEmpty extends T['by'] extends never[] ? True : False,
    InputErrors extends ByEmpty extends True
    ? \`Error: "by" must not be empty.\`
    : HavingValid extends False
    ? {
        [P in HavingFields]: P extends ByFields
          ? never
          : P extends string
          ? \`Error: Field "\${P}" used in "having" needs to be provided in "by".\`
          : [
              Error,
              'Field ',
              P,
              \` in "having" needs to be provided in "by"\`,
            ]
      }[HavingFields]
    : 'take' extends Keys<T>
    ? 'orderBy' extends Keys<T>
      ? ByValid extends True
        ? {}
        : {
            [P in OrderFields]: P extends ByFields
              ? never
              : \`Error: Field "\${P}" in "orderBy" needs to be provided in "by"\`
          }[OrderFields]
      : 'Error: If you provide "take", you also need to provide "orderBy"'
    : 'skip' extends Keys<T>
    ? 'orderBy' extends Keys<T>
      ? ByValid extends True
        ? {}
        : {
            [P in OrderFields]: P extends ByFields
              ? never
              : \`Error: Field "\${P}" in "orderBy" needs to be provided in "by"\`
          }[OrderFields]
      : 'Error: If you provide "skip", you also need to provide "orderBy"'
    : ByValid extends True
    ? {}
    : {
        [P in OrderFields]: P extends ByFields
          ? never
          : \`Error: Field "\${P}" in "orderBy" needs to be provided in "by"\`
      }[OrderFields]
  >(args: SubsetIntersection<T, ${groupByArgsName}, OrderByArg> & InputErrors): {} extends InputErrors ? ${getGroupByPayloadName(
      name
    )}<T> : Prisma.PrismaPromise<InputErrors>` : ""}
/**
 * Fields of the ${name} model
 */
readonly fields: ${getFieldRefsTypeName(name)};
}

${stringify(buildFluentWrapperDefinition(name, this.outputType, this.context))}
`;
  }
};
function buildModelDelegateMethod(modelName, actionName, context) {
  const mapping = context.dmmf.mappingsMap[modelName] ?? { model: modelName, plural: `${modelName}s` };
  const modelOrType = context.dmmf.typeAndModelMap[modelName];
  const dependencyValidators = getNonAggregateMethodDependencyValidations(mapping, actionName, context);
  const method2 = method(actionName).setDocComment(docComment(getMethodJSDocBody(actionName, mapping, modelOrType))).addParameter(getNonAggregateMethodArgs(modelName, actionName, dependencyValidators)).setReturnType(getReturnType({ modelName, actionName }));
  const generic = getNonAggregateMethodGenericParam(modelName, actionName);
  if (generic) {
    method2.addGenericParameter(generic);
  }
  for (const validator of dependencyValidators) {
    method2.addGenericParameter(validator);
  }
  return method2;
}
function getNonAggregateMethodArgs(modelName, actionName, dependencyValidators) {
  const makeParameter = (type2) => {
    if (dependencyValidators.length > 0) {
      type2 = intersectionType([type2, ...dependencyValidators.map((validator) => namedType(validator.name))]);
    }
    return parameter("args", type2);
  };
  if (actionName === "count" /* count */) {
    const type2 = omit(
      namedType(getModelArgName(modelName, "findMany" /* findMany */)),
      unionType(stringLiteral("select")).addVariant(stringLiteral("include")).addVariant(stringLiteral("distinct"))
    );
    return makeParameter(type2).optional();
  }
  if (actionName === "findRaw" /* findRaw */ || actionName === "aggregateRaw" /* aggregateRaw */) {
    return makeParameter(namedType(getModelArgName(modelName, actionName))).optional();
  }
  const type = namedType("SelectSubset").addGenericArgument(namedType("T")).addGenericArgument(
    namedType(getModelArgName(modelName, actionName)).addGenericArgument(extArgsParam.toArgument())
  );
  const param = makeParameter(type);
  if (actionName === "findMany" /* findMany */ || actionName === "findFirst" /* findFirst */ || actionName === "deleteMany" /* deleteMany */ || actionName === "createMany" /* createMany */ || actionName === "createManyAndReturn" /* createManyAndReturn */ || actionName === "findFirstOrThrow" /* findFirstOrThrow */) {
    param.optional();
  }
  return param;
}
function getNonAggregateMethodGenericParam(modelName, actionName) {
  if (actionName === "count" /* count */ || actionName === "findRaw" /* findRaw */ || actionName === "aggregateRaw" /* aggregateRaw */) {
    return null;
  }
  const arg = genericParameter("T");
  if (actionName === "aggregate" /* aggregate */) {
    return arg.extends(namedType(getAggregateArgsName(modelName)));
  }
  return arg.extends(namedType(getModelArgName(modelName, actionName)));
}
function getNonAggregateMethodDependencyValidations(modelMapping, actionName, context) {
  const outputFieldName = modelMapping[actionName];
  if (!outputFieldName) {
    throw new Error(`Missing mapping for ${modelMapping.model}.${actionName}`);
  }
  const outputField = context.dmmf.outputTypeMap.prisma["Query"].fields.find((f) => f.name === outputFieldName) ?? context.dmmf.outputTypeMap.prisma["Mutation"].fields.find((f) => f.name === outputFieldName);
  if (!outputField) {
    throw new Error(`Can't find output field ${outputFieldName} in the schema`);
  }
  const validators = [];
  for (const args of outputField.args) {
    if (args.requiresOtherFields === void 0) {
      continue;
    }
    const objectType2 = objectType();
    for (const reqArg of args.requiresOtherFields) {
      objectType2.add(property(reqArg, objectType()));
    }
    validators.push(
      genericParameter(`${capitalize(args.name)}DependenciesValidator`).extends(
        conditionalType().check(stringLiteral(args.name)).extends(namedType("Prisma.Keys<T>")).then(objectType2).else(objectType())
      )
    );
  }
  return validators;
}
function getReturnType({
  modelName,
  actionName,
  isChaining = false,
  isNullable = false
}) {
  if (actionName === "count" /* count */) {
    return promise(numberType);
  }
  if (actionName === "aggregate" /* aggregate */) {
    return promise(namedType(getAggregateGetName(modelName)).addGenericArgument(namedType("T")));
  }
  if (actionName === "findRaw" /* findRaw */ || actionName === "aggregateRaw" /* aggregateRaw */) {
    return prismaPromise(namedType("JsonObject"));
  }
  if (actionName === "deleteMany" /* deleteMany */ || actionName === "updateMany" /* updateMany */ || actionName === "createMany" /* createMany */) {
    return prismaPromise(namedType("BatchPayload"));
  }
  const isList = actionName === "findMany" /* findMany */ || actionName === "createManyAndReturn" /* createManyAndReturn */ || actionName === "updateManyAndReturn" /* updateManyAndReturn */;
  if (isList) {
    let result = getResultType(modelName, actionName);
    if (isChaining) {
      result = unionType(result).addVariant(namedType("Null"));
    }
    return prismaPromise(result);
  }
  if (isChaining && actionName === "findUniqueOrThrow" /* findUniqueOrThrow */) {
    const nullType2 = isNullable ? nullType : namedType("Null");
    const result = unionType(getResultType(modelName, actionName)).addVariant(nullType2);
    return getFluentWrapper(modelName, result, nullType2);
  }
  if (actionName === "findFirst" /* findFirst */ || actionName === "findUnique" /* findUnique */) {
    const result = unionType(getResultType(modelName, actionName)).addVariant(nullType);
    return getFluentWrapper(modelName, result, nullType);
  }
  return getFluentWrapper(modelName, getResultType(modelName, actionName));
}
function getFluentWrapper(modelName, resultType, nullType2 = neverType) {
  return namedType(fluentWrapperName(modelName)).addGenericArgument(resultType).addGenericArgument(nullType2).addGenericArgument(extArgsParam.toArgument()).addGenericArgument(namedType("GlobalOmitOptions"));
}
function getResultType(modelName, actionName) {
  return namedType("$Result.GetResult").addGenericArgument(namedType(getPayloadName(modelName)).addGenericArgument(extArgsParam.toArgument())).addGenericArgument(namedType("T")).addGenericArgument(stringLiteral(actionName)).addGenericArgument(namedType("GlobalOmitOptions"));
}
function buildFluentWrapperDefinition(modelName, outputType, context) {
  const definition = interfaceDeclaration(fluentWrapperName(modelName));
  definition.addGenericParameter(genericParameter("T")).addGenericParameter(genericParameter("Null").default(neverType)).addGenericParameter(extArgsParam).addGenericParameter(genericParameter("GlobalOmitOptions").default(objectType())).extends(prismaPromise(namedType("T")));
  definition.add(property(toStringTag, stringLiteral("PrismaPromise")).readonly());
  definition.addMultiple(
    outputType.fields.filter(
      (field) => field.outputType.location === "outputObjectTypes" && !context.dmmf.isComposite(field.outputType.type) && field.name !== "_count"
    ).map((field) => {
      const fieldArgType = namedType(getFieldArgName(field, modelName)).addGenericArgument(extArgsParam.toArgument());
      const argsParam = genericParameter("T").extends(fieldArgType).default(objectType());
      return method(field.name).addGenericParameter(argsParam).addParameter(parameter("args", subset(argsParam.toArgument(), fieldArgType)).optional()).setReturnType(
        getReturnType({
          modelName: field.outputType.type,
          actionName: field.outputType.isList ? "findMany" /* findMany */ : "findUniqueOrThrow" /* findUniqueOrThrow */,
          isChaining: true,
          isNullable: field.isNullable
        })
      );
    })
  );
  definition.add(
    method("then").setDocComment(
      docComment`
          Attaches callbacks for the resolution and/or rejection of the Promise.
          @param onfulfilled The callback to execute when the Promise is resolved.
          @param onrejected The callback to execute when the Promise is rejected.
          @returns A Promise for the completion of which ever callback is executed.
        `
    ).addGenericParameter(genericParameter("TResult1").default(namedType("T"))).addGenericParameter(genericParameter("TResult2").default(neverType)).addParameter(promiseCallback("onfulfilled", parameter("value", namedType("T")), namedType("TResult1"))).addParameter(promiseCallback("onrejected", parameter("reason", anyType), namedType("TResult2"))).setReturnType(promise(unionType([namedType("TResult1"), namedType("TResult2")])))
  );
  definition.add(
    method("catch").setDocComment(
      docComment`
          Attaches a callback for only the rejection of the Promise.
          @param onrejected The callback to execute when the Promise is rejected.
          @returns A Promise for the completion of the callback.
        `
    ).addGenericParameter(genericParameter("TResult").default(neverType)).addParameter(promiseCallback("onrejected", parameter("reason", anyType), namedType("TResult"))).setReturnType(promise(unionType([namedType("T"), namedType("TResult")])))
  );
  definition.add(
    method("finally").setDocComment(
      docComment`
          Attaches a callback that is invoked when the Promise is settled (fulfilled or rejected). The
          resolved value cannot be modified from the callback.
          @param onfinally The callback to execute when the Promise is settled (fulfilled or rejected).
          @returns A Promise for the completion of the callback.
      `
    ).addParameter(
      parameter("onfinally", unionType([functionType(), undefinedType, nullType])).optional()
    ).setReturnType(promise(namedType("T")))
  );
  return moduleExport(definition).setDocComment(docComment`
      The delegate class that acts as a "Promise-like" for ${modelName}.
      Why is this prefixed with \`Prisma__\`?
      Because we want to prevent naming conflicts as mentioned in
      https://github.com/prisma/prisma-client-js/issues/707
    `);
}
function promiseCallback(name, callbackParam, returnType) {
  return parameter(
    name,
    unionType([
      functionType().addParameter(callbackParam).setReturnType(typeOrPromiseLike(returnType)),
      undefinedType,
      nullType
    ])
  ).optional();
}
function typeOrPromiseLike(type) {
  return unionType([type, namedType("PromiseLike").addGenericArgument(type)]);
}
function subset(arg, baseType) {
  return namedType("Subset").addGenericArgument(arg).addGenericArgument(baseType);
}
function fluentWrapperName(modelName) {
  return `Prisma__${modelName}Client`;
}

// ../param-graph/src/serialization.ts
function serializeParamGraph(data) {
  return new Serializer(data).serialize();
}
function encodeBase64url(bytes) {
  return Buffer.from(bytes.buffer, bytes.byteOffset, bytes.byteLength).toString("base64url");
}
function varuintSize(value) {
  let size = 1;
  while (value >= 128) {
    size++;
    value >>>= 7;
  }
  return size;
}
var Serializer = class {
  #data;
  #buffer;
  #view;
  #offset = 0;
  #rootKeys;
  constructor(data) {
    this.#data = data;
    this.#rootKeys = Object.keys(data.roots);
    const size = this.#calculateBufferSize();
    this.#buffer = new ArrayBuffer(size);
    this.#view = new DataView(this.#buffer);
  }
  serialize() {
    this.#writeHeader();
    this.#writeInputNodes();
    this.#writeOutputNodes();
    this.#writeRoots();
    return {
      strings: this.#data.strings,
      graph: encodeBase64url(new Uint8Array(this.#buffer, 0, this.#offset))
    };
  }
  #writeVaruint(value) {
    while (value >= 128) {
      this.#view.setUint8(this.#offset++, value & 127 | 128);
      value >>>= 7;
    }
    this.#view.setUint8(this.#offset++, value);
  }
  #writeOptionalVaruint(value) {
    this.#writeVaruint(value === void 0 ? 0 : value + 1);
  }
  #writeByte(value) {
    this.#view.setUint8(this.#offset, value);
    this.#offset += 1;
  }
  #writeU16(value) {
    this.#view.setUint16(this.#offset, value, true);
    this.#offset += 2;
  }
  #calculateBufferSize() {
    let size = 0;
    size += varuintSize(this.#data.inputNodes.length);
    size += varuintSize(this.#data.outputNodes.length);
    size += varuintSize(this.#rootKeys.length);
    for (const node of this.#data.inputNodes) {
      const fieldIndices = Object.keys(node.edges).map(Number);
      size += varuintSize(fieldIndices.length);
      for (const fieldIndex of fieldIndices) {
        const edge = node.edges[fieldIndex];
        size += varuintSize(fieldIndex);
        size += 2;
        size += varuintSize(edge.childNodeId === void 0 ? 0 : edge.childNodeId + 1);
        size += varuintSize(edge.enumNameIndex === void 0 ? 0 : edge.enumNameIndex + 1);
        size += 1;
      }
    }
    for (const node of this.#data.outputNodes) {
      const fieldIndices = Object.keys(node.edges).map(Number);
      size += varuintSize(fieldIndices.length);
      for (const fieldIndex of fieldIndices) {
        const edge = node.edges[fieldIndex];
        size += varuintSize(fieldIndex);
        size += varuintSize(edge.argsNodeId === void 0 ? 0 : edge.argsNodeId + 1);
        size += varuintSize(edge.outputNodeId === void 0 ? 0 : edge.outputNodeId + 1);
      }
    }
    for (const key of this.#rootKeys) {
      const root = this.#data.roots[key];
      const keyIndex = this.#data.strings.indexOf(key);
      size += varuintSize(keyIndex);
      size += varuintSize(root.argsNodeId === void 0 ? 0 : root.argsNodeId + 1);
      size += varuintSize(root.outputNodeId === void 0 ? 0 : root.outputNodeId + 1);
    }
    return size;
  }
  #writeHeader() {
    this.#writeVaruint(this.#data.inputNodes.length);
    this.#writeVaruint(this.#data.outputNodes.length);
    this.#writeVaruint(this.#rootKeys.length);
  }
  #writeInputNodes() {
    for (const node of this.#data.inputNodes) {
      const fieldIndices = Object.keys(node.edges).map(Number);
      this.#writeVaruint(fieldIndices.length);
      for (const fieldIndex of fieldIndices) {
        const edge = node.edges[fieldIndex];
        this.#writeVaruint(fieldIndex);
        this.#writeU16(edge.scalarMask ?? 0);
        this.#writeOptionalVaruint(edge.childNodeId);
        this.#writeOptionalVaruint(edge.enumNameIndex);
        this.#writeByte(edge.flags);
      }
    }
  }
  #writeOutputNodes() {
    for (const node of this.#data.outputNodes) {
      const fieldIndices = Object.keys(node.edges).map(Number);
      this.#writeVaruint(fieldIndices.length);
      for (const fieldIndex of fieldIndices) {
        const edge = node.edges[fieldIndex];
        this.#writeVaruint(fieldIndex);
        this.#writeOptionalVaruint(edge.argsNodeId);
        this.#writeOptionalVaruint(edge.outputNodeId);
      }
    }
  }
  #writeRoots() {
    for (const key of this.#rootKeys) {
      const root = this.#data.roots[key];
      const keyIndex = this.#data.strings.indexOf(key);
      if (keyIndex === -1) {
        throw new Error(`Root key "${key}" not found in strings table`);
      }
      this.#writeVaruint(keyIndex);
      this.#writeOptionalVaruint(root.argsNodeId);
      this.#writeOptionalVaruint(root.outputNodeId);
    }
  }
};

// ../param-graph/src/param-graph.ts
var EdgeFlag = {
  /**
   * Field may be parameterized as a scalar value.
   * Check ScalarMask to validate the value type.
   */
  ParamScalar: 1,
  /**
   * Field may be parameterized as an enum.
   * Check enum ID to validate the value type.
   */
  ParamEnum: 2,
  /**
   * Field accepts list-of-scalar values.
   * Parameterize the whole list if all elements match ScalarMask.
   */
  ParamListScalar: 4,
  /**
   * Field accepts list-of-enum values.
   * Parameterize the whole list if all elements match enum ID.
   */
  ParamListEnum: 8,
  /**
   * Field accepts list-of-object values.
   * Recurse into each element using the child node.
   */
  ListObject: 16,
  /**
   * Field accepts object values.
   * Recurse into child input node.
   */
  Object: 32
};
var ScalarMask = {
  String: 1,
  Int: 2,
  BigInt: 4,
  Float: 8,
  Decimal: 16,
  Boolean: 32,
  DateTime: 64,
  Json: 128,
  Bytes: 256
};
function scalarTypeToMask(typeName) {
  switch (typeName) {
    case "String":
    case "UUID":
      return ScalarMask.String;
    case "Int":
      return ScalarMask.Int;
    case "BigInt":
      return ScalarMask.BigInt;
    case "Float":
      return ScalarMask.Float;
    case "Decimal":
      return ScalarMask.Decimal;
    case "Boolean":
      return ScalarMask.Boolean;
    case "DateTime":
      return ScalarMask.DateTime;
    case "Json":
      return ScalarMask.Json;
    case "Bytes":
      return ScalarMask.Bytes;
    default:
      return 0;
  }
}

// ../param-graph-builder/src/dmmf-traverser.ts
var DMMFTraverser = class {
  #builder;
  #inputTypeMap;
  #outputTypeMap;
  #pendingInputNodes = [];
  #pendingUnionNodes = [];
  constructor(builder, dmmf) {
    this.#builder = builder;
    this.#inputTypeMap = /* @__PURE__ */ new Map();
    this.#outputTypeMap = /* @__PURE__ */ new Map();
    for (const inputType of dmmf.schema.inputObjectTypes.prisma ?? []) {
      this.#inputTypeMap.set(getTypeName(inputType.name, "prisma"), inputType);
    }
    for (const inputType of dmmf.schema.inputObjectTypes.model ?? []) {
      this.#inputTypeMap.set(getTypeName(inputType.name, "model"), inputType);
    }
    for (const outputType of dmmf.schema.outputObjectTypes.prisma ?? []) {
      this.#outputTypeMap.set(getTypeName(outputType.name, "prisma"), outputType);
    }
    for (const outputType of dmmf.schema.outputObjectTypes.model ?? []) {
      this.#outputTypeMap.set(getTypeName(outputType.name, "model"), outputType);
    }
  }
  /**
   * Process all root operations from model mappings.
   */
  processRoots(mappings) {
    for (const mapping of mappings) {
      const modelName = mapping.model;
      const actions = Object.keys(ModelAction);
      for (const action2 of actions) {
        const fieldName = mapping[action2];
        if (!fieldName) continue;
        const rootField = this.#findRootField(fieldName);
        if (!rootField) continue;
        const argsNodeId = this.buildInputNodeFromArgs(rootField.args);
        let outputNodeId;
        if (rootField.outputType.location === "outputObjectTypes") {
          outputNodeId = this.buildOutputTypeNode(
            getTypeName(rootField.outputType.type, rootField.outputType.namespace)
          );
        }
        const dmmfActionToJsonAction = {
          create: "createOne",
          update: "updateOne",
          delete: "deleteOne",
          upsert: "upsertOne"
        };
        const jsonAction = dmmfActionToJsonAction[action2] ?? action2;
        const rootKey = `${modelName}.${jsonAction}`;
        this.#builder.setRoot(rootKey, {
          argsNodeId,
          outputNodeId
        });
      }
    }
    this.#drainPendingWork();
  }
  /**
   * Iteratively processes all pending input and union nodes.
   * This avoids deep recursion that can cause stack overflow on complex schemas.
   */
  #drainPendingWork() {
    while (this.#pendingInputNodes.length > 0 || this.#pendingUnionNodes.length > 0) {
      while (this.#pendingInputNodes.length > 0) {
        const pending = this.#pendingInputNodes.pop();
        this.#processInputNodeFields(pending.nodeId, pending.fields);
      }
      while (this.#pendingUnionNodes.length > 0) {
        const pending = this.#pendingUnionNodes.pop();
        this.#processUnionNodeFields(pending.nodeId, pending.typeNames);
      }
    }
  }
  #findRootField(fieldName) {
    const queryType = this.#outputTypeMap.get("prisma.Query");
    if (queryType) {
      const field = queryType.fields.find((f) => f.name === fieldName);
      if (field) return field;
    }
    const mutationType = this.#outputTypeMap.get("prisma.Mutation");
    if (mutationType) {
      const field = mutationType.fields.find((f) => f.name === fieldName);
      if (field) return field;
    }
    return void 0;
  }
  /**
   * Builds an input node from schema arguments.
   */
  buildInputNodeFromArgs(args) {
    const edges = {};
    let hasAnyEdge = false;
    for (const arg of args) {
      const edge = this.#mergeFieldVariants([arg]);
      if (edge) {
        const stringIndex = this.#builder.internString(arg.name);
        edges[stringIndex] = edge;
        hasAnyEdge = true;
      }
    }
    if (!hasAnyEdge) {
      return void 0;
    }
    const nodeId = this.#builder.allocateInputNode();
    this.#builder.setInputNodeEdges(nodeId, edges);
    return nodeId;
  }
  /**
   * Builds an input node for a named input type.
   * Node allocation happens immediately, but field processing is deferred
   * to avoid deep recursion.
   */
  buildInputTypeNode(typeName) {
    if (this.#builder.hasInputTypeNode(typeName)) {
      return this.#builder.getInputTypeNode(typeName);
    }
    const inputType = this.#inputTypeMap.get(typeName);
    if (!inputType) {
      this.#builder.setInputTypeNode(typeName, void 0);
      return void 0;
    }
    const nodeId = this.#builder.allocateInputNode();
    this.#builder.setInputTypeNode(typeName, nodeId);
    this.#pendingInputNodes.push({ nodeId, fields: inputType.fields });
    return nodeId;
  }
  /**
   * Process fields for an input node (called from drainPendingWork).
   */
  #processInputNodeFields(nodeId, fields) {
    const edges = {};
    let hasAnyEdge = false;
    for (const field of fields) {
      const edge = this.#mergeFieldVariants([field]);
      if (edge) {
        const stringIndex = this.#builder.internString(field.name);
        edges[stringIndex] = edge;
        hasAnyEdge = true;
      }
    }
    if (hasAnyEdge) {
      this.#builder.setInputNodeEdges(nodeId, edges);
    }
  }
  /**
   * Builds a union node for multiple input types.
   * Node allocation happens immediately, but field processing is deferred
   * to avoid deep recursion.
   */
  buildUnionNode(typeNames) {
    const sortedNames = [...typeNames].sort();
    const cacheKey = sortedNames.join("|");
    if (this.#builder.hasUnionNode(cacheKey)) {
      return this.#builder.getUnionNode(cacheKey);
    }
    const nodeId = this.#builder.allocateInputNode();
    this.#builder.setUnionNode(cacheKey, nodeId);
    this.#pendingUnionNodes.push({ nodeId, typeNames });
    return nodeId;
  }
  /**
   * Process fields for a union node (called from drainPendingWork).
   */
  #processUnionNodeFields(nodeId, typeNames) {
    const fieldsByName = /* @__PURE__ */ new Map();
    for (const typeName of typeNames) {
      const inputType = this.#inputTypeMap.get(typeName);
      if (!inputType) continue;
      for (const field of inputType.fields) {
        let fieldsForName = fieldsByName.get(field.name);
        if (!fieldsForName) {
          fieldsForName = [];
          fieldsByName.set(field.name, fieldsForName);
        }
        fieldsForName.push(field);
      }
    }
    const mergedEdges = {};
    let hasAnyEdge = false;
    for (const [fieldName, variantFields] of fieldsByName) {
      const mergedEdge = this.#mergeFieldVariants(variantFields);
      if (mergedEdge) {
        const stringIndex = this.#builder.internString(fieldName);
        mergedEdges[stringIndex] = mergedEdge;
        hasAnyEdge = true;
      }
    }
    if (hasAnyEdge) {
      this.#builder.setInputNodeEdges(nodeId, mergedEdges);
    }
  }
  /**
   * Merges field variants to produce a single edge descriptor.
   * This is the most complex part of the traversal - it handles
   * union types and determines what kinds of values a field accepts.
   */
  #mergeFieldVariants(variants) {
    let flags = 0;
    let scalarMask = 0;
    let childNodeId;
    let enumNameIndex;
    const scalarTypes = [];
    const enumTypes = [];
    const inputObjectTypes = [];
    for (const variant of variants) {
      for (const inputType of variant.inputTypes) {
        switch (inputType.location) {
          case "scalar":
            if (variant.isParameterizable) {
              scalarTypes.push(inputType);
            }
            break;
          case "enumTypes":
            if (variant.isParameterizable) {
              enumTypes.push(inputType);
            }
            break;
          case "inputObjectTypes":
            if (!inputObjectTypes.some(
              (ot) => ot.type === inputType.type && ot.namespace === inputType.namespace && ot.isList === inputType.isList
            )) {
              inputObjectTypes.push(inputType);
            }
            break;
          case "fieldRefTypes":
            break;
          default:
            throw new Error(`Invalid location ${inputType.location}`);
        }
      }
    }
    for (const st of scalarTypes) {
      scalarMask |= scalarTypeToMask(st.type);
      if (st.isList) {
        flags |= EdgeFlag.ParamListScalar;
      } else {
        flags |= EdgeFlag.ParamScalar;
      }
    }
    for (const et of enumTypes) {
      if (et.namespace === "model") {
        enumNameIndex = this.#builder.internString(et.type);
        if (et.isList) {
          flags |= EdgeFlag.ParamListEnum;
        } else {
          flags |= EdgeFlag.ParamEnum;
        }
        break;
      }
    }
    if (inputObjectTypes.length > 0) {
      const hasObjectList = inputObjectTypes.some((iot) => iot.isList);
      const hasSingleObject = inputObjectTypes.some((iot) => !iot.isList);
      if (hasObjectList) {
        flags |= EdgeFlag.ListObject;
      }
      if (hasSingleObject) {
        flags |= EdgeFlag.Object;
      }
      if (inputObjectTypes.length === 1) {
        childNodeId = this.buildInputTypeNode(getTypeName(inputObjectTypes[0].type, inputObjectTypes[0].namespace));
      } else {
        childNodeId = this.buildUnionNode(inputObjectTypes.map((iot) => getTypeName(iot.type, iot.namespace)));
      }
    }
    if (flags === 0) {
      return void 0;
    }
    const edge = { flags };
    if (childNodeId !== void 0) {
      edge.childNodeId = childNodeId;
    }
    if (scalarMask !== 0) {
      edge.scalarMask = scalarMask;
    }
    if (enumNameIndex !== void 0) {
      edge.enumNameIndex = enumNameIndex;
    }
    return edge;
  }
  /**
   * Builds an output node for a named output type.
   */
  buildOutputTypeNode(typeName) {
    if (this.#builder.hasOutputTypeNode(typeName)) {
      return this.#builder.getOutputTypeNode(typeName);
    }
    const outputType = this.#outputTypeMap.get(typeName);
    if (!outputType) {
      this.#builder.setOutputTypeNode(typeName, void 0);
      return void 0;
    }
    const nodeId = this.#builder.allocateOutputNode();
    this.#builder.setOutputTypeNode(typeName, nodeId);
    const edges = {};
    let hasAnyEdge = false;
    for (const field of outputType.fields) {
      const edge = this.#buildOutputEdge(field);
      if (edge) {
        const stringIndex = this.#builder.internString(field.name);
        edges[stringIndex] = edge;
        hasAnyEdge = true;
      }
    }
    if (hasAnyEdge) {
      this.#builder.setOutputNodeEdges(nodeId, edges);
    }
    return nodeId;
  }
  #buildOutputEdge(field) {
    let argsNodeId;
    let outputNodeId;
    if (field.args.length > 0) {
      argsNodeId = this.buildInputNodeFromArgs(field.args);
    }
    if (field.outputType.location === "outputObjectTypes") {
      outputNodeId = this.buildOutputTypeNode(getTypeName(field.outputType.type, field.outputType.namespace));
    }
    if (argsNodeId === void 0 && outputNodeId === void 0) {
      return void 0;
    }
    const edge = {};
    if (argsNodeId !== void 0) {
      edge.argsNodeId = argsNodeId;
    }
    if (outputNodeId !== void 0) {
      edge.outputNodeId = outputNodeId;
    }
    return edge;
  }
};
function getTypeName(name, namespace2) {
  if (namespace2 === void 0) {
    return name;
  }
  return `${namespace2}.${name}`;
}

// ../param-graph-builder/src/param-graph-builder.ts
var ParamGraphBuilder = class {
  #stringTable = [];
  #stringToIndex = /* @__PURE__ */ new Map();
  #inputNodes = [];
  #outputNodes = [];
  #roots = {};
  #inputTypeNodeCache = /* @__PURE__ */ new Map();
  #unionNodeCache = /* @__PURE__ */ new Map();
  #outputTypeNodeCache = /* @__PURE__ */ new Map();
  /**
   * Interns a string into the string table, returning its index.
   * Both field names and enum names go through this method.
   */
  internString(str) {
    let index = this.#stringToIndex.get(str);
    if (index === void 0) {
      index = this.#stringTable.length;
      this.#stringTable.push(str);
      this.#stringToIndex.set(str, index);
    }
    return index;
  }
  /**
   * Allocates a new input node and returns its ID.
   */
  allocateInputNode() {
    const id = this.#inputNodes.length;
    this.#inputNodes.push({ edges: {} });
    return id;
  }
  /**
   * Sets edges on an input node.
   */
  setInputNodeEdges(nodeId, edges) {
    if (Object.keys(edges).length > 0) {
      this.#inputNodes[nodeId].edges = edges;
    }
  }
  /**
   * Allocates a new output node and returns its ID.
   */
  allocateOutputNode() {
    const id = this.#outputNodes.length;
    this.#outputNodes.push({ edges: {} });
    return id;
  }
  /**
   * Sets edges on an output node.
   */
  setOutputNodeEdges(nodeId, edges) {
    if (Object.keys(edges).length > 0) {
      this.#outputNodes[nodeId].edges = edges;
    }
  }
  /**
   * Records a root entry for an operation.
   */
  setRoot(key, entry) {
    if (entry.argsNodeId !== void 0 || entry.outputNodeId !== void 0) {
      this.internString(key);
      this.#roots[key] = entry;
    }
  }
  // Cache methods for input type nodes
  getInputTypeNode(typeName) {
    return this.#inputTypeNodeCache.get(typeName);
  }
  setInputTypeNode(typeName, nodeId) {
    this.#inputTypeNodeCache.set(typeName, nodeId);
  }
  hasInputTypeNode(typeName) {
    return this.#inputTypeNodeCache.has(typeName);
  }
  // Cache methods for union nodes
  getUnionNode(key) {
    return this.#unionNodeCache.get(key);
  }
  setUnionNode(key, nodeId) {
    this.#unionNodeCache.set(key, nodeId);
  }
  hasUnionNode(key) {
    return this.#unionNodeCache.has(key);
  }
  // Cache methods for output type nodes
  getOutputTypeNode(typeName) {
    return this.#outputTypeNodeCache.get(typeName);
  }
  setOutputTypeNode(typeName, nodeId) {
    this.#outputTypeNodeCache.set(typeName, nodeId);
  }
  hasOutputTypeNode(typeName) {
    return this.#outputTypeNodeCache.has(typeName);
  }
  /**
   * Builds the final ParamGraphData structure.
   */
  build() {
    return {
      strings: this.#stringTable,
      inputNodes: this.#inputNodes,
      outputNodes: this.#outputNodes,
      roots: this.#roots
    };
  }
  /**
   * Builds and serializes to the compact binary format.
   */
  buildAndSerialize() {
    return serializeParamGraph(this.build());
  }
};

// ../param-graph-builder/src/build-param-graph.ts
function buildAndSerializeParamGraph(dmmf) {
  const builder = new ParamGraphBuilder();
  const traverser = new DMMFTraverser(builder, dmmf);
  traverser.processRoots(dmmf.mappings.modelOperations);
  return builder.buildAndSerialize();
}

// ../client-generator-js/src/TSClient/TSClient.ts
var import_indent_string7 = __toESM(require_indent_string());

// ../client-generator-js/src/dmmf.ts
var DMMFHelper = class {
  constructor(document) {
    this.document = document;
  }
  _compositeNames;
  _inputTypesByName;
  _typeAndModelMap;
  _mappingsMap;
  _outputTypeMap;
  _rootFieldMap;
  get compositeNames() {
    return this._compositeNames ??= new Set(this.datamodel.types.map((t) => t.name));
  }
  get inputTypesByName() {
    return this._inputTypesByName ??= this.buildInputTypesMap();
  }
  get typeAndModelMap() {
    return this._typeAndModelMap ??= this.buildTypeModelMap();
  }
  get mappingsMap() {
    return this._mappingsMap ??= this.buildMappingsMap();
  }
  get outputTypeMap() {
    return this._outputTypeMap ??= this.buildMergedOutputTypeMap();
  }
  get rootFieldMap() {
    return this._rootFieldMap ??= this.buildRootFieldMap();
  }
  get datamodel() {
    return this.document.datamodel;
  }
  get mappings() {
    return this.document.mappings;
  }
  get schema() {
    return this.document.schema;
  }
  get inputObjectTypes() {
    return this.schema.inputObjectTypes;
  }
  get outputObjectTypes() {
    return this.schema.outputObjectTypes;
  }
  isComposite(modelOrTypeName) {
    return this.compositeNames.has(modelOrTypeName);
  }
  getOtherOperationNames() {
    return [
      Object.values(this.mappings.otherOperations.write),
      Object.values(this.mappings.otherOperations.read)
    ].flat();
  }
  hasEnumInNamespace(enumName, namespace2) {
    return this.schema.enumTypes[namespace2]?.find((schemaEnum) => schemaEnum.name === enumName) !== void 0;
  }
  resolveInputObjectType(ref) {
    return this.inputTypesByName.get(fullyQualifiedName(ref.type, ref.namespace));
  }
  resolveOutputObjectType(ref) {
    if (ref.location !== "outputObjectTypes") {
      return void 0;
    }
    return this.outputObjectTypes[ref.namespace ?? "prisma"].find((outputObject) => outputObject.name === ref.type);
  }
  buildModelMap() {
    return keyBy(this.datamodel.models, "name");
  }
  buildTypeMap() {
    return keyBy(this.datamodel.types, "name");
  }
  buildTypeModelMap() {
    return { ...this.buildTypeMap(), ...this.buildModelMap() };
  }
  buildMappingsMap() {
    return keyBy(this.mappings.modelOperations, "model");
  }
  buildMergedOutputTypeMap() {
    if (!this.schema.outputObjectTypes.prisma) {
      return {
        model: keyBy(this.schema.outputObjectTypes.model, "name"),
        prisma: keyBy([], "name")
      };
    }
    return {
      model: keyBy(this.schema.outputObjectTypes.model, "name"),
      prisma: keyBy(this.schema.outputObjectTypes.prisma, "name")
    };
  }
  buildRootFieldMap() {
    return {
      ...keyBy(this.outputTypeMap.prisma.Query.fields, "name"),
      ...keyBy(this.outputTypeMap.prisma.Mutation.fields, "name")
    };
  }
  buildInputTypesMap() {
    const result = /* @__PURE__ */ new Map();
    for (const type of this.inputObjectTypes.prisma ?? []) {
      result.set(fullyQualifiedName(type.name, "prisma"), type);
    }
    if (!this.inputObjectTypes.model) {
      return result;
    }
    for (const type of this.inputObjectTypes.model) {
      result.set(fullyQualifiedName(type.name, "model"), type);
    }
    return result;
  }
};
function fullyQualifiedName(typeName, namespace2) {
  if (namespace2) {
    return `${namespace2}.${typeName}`;
  }
  return typeName;
}

// ../client-generator-js/src/GenericsArgsInfo.ts
var GenericArgsInfo = class {
  constructor(_dmmf) {
    this._dmmf = _dmmf;
  }
  _cache = new Cache();
  /**
   * Determines if arg types need generic <$PrismaModel> argument added.
   * Essentially, performs breadth-first search for any fieldRefTypes that
   * do not have corresponding `meta.source` defined.
   *
   * @param type
   * @returns
   */
  typeNeedsGenericModelArg(topLevelType) {
    return this._cache.getOrCreate(topLevelType, () => {
      const toVisit = [{ type: topLevelType }];
      const visited = /* @__PURE__ */ new Set();
      let item2;
      while (item2 = toVisit.shift()) {
        const { type: currentType } = item2;
        const cached = this._cache.get(currentType);
        if (cached === true) {
          this._cacheResultsForTree(item2);
          return true;
        }
        if (cached === false) {
          continue;
        }
        if (visited.has(currentType)) {
          continue;
        }
        if (currentType.meta?.source) {
          this._cache.set(currentType, false);
          continue;
        }
        visited.add(currentType);
        for (const field of currentType.fields) {
          for (const fieldType of field.inputTypes) {
            if (fieldType.location === "fieldRefTypes") {
              this._cacheResultsForTree(item2);
              return true;
            }
            const inputObject = this._dmmf.resolveInputObjectType(fieldType);
            if (inputObject) {
              toVisit.push({ type: inputObject, parent: item2 });
            }
          }
        }
      }
      for (const visitedType of visited) {
        this._cache.set(visitedType, false);
      }
      return false;
    });
  }
  typeRefNeedsGenericModelArg(ref) {
    if (ref.location === "fieldRefTypes") {
      return true;
    }
    const inputType = this._dmmf.resolveInputObjectType(ref);
    if (!inputType) {
      return false;
    }
    return this.typeNeedsGenericModelArg(inputType);
  }
  _cacheResultsForTree(item2) {
    let currentItem = item2;
    while (currentItem) {
      this._cache.set(currentItem.type, true);
      currentItem = currentItem.parent;
    }
  }
};

// ../client-generator-js/src/utils/buildDebugInitialization.ts
function buildDebugInitialization(edge) {
  if (!edge) {
    return "";
  }
  const debugVar = `typeof globalThis !== 'undefined' && globalThis['DEBUG'] || (typeof process !== 'undefined' && process.env && process.env.DEBUG) || undefined`;
  return `if (${debugVar}) {
  Debug.enable(${debugVar})
}
`;
}

// ../client-generator-js/src/utils/buildDMMF.ts
function buildRuntimeDataModel(datamodel, runtimeName) {
  const runtimeDataModel = dmmfToRuntimeDataModel(datamodel);
  let prunedDataModel;
  if (runtimeName === "wasm-compiler-edge" || runtimeName === "client") {
    prunedDataModel = pruneRuntimeDataModel(runtimeDataModel);
  } else {
    prunedDataModel = runtimeDataModel;
  }
  const datamodelString = escapeJson(JSON.stringify(prunedDataModel));
  return `
config.runtimeDataModel = JSON.parse(${JSON.stringify(datamodelString)})
defineDmmfProperty(exports.Prisma, config.runtimeDataModel)`;
}

// ../client-generator-js/src/utils/buildGetQueryCompilerWasmModule.ts
function buildQueryCompilerWasmModule(forceEdgeWasmLoader, runtimeName, compilerBuild) {
  const artifactName = `query_compiler_${compilerBuild}_bg`;
  if (runtimeName === "client" && !forceEdgeWasmLoader) {
    return `config.compilerWasm = {
      getRuntime: async () => require('./${artifactName}.js'),
      getQueryCompilerWasmModule: async () => {
        const { Buffer } = require('node:buffer')
        const { wasm } = require('./${artifactName}.wasm-base64.js')
        const queryCompilerWasmFileBytes = Buffer.from(wasm, 'base64')

        return new WebAssembly.Module(queryCompilerWasmFileBytes)
      },
      importName: './${artifactName}.js',
    }`;
  }
  if (runtimeName === "client" && forceEdgeWasmLoader || runtimeName === "wasm-compiler-edge") {
    return `config.compilerWasm = {
  getRuntime: async () => require('./${artifactName}.js'),
  getQueryCompilerWasmModule: async () => {
    const loader = (await import('#wasm-compiler-loader')).default
    const compiler = (await loader).default
    return compiler
  },
  importName: './${artifactName}.js',
}`;
  }
  return `config.compilerWasm = undefined`;
}

// ../client-generator-js/src/utils/buildRequirePath.ts
function buildRequirePath(edge) {
  if (edge === true) return "";
  return `
  const path = require('path')`;
}

// ../client-generator-js/src/TSClient/common.ts
var import_indent_string4 = __toESM(require_indent_string());
var commonCodeJS = ({
  runtimeBase,
  runtimeName,
  browser,
  clientVersion,
  engineVersion,
  generator
}) => `
Object.defineProperty(exports, "__esModule", { value: true });
${browser ? `
const {
  Decimal,
  DbNull,
  JsonNull,
  AnyNull,
  NullTypes,
  makeStrictEnum,
  Public,
  getRuntime,
  skip
} = require('${runtimeBase}/${runtimeName}.js')
` : `
const {
  PrismaClientKnownRequestError,
  PrismaClientUnknownRequestError,
  PrismaClientRustPanicError,
  PrismaClientInitializationError,
  PrismaClientValidationError,
  getPrismaClient,
  sqltag,
  empty,
  join,
  raw,
  skip,
  Decimal,
  Debug,
  DbNull,
  JsonNull,
  AnyNull,
  NullTypes,
  makeStrictEnum,
  Extensions,
  warnOnce,
  defineDmmfProperty,
  Public,
  getRuntime,
  createParam,
} = require('${runtimeBase}/${runtimeName}.js')
`}

const Prisma = {}

exports.Prisma = Prisma
exports.$Enums = {}

/**
 * Prisma Client JS version: ${clientVersion}
 * Query Engine version: ${engineVersion}
 */
Prisma.prismaVersion = {
  client: "${clientVersion}",
  engine: "${engineVersion}"
}

Prisma.PrismaClientKnownRequestError = ${notSupportOnBrowser("PrismaClientKnownRequestError", browser)};
Prisma.PrismaClientUnknownRequestError = ${notSupportOnBrowser("PrismaClientUnknownRequestError", browser)}
Prisma.PrismaClientRustPanicError = ${notSupportOnBrowser("PrismaClientRustPanicError", browser)}
Prisma.PrismaClientInitializationError = ${notSupportOnBrowser("PrismaClientInitializationError", browser)}
Prisma.PrismaClientValidationError = ${notSupportOnBrowser("PrismaClientValidationError", browser)}
Prisma.Decimal = Decimal

/**
 * Re-export of sql-template-tag
 */
Prisma.sql = ${notSupportOnBrowser("sqltag", browser)}
Prisma.empty = ${notSupportOnBrowser("empty", browser)}
Prisma.join = ${notSupportOnBrowser("join", browser)}
Prisma.raw = ${notSupportOnBrowser("raw", browser)}
Prisma.validator = Public.validator

/**
* Extensions
*/
Prisma.getExtensionContext = ${notSupportOnBrowser("Extensions.getExtensionContext", browser)}
Prisma.defineExtension = ${notSupportOnBrowser("Extensions.defineExtension", browser)}

/**
 * Shorthand utilities for JSON filtering
 */
Prisma.DbNull = DbNull
Prisma.JsonNull = JsonNull
Prisma.AnyNull = AnyNull

Prisma.NullTypes = NullTypes

${buildPrismaSkipJs(generator.previewFeatures)}
`;
var notSupportOnBrowser = (fnc, browser) => {
  if (browser) {
    return `() => {
  const runtimeName = getRuntime().prettyName;
  throw new Error(\`${fnc} is unable to run in this browser environment, or has been bundled for the browser (running in \${runtimeName}).
In case this error is unexpected for you, please report it in https://pris.ly/prisma-prisma-bug-report\`,
)}`;
  }
  return fnc;
};
var commonCodeTS = ({
  runtimeBase,
  runtimeName,
  clientVersion,
  engineVersion,
  generator
}) => ({
  tsWithoutNamespace: () => `import * as runtime from '${runtimeBase}/${runtimeName}.js';
import $Types = runtime.Types // general types
import $Public = runtime.Types.Public
import $Utils = runtime.Types.Utils
import $Extensions = runtime.Types.Extensions
import $Result = runtime.Types.Result

export type PrismaPromise<T> = $Public.PrismaPromise<T>
`,
  ts: () => `export import DMMF = runtime.DMMF

export type PrismaPromise<T> = $Public.PrismaPromise<T>

/**
 * Validator
 */
export import validator = runtime.Public.validator

/**
 * Prisma Errors
 */
export import PrismaClientKnownRequestError = runtime.PrismaClientKnownRequestError
export import PrismaClientUnknownRequestError = runtime.PrismaClientUnknownRequestError
export import PrismaClientRustPanicError = runtime.PrismaClientRustPanicError
export import PrismaClientInitializationError = runtime.PrismaClientInitializationError
export import PrismaClientValidationError = runtime.PrismaClientValidationError

/**
 * Re-export of sql-template-tag
 */
export import sql = runtime.sqltag
export import empty = runtime.empty
export import join = runtime.join
export import raw = runtime.raw
export import Sql = runtime.Sql

${buildPrismaSkipTs(generator.previewFeatures)}

/**
 * Decimal.js
 */
export import Decimal = runtime.Decimal

export type DecimalJsLike = runtime.DecimalJsLike

/**
* Extensions
*/
export import Extension = $Extensions.UserArgs
export import getExtensionContext = runtime.Extensions.getExtensionContext
export import Args = $Public.Args
export import Payload = $Public.Payload
export import Result = $Public.Result
export import Exact = $Public.Exact

/**
 * Prisma Client JS version: ${clientVersion}
 * Query Engine version: ${engineVersion}
 */
export type PrismaVersion = {
  client: string
  engine: string
}

export const prismaVersion: PrismaVersion

/**
 * Utility Types
 */


export import Bytes = runtime.Bytes
export import JsonObject = runtime.JsonObject
export import JsonArray = runtime.JsonArray
export import JsonValue = runtime.JsonValue
export import InputJsonObject = runtime.InputJsonObject
export import InputJsonArray = runtime.InputJsonArray
export import InputJsonValue = runtime.InputJsonValue

/**
 * Types of the values used to represent different kinds of \`null\` values when working with JSON fields.
 *
 * @see https://www.prisma.io/docs/concepts/components/prisma-client/working-with-fields/working-with-json-fields#filtering-on-a-json-field
 */
namespace NullTypes {
${buildNullClass("DbNull")}

${buildNullClass("JsonNull")}

${buildNullClass("AnyNull")}
}

/**
 * Helper for filtering JSON entries that have \`null\` on the database (empty on the db)
 *
 * @see https://www.prisma.io/docs/concepts/components/prisma-client/working-with-fields/working-with-json-fields#filtering-on-a-json-field
 */
export const DbNull: NullTypes.DbNull

/**
 * Helper for filtering JSON entries that have JSON \`null\` values (not empty on the db)
 *
 * @see https://www.prisma.io/docs/concepts/components/prisma-client/working-with-fields/working-with-json-fields#filtering-on-a-json-field
 */
export const JsonNull: NullTypes.JsonNull

/**
 * Helper for filtering JSON entries that are \`Prisma.DbNull\` or \`Prisma.JsonNull\`
 *
 * @see https://www.prisma.io/docs/concepts/components/prisma-client/working-with-fields/working-with-json-fields#filtering-on-a-json-field
 */
export const AnyNull: NullTypes.AnyNull

type SelectAndInclude = {
  select: any
  include: any
}

type SelectAndOmit = {
  select: any
  omit: any
}

/**
 * Get the type of the value, that the Promise holds.
 */
export type PromiseType<T extends PromiseLike<any>> = T extends PromiseLike<infer U> ? U : T;

/**
 * Get the return type of a function which returns a Promise.
 */
export type PromiseReturnType<T extends (...args: any) => $Utils.JsPromise<any>> = PromiseType<ReturnType<T>>

/**
 * From T, pick a set of properties whose keys are in the union K
 */
type Prisma__Pick<T, K extends keyof T> = {
    [P in K]: T[P];
};


export type Enumerable<T> = T | Array<T>;

export type RequiredKeys<T> = {
  [K in keyof T]-?: {} extends Prisma__Pick<T, K> ? never : K
}[keyof T]

export type TruthyKeys<T> = keyof {
  [K in keyof T as T[K] extends false | undefined | null ? never : K]: K
}

export type TrueKeys<T> = TruthyKeys<Prisma__Pick<T, RequiredKeys<T>>>

/**
 * Subset
 * @desc From \`T\` pick properties that exist in \`U\`. Simple version of Intersection
 */
export type Subset<T, U> = {
  [key in keyof T]: key extends keyof U ? T[key] : never;
};

/**
 * SelectSubset
 * @desc From \`T\` pick properties that exist in \`U\`. Simple version of Intersection.
 * Additionally, it validates, if both select and include are present. If the case, it errors.
 */
export type SelectSubset<T, U> = {
  [key in keyof T]: key extends keyof U ? T[key] : never
} &
  (T extends SelectAndInclude
    ? 'Please either choose \`select\` or \`include\`.'
    : T extends SelectAndOmit
      ? 'Please either choose \`select\` or \`omit\`.'
      : {})

/**
 * Subset + Intersection
 * @desc From \`T\` pick properties that exist in \`U\` and intersect \`K\`
 */
export type SubsetIntersection<T, U, K> = {
  [key in keyof T]: key extends keyof U ? T[key] : never
} &
  K

type Without<T, U> = { [P in Exclude<keyof T, keyof U>]?: never };

/**
 * XOR is needed to have a real mutually exclusive union type
 * https://stackoverflow.com/questions/42123407/does-typescript-support-mutually-exclusive-types
 */
type XOR<T, U> =
  T extends object ?
  U extends object ?
    (Without<T, U> & U) | (Without<U, T> & T)
  : U : T


/**
 * Is T a Record?
 */
type IsObject<T extends any> = T extends Array<any>
? False
: T extends Date
? False
: T extends Uint8Array
? False
: T extends BigInt
? False
: T extends object
? True
: False


/**
 * If it's T[], return T
 */
export type UnEnumerate<T extends unknown> = T extends Array<infer U> ? U : T

/**
 * From ts-toolbelt
 */

type __Either<O extends object, K extends Key> = Omit<O, K> &
  {
    // Merge all but K
    [P in K]: Prisma__Pick<O, P & keyof O> // With K possibilities
  }[K]

type EitherStrict<O extends object, K extends Key> = Strict<__Either<O, K>>

type EitherLoose<O extends object, K extends Key> = ComputeRaw<__Either<O, K>>

type _Either<
  O extends object,
  K extends Key,
  strict extends Boolean
> = {
  1: EitherStrict<O, K>
  0: EitherLoose<O, K>
}[strict]

type Either<
  O extends object,
  K extends Key,
  strict extends Boolean = 1
> = O extends unknown ? _Either<O, K, strict> : never

export type Union = any

type PatchUndefined<O extends object, O1 extends object> = {
  [K in keyof O]: O[K] extends undefined ? At<O1, K> : O[K]
} & {}

/** Helper Types for "Merge" **/
export type IntersectOf<U extends Union> = (
  U extends unknown ? (k: U) => void : never
) extends (k: infer I) => void
  ? I
  : never

export type Overwrite<O extends object, O1 extends object> = {
    [K in keyof O]: K extends keyof O1 ? O1[K] : O[K];
} & {};

type _Merge<U extends object> = IntersectOf<Overwrite<U, {
    [K in keyof U]-?: At<U, K>;
}>>;

type Key = string | number | symbol;
type AtBasic<O extends object, K extends Key> = K extends keyof O ? O[K] : never;
type AtStrict<O extends object, K extends Key> = O[K & keyof O];
type AtLoose<O extends object, K extends Key> = O extends unknown ? AtStrict<O, K> : never;
export type At<O extends object, K extends Key, strict extends Boolean = 1> = {
    1: AtStrict<O, K>;
    0: AtLoose<O, K>;
}[strict];

export type ComputeRaw<A extends any> = A extends Function ? A : {
  [K in keyof A]: A[K];
} & {};

export type OptionalFlat<O> = {
  [K in keyof O]?: O[K];
} & {};

type _Record<K extends keyof any, T> = {
  [P in K]: T;
};

// cause typescript not to expand types and preserve names
type NoExpand<T> = T extends unknown ? T : never;

// this type assumes the passed object is entirely optional
type AtLeast<O extends object, K extends string> = NoExpand<
  O extends unknown
  ? | (K extends keyof O ? { [P in K]: O[P] } & O : O)
    | {[P in keyof O as P extends K ? P : never]-?: O[P]} & O
  : never>;

type _Strict<U, _U = U> = U extends unknown ? U & OptionalFlat<_Record<Exclude<Keys<_U>, keyof U>, never>> : never;

export type Strict<U extends object> = ComputeRaw<_Strict<U>>;
/** End Helper Types for "Merge" **/

export type Merge<U extends object> = ComputeRaw<_Merge<Strict<U>>>;

/**
A [[Boolean]]
*/
export type Boolean = True | False

// /**
// 1
// */
export type True = 1

/**
0
*/
export type False = 0

export type Not<B extends Boolean> = {
  0: 1
  1: 0
}[B]

export type Extends<A1 extends any, A2 extends any> = [A1] extends [never]
  ? 0 // anything \`never\` is false
  : A1 extends A2
  ? 1
  : 0

export type Has<U extends Union, U1 extends Union> = Not<
  Extends<Exclude<U1, U>, U1>
>

export type Or<B1 extends Boolean, B2 extends Boolean> = {
  0: {
    0: 0
    1: 1
  }
  1: {
    0: 1
    1: 1
  }
}[B1][B2]

export type Keys<U extends Union> = U extends unknown ? keyof U : never

type Cast<A, B> = A extends B ? A : B;

export const type: unique symbol;



/**
 * Used by group by
 */

export type GetScalarType<T, O> = O extends object ? {
  [P in keyof T]: P extends keyof O
    ? O[P]
    : never
} : never

type FieldPaths<
  T,
  U = Omit<T, '_avg' | '_sum' | '_count' | '_min' | '_max'>
> = IsObject<T> extends True ? U : T

type GetHavingFields<T> = {
  [K in keyof T]: Or<
    Or<Extends<'OR', K>, Extends<'AND', K>>,
    Extends<'NOT', K>
  > extends True
    ? // infer is only needed to not hit TS limit
      // based on the brilliant idea of Pierre-Antoine Mills
      // https://github.com/microsoft/TypeScript/issues/30188#issuecomment-478938437
      T[K] extends infer TK
      ? GetHavingFields<UnEnumerate<TK> extends object ? Merge<UnEnumerate<TK>> : never>
      : never
    : {} extends FieldPaths<T[K]>
    ? never
    : K
}[keyof T]

/**
 * Convert tuple to union
 */
type _TupleToUnion<T> = T extends (infer E)[] ? E : never
type TupleToUnion<K extends readonly any[]> = _TupleToUnion<K>
type MaybeTupleToUnion<T> = T extends any[] ? TupleToUnion<T> : T

/**
 * Like \`Pick\`, but additionally can also accept an array of keys
 */
type PickEnumerable<T, K extends Enumerable<keyof T> | keyof T> = Prisma__Pick<T, MaybeTupleToUnion<K>>

/**
 * Exclude all keys with underscores
 */
type ExcludeUnderscoreKeys<T extends string> = T extends \`_\${string}\` ? never : T


export type FieldRef<Model, FieldType> = runtime.FieldRef<Model, FieldType>

type FieldRefInputType<Model, FieldType> = Model extends never ? never : FieldRef<Model, FieldType>

`
});
function buildNullClass(name) {
  const source = `/**
* Type of \`Prisma.${name}\`.
*
* You cannot use other instances of this class. Please use the \`Prisma.${name}\` value.
*
* @see https://www.prisma.io/docs/concepts/components/prisma-client/working-with-fields/working-with-json-fields#filtering-on-a-json-field
*/
class ${name} {
  private ${name}: never
  private constructor()
}`;
  return (0, import_indent_string4.default)(source, TAB_SIZE);
}
function buildPrismaSkipTs(previewFeatures) {
  if (previewFeatures.includes("strictUndefinedChecks")) {
    return `
/**
 * Prisma.skip
 */
export import skip = runtime.skip
`;
  }
  return "";
}
function buildPrismaSkipJs(previewFeatures) {
  if (previewFeatures.includes("strictUndefinedChecks")) {
    return `
Prisma.skip = skip
`;
  }
  return "";
}

// ../client-generator-js/src/TSClient/Count.ts
var import_indent_string5 = __toESM(require_indent_string());
var Count = class {
  constructor(type, context) {
    this.type = type;
    this.context = context;
  }
  get argsTypes() {
    const argsTypes = [];
    argsTypes.push(
      new ArgsTypeBuilder(this.type, this.context).addSelectArg().addIncludeArgIfHasRelations().createExport()
    );
    for (const field of this.type.fields) {
      if (field.args.length > 0) {
        argsTypes.push(
          new ArgsTypeBuilder(this.type, this.context).addSchemaArgs(field.args).setGeneratedName(getCountArgsType(this.type.name, field.name)).createExport()
        );
      }
    }
    return argsTypes;
  }
  toTS() {
    const { type } = this;
    const { name } = type;
    const outputType = buildOutputType(type);
    return `
/**
 * Count Type ${name}
 */

${stringify(outputType)}

export type ${getSelectName(name)}<ExtArgs extends $Extensions.InternalArgs = $Extensions.DefaultArgs> = {
${(0, import_indent_string5.default)(
      type.fields.map((field) => {
        const types = ["boolean"];
        if (field.outputType.location === "outputObjectTypes") {
          types.push(getFieldArgName(field, this.type.name));
        }
        if (field.args.length > 0) {
          types.push(getCountArgsType(name, field.name));
        }
        return `${field.name}?: ${types.join(" | ")}`;
      }).join("\n"),
      TAB_SIZE
    )}
}

// Custom InputTypes
${this.argsTypes.map((typeExport) => stringify(typeExport)).join("\n\n")}
`;
  }
};
function getCountArgsType(typeName, fieldName) {
  return `${typeName}Count${capitalize(fieldName)}Args`;
}

// ../client-generator-js/src/TSClient/FieldRefInput.ts
var FieldRefInput = class {
  constructor(type) {
    this.type = type;
  }
  toTS() {
    const allowedTypes = this.getAllowedTypes();
    return `
/**
 * Reference to a field of type ${allowedTypes}
 */
export type ${this.type.name}<$PrismaModel> = FieldRefInputType<$PrismaModel, ${allowedTypes}>
    `;
  }
  getAllowedTypes() {
    return this.type.allowTypes.map(getRefAllowedTypeName).join(" | ");
  }
};

// ../client-generator-js/src/TSClient/GenerateContext.ts
var GenerateContext = class {
  dmmf;
  genericArgsInfo;
  generator;
  provider;
  constructor({ dmmf, genericArgsInfo, generator, provider }) {
    this.dmmf = dmmf;
    this.genericArgsInfo = genericArgsInfo;
    this.generator = generator;
    this.provider = provider;
  }
  isPreviewFeatureOn(previewFeature) {
    return this.generator?.previewFeatures?.includes(previewFeature) ?? false;
  }
  isSqlProvider() {
    return this.provider !== "mongodb";
  }
};

// ../client-generator-js/src/TSClient/PrismaClient.ts
var import_indent_string6 = __toESM(require_indent_string());

// ../client-generator-js/src/utils/runtimeImport.ts
function runtimeImportedType(name) {
  return namedType(`runtime.${name}`);
}

// ../client-generator-js/src/TSClient/globalOmit.ts
function globalOmitConfig(dmmf) {
  const objectType2 = objectType().addMultiple(
    dmmf.datamodel.models.map((model) => {
      const type = namedType(getOmitName(model.name));
      return property(uncapitalize(model.name), type).optional();
    })
  );
  return moduleExport(typeDeclaration("GlobalOmitConfig", objectType2));
}

// ../client-generator-js/src/TSClient/PrismaClient.ts
function clientTypeMapModelsDefinition(context) {
  const meta = objectType();
  const modelNames = context.dmmf.datamodel.models.map((m) => m.name);
  if (modelNames.length === 0) {
    meta.add(property("modelProps", neverType));
  } else {
    meta.add(property("modelProps", unionType(modelNames.map((name) => stringLiteral(uncapitalize(name))))));
  }
  const isolationLevel = context.dmmf.hasEnumInNamespace("TransactionIsolationLevel", "prisma") ? namedType("Prisma.TransactionIsolationLevel") : neverType;
  meta.add(property("txIsolationLevel", isolationLevel));
  const model = objectType();
  model.addMultiple(
    modelNames.map((modelName) => {
      const entry = objectType();
      entry.add(
        property("payload", namedType(getPayloadName(modelName)).addGenericArgument(extArgsParam.toArgument()))
      );
      entry.add(property("fields", namedType(`Prisma.${getFieldRefsTypeName(modelName)}`)));
      const actions = getModelActions(context.dmmf, modelName);
      const operations = objectType();
      operations.addMultiple(
        actions.map((action2) => {
          const operationType = objectType();
          const argsType = `Prisma.${getModelArgName(modelName, action2)}`;
          operationType.add(property("args", namedType(argsType).addGenericArgument(extArgsParam.toArgument())));
          operationType.add(property("result", clientTypeMapModelsResultDefinition(modelName, action2)));
          return property(action2, operationType);
        })
      );
      entry.add(property("operations", operations));
      return property(modelName, entry);
    })
  );
  return objectType().add(property("globalOmitOptions", objectType().add(property("omit", namedType("GlobalOmitOptions"))))).add(property("meta", meta)).add(property("model", model));
}
function clientTypeMapModelsResultDefinition(modelName, action2) {
  if (action2 === "count")
    return unionType([optional(namedType(getCountAggregateOutputName(modelName))), numberType]);
  if (action2 === "groupBy") return array(optional(namedType(getGroupByName(modelName))));
  if (action2 === "aggregate") return optional(namedType(getAggregateName(modelName)));
  if (action2 === "findRaw") return namedType("JsonObject");
  if (action2 === "aggregateRaw") return namedType("JsonObject");
  if (action2 === "deleteMany") return namedType("BatchPayload");
  if (action2 === "createMany") return namedType("BatchPayload");
  if (action2 === "createManyAndReturn") return array(payloadToResult(modelName));
  if (action2 === "updateMany") return namedType("BatchPayload");
  if (action2 === "updateManyAndReturn") return array(payloadToResult(modelName));
  if (action2 === "findMany") return array(payloadToResult(modelName));
  if (action2 === "findFirst") return unionType([payloadToResult(modelName), nullType]);
  if (action2 === "findUnique") return unionType([payloadToResult(modelName), nullType]);
  if (action2 === "findFirstOrThrow") return payloadToResult(modelName);
  if (action2 === "findUniqueOrThrow") return payloadToResult(modelName);
  if (action2 === "create") return payloadToResult(modelName);
  if (action2 === "update") return payloadToResult(modelName);
  if (action2 === "upsert") return payloadToResult(modelName);
  if (action2 === "delete") return payloadToResult(modelName);
  assertNever(action2, `Unknown action: ${action2}`);
}
function payloadToResult(modelName) {
  return namedType("$Utils.PayloadToResult").addGenericArgument(namedType(getPayloadName(modelName)));
}
function clientTypeMapOthersDefinition(context) {
  const otherOperationsNames = context.dmmf.getOtherOperationNames().flatMap((name) => {
    const results = [`$${name}`];
    if (name === "executeRaw" || name === "queryRaw") {
      results.push(`$${name}Unsafe`);
    }
    if (name === "queryRaw" && context.isPreviewFeatureOn("typedSql")) {
      results.push(`$queryRawTyped`);
    }
    return results;
  });
  const argsResultMap = {
    $executeRaw: { args: "[query: TemplateStringsArray | Prisma.Sql, ...values: any[]]", result: "any" },
    $queryRaw: { args: "[query: TemplateStringsArray | Prisma.Sql, ...values: any[]]", result: "any" },
    $executeRawUnsafe: { args: "[query: string, ...values: any[]]", result: "any" },
    $queryRawUnsafe: { args: "[query: string, ...values: any[]]", result: "any" },
    $runCommandRaw: { args: "Prisma.InputJsonObject", result: "Prisma.JsonObject" },
    $queryRawTyped: { args: "runtime.UnknownTypedSql", result: "Prisma.JsonObject" }
  };
  return `{
  other: {
    payload: any
    operations: {${otherOperationsNames.reduce((acc, action2) => {
    return `${acc}
      ${action2}: {
        args: ${argsResultMap[action2].args},
        result: ${argsResultMap[action2].result}
      }`;
  }, "")}
    }
  }
}`;
}
function clientTypeMapDefinition(context) {
  const typeMap = `${stringify(clientTypeMapModelsDefinition(context))} & ${clientTypeMapOthersDefinition(context)}`;
  return `
interface TypeMapCb<ClientOptions = {}> extends $Utils.Fn<{extArgs: $Extensions.InternalArgs }, $Utils.Record<string, any>> {
  returns: Prisma.TypeMap<this['params']['extArgs'], ClientOptions extends { omit: infer OmitOptions } ? OmitOptions : {}>
}

export type TypeMap<ExtArgs extends $Extensions.InternalArgs = $Extensions.DefaultArgs, GlobalOmitOptions = {}> = ${typeMap}`;
}
function clientExtensionsDefinitions(context) {
  const typeMap = clientTypeMapDefinition(context);
  const define2 = moduleExport(
    constDeclaration(
      "defineExtension",
      namedType("$Extensions.ExtendsHook").addGenericArgument(stringLiteral("define")).addGenericArgument(namedType("Prisma.TypeMapCb")).addGenericArgument(namedType("$Extensions.DefaultArgs"))
    )
  );
  return [typeMap, stringify(define2)].join("\n");
}
function extendsPropertyDefinition() {
  const extendsDefinition = namedType("$Extensions.ExtendsHook").addGenericArgument(stringLiteral("extends")).addGenericArgument(namedType("Prisma.TypeMapCb").addGenericArgument(namedType("ClientOptions"))).addGenericArgument(namedType("ExtArgs")).addGenericArgument(
    namedType("$Utils.Call").addGenericArgument(namedType("Prisma.TypeMapCb").addGenericArgument(namedType("ClientOptions"))).addGenericArgument(objectType().add(property("extArgs", namedType("ExtArgs"))))
  );
  return stringify(property("$extends", extendsDefinition), { indentLevel: 1 });
}
function batchingTransactionDefinition(context) {
  const method2 = method("$transaction").setDocComment(
    docComment`
        Allows the running of a sequence of read/write operations that are guaranteed to either succeed or fail as a whole.
        @example
        \`\`\`
        const [george, bob, alice] = await prisma.$transaction([
          prisma.user.create({ data: { name: 'George' } }),
          prisma.user.create({ data: { name: 'Bob' } }),
          prisma.user.create({ data: { name: 'Alice' } }),
        ])
        \`\`\`

        Read more in our [docs](https://www.prisma.io/docs/orm/prisma-client/queries/transactions).
      `
  ).addGenericParameter(genericParameter("P").extends(array(prismaPromise(anyType)))).addParameter(parameter("arg", arraySpread(namedType("P")))).setReturnType(promise(namedType("runtime.Types.Utils.UnwrapTuple").addGenericArgument(namedType("P"))));
  if (context.dmmf.hasEnumInNamespace("TransactionIsolationLevel", "prisma")) {
    const options2 = objectType().formatInline().add(property("isolationLevel", namedType("Prisma.TransactionIsolationLevel")).optional());
    method2.addParameter(parameter("options", options2).optional());
  }
  return stringify(method2, { indentLevel: 1, newLine: "leading" });
}
function interactiveTransactionDefinition(context) {
  const options2 = objectType().formatInline().add(property("maxWait", numberType).optional()).add(property("timeout", numberType).optional());
  if (context.dmmf.hasEnumInNamespace("TransactionIsolationLevel", "prisma")) {
    const isolationLevel = property("isolationLevel", namedType("Prisma.TransactionIsolationLevel")).optional();
    options2.add(isolationLevel);
  }
  const returnType = promise(namedType("R"));
  const callbackType = functionType().addParameter(
    parameter("prisma", omit(namedType("PrismaClient"), namedType("runtime.ITXClientDenyList")))
  ).setReturnType(returnType);
  const method2 = method("$transaction").addGenericParameter(genericParameter("R")).addParameter(parameter("fn", callbackType)).addParameter(parameter("options", options2).optional()).setReturnType(returnType);
  return stringify(method2, { indentLevel: 1, newLine: "leading" });
}
function queryRawDefinition(context) {
  if (!context.dmmf.mappings.otherOperations.write.includes("queryRaw")) {
    return "";
  }
  return `
  /**
   * Performs a prepared raw query and returns the \`SELECT\` data.
   * @example
   * \`\`\`
   * const result = await prisma.$queryRaw\`SELECT * FROM User WHERE id = \${1} OR email = \${'user@email.com'};\`
   * \`\`\`
   *
   * Read more in our [docs](https://pris.ly/d/raw-queries).
   */
  $queryRaw<T = unknown>(query: TemplateStringsArray | Prisma.Sql, ...values: any[]): Prisma.PrismaPromise<T>;

  /**
   * Performs a raw query and returns the \`SELECT\` data.
   * Susceptible to SQL injections, see documentation.
   * @example
   * \`\`\`
   * const result = await prisma.$queryRawUnsafe('SELECT * FROM User WHERE id = $1 OR email = $2;', 1, 'user@email.com')
   * \`\`\`
   *
   * Read more in our [docs](https://pris.ly/d/raw-queries).
   */
  $queryRawUnsafe<T = unknown>(query: string, ...values: any[]): Prisma.PrismaPromise<T>;`;
}
function executeRawDefinition(context) {
  if (!context.dmmf.mappings.otherOperations.write.includes("executeRaw")) {
    return "";
  }
  return `
  /**
   * Executes a prepared raw query and returns the number of affected rows.
   * @example
   * \`\`\`
   * const result = await prisma.$executeRaw\`UPDATE User SET cool = \${true} WHERE email = \${'user@email.com'};\`
   * \`\`\`
   *
   * Read more in our [docs](https://pris.ly/d/raw-queries).
   */
  $executeRaw<T = unknown>(query: TemplateStringsArray | Prisma.Sql, ...values: any[]): Prisma.PrismaPromise<number>;

  /**
   * Executes a raw query and returns the number of affected rows.
   * Susceptible to SQL injections, see documentation.
   * @example
   * \`\`\`
   * const result = await prisma.$executeRawUnsafe('UPDATE User SET cool = $1 WHERE email = $2 ;', true, 'user@email.com')
   * \`\`\`
   *
   * Read more in our [docs](https://pris.ly/d/raw-queries).
   */
  $executeRawUnsafe<T = unknown>(query: string, ...values: any[]): Prisma.PrismaPromise<number>;`;
}
function queryRawTypedDefinition(context) {
  if (!context.isPreviewFeatureOn("typedSql")) {
    return "";
  }
  if (!context.dmmf.mappings.otherOperations.write.includes("queryRaw")) {
    return "";
  }
  const param = genericParameter("T");
  const method2 = method("$queryRawTyped").setDocComment(
    docComment`
        Executes a typed SQL query and returns a typed result
        @example
        \`\`\`
        import { myQuery } from '@prisma/client/sql'

        const result = await prisma.$queryRawTyped(myQuery())
        \`\`\`
      `
  ).addGenericParameter(param).addParameter(
    parameter(
      "typedSql",
      runtimeImportedType("TypedSql").addGenericArgument(array(unknownType)).addGenericArgument(param.toArgument())
    )
  ).setReturnType(prismaPromise(array(param.toArgument())));
  return stringify(method2, { indentLevel: 1, newLine: "leading" });
}
function runCommandRawDefinition(context) {
  if (!context.dmmf.mappings.otherOperations.write.includes("runCommandRaw")) {
    return "";
  }
  const method2 = method("$runCommandRaw").addParameter(parameter("command", namedType("Prisma.InputJsonObject"))).setReturnType(prismaPromise(namedType("Prisma.JsonObject"))).setDocComment(docComment`
      Executes a raw MongoDB command and returns the result of it.
      @example
      \`\`\`
      const user = await prisma.$runCommandRaw({
        aggregate: 'User',
        pipeline: [{ $match: { name: 'Bob' } }, { $project: { email: true, _id: false } }],
        explain: false,
      })
      \`\`\`

      Read more in our [docs](https://pris.ly/d/raw-queries).
    `);
  return stringify(method2, { indentLevel: 1, newLine: "leading" });
}
var PrismaClientClass = class {
  constructor(context, internalDatasources, outputDir, runtimeName, browser) {
    this.context = context;
    this.internalDatasources = internalDatasources;
    this.outputDir = outputDir;
    this.runtimeName = runtimeName;
    this.browser = browser;
  }
  get jsDoc() {
    const { dmmf } = this.context;
    let example;
    if (dmmf.mappings.modelOperations.length) {
      example = dmmf.mappings.modelOperations[0];
    } else {
      example = {
        model: "User",
        plural: "users"
      };
    }
    return `/**
 * ##  Prisma Client \u02B2\u02E2
 *
 * Type-safe database client for TypeScript & Node.js
 * @example
 * \`\`\`
 * const prisma = new PrismaClient({
 *   adapter: new PrismaPg({ connectionString: process.env.DATABASE_URL })
 * })
 * // Fetch zero or more ${capitalize(example.plural)}
 * const ${uncapitalize(example.plural)} = await prisma.${uncapitalize(example.model)}.findMany()
 * \`\`\`
 *
 *
 * Read more in our [docs](https://pris.ly/d/client).
 */`;
  }
  toTSWithoutNamespace() {
    const { dmmf } = this.context;
    return `${this.jsDoc}
export class PrismaClient<
  ClientOptions extends Prisma.PrismaClientOptions = Prisma.PrismaClientOptions,
  const U = 'log' extends keyof ClientOptions ? ClientOptions['log'] extends Array<Prisma.LogLevel | Prisma.LogDefinition> ? Prisma.GetEvents<ClientOptions['log']> : never : never,
  ExtArgs extends $Extensions.InternalArgs = $Extensions.DefaultArgs
> {
  [K: symbol]: { types: Prisma.TypeMap<ExtArgs>['other'] }

  ${(0, import_indent_string6.default)(this.jsDoc, TAB_SIZE)}

  constructor(optionsArg ?: Prisma.Subset<ClientOptions, Prisma.PrismaClientOptions>);
  $on<V extends U>(eventType: V, callback: (event: V extends 'query' ? Prisma.QueryEvent : Prisma.LogEvent) => void): PrismaClient;

  /**
   * Connect with the database
   */
  $connect(): $Utils.JsPromise<void>;

  /**
   * Disconnect from the database
   */
  $disconnect(): $Utils.JsPromise<void>;

${[
      executeRawDefinition(this.context),
      queryRawDefinition(this.context),
      queryRawTypedDefinition(this.context),
      batchingTransactionDefinition(this.context),
      interactiveTransactionDefinition(this.context),
      runCommandRawDefinition(this.context),
      extendsPropertyDefinition()
    ].filter((d) => d !== null).join("\n").trim()}

    ${(0, import_indent_string6.default)(
      dmmf.mappings.modelOperations.filter((m) => m.findMany).map((m) => {
        let methodName = uncapitalize(m.model);
        if (methodName === "constructor") {
          methodName = '["constructor"]';
        }
        const generics = ["ExtArgs", "ClientOptions"];
        return `/**
 * \`prisma.${methodName}\`: Exposes CRUD operations for the **${m.model}** model.
  * Example usage:
  * \`\`\`ts
  * // Fetch zero or more ${capitalize(m.plural)}
  * const ${uncapitalize(m.plural)} = await prisma.${methodName}.findMany()
  * \`\`\`
  */
get ${methodName}(): Prisma.${m.model}Delegate<${generics.join(", ")}>;`;
      }).join("\n\n"),
      2
    )}
}`;
  }
  toTS() {
    const clientOptions = this.buildClientOptions();
    return `${clientExtensionsDefinitions(this.context)}
export type DefaultPrismaClient = PrismaClient
export type ErrorFormat = 'pretty' | 'colorless' | 'minimal'
${stringify(moduleExport(clientOptions))}
${stringify(globalOmitConfig(this.context.dmmf))}

/* Types for Logging */
export type LogLevel = 'info' | 'query' | 'warn' | 'error'
export type LogDefinition = {
  level: LogLevel
  emit: 'stdout' | 'event'
}

export type CheckIsLogLevel<T> = T extends LogLevel ? T : never;

export type GetLogType<T> = CheckIsLogLevel<
  T extends LogDefinition ? T['level'] : T
>;

export type GetEvents<T extends any[]> = T extends Array<LogLevel | LogDefinition>
  ? GetLogType<T[number]>
  : never;

export type QueryEvent = {
  timestamp: Date
  query: string
  params: string
  duration: number
  target: string
}

export type LogEvent = {
  timestamp: Date
  message: string
  target: string
}
/* End Types for Logging */


export type PrismaAction =
  | 'findUnique'
  | 'findUniqueOrThrow'
  | 'findMany'
  | 'findFirst'
  | 'findFirstOrThrow'
  | 'create'
  | 'createMany'
  | 'createManyAndReturn'
  | 'update'
  | 'updateMany'
  | 'updateManyAndReturn'
  | 'upsert'
  | 'delete'
  | 'deleteMany'
  | 'executeRaw'
  | 'queryRaw'
  | 'aggregate'
  | 'count'
  | 'runCommandRaw'
  | 'findRaw'
  | 'groupBy'

// tested in getLogLevel.test.ts
export function getLogLevel(log: Array<LogLevel | LogDefinition>): LogLevel | undefined;

/**
 * \`PrismaClient\` proxy available in interactive transactions.
 */
export type TransactionClient = Omit<Prisma.DefaultPrismaClient, runtime.ITXClientDenyList>
`;
  }
  buildClientOptions() {
    const clientOptions = interfaceDeclaration("PrismaClientOptions").add(
      property("errorFormat", namedType("ErrorFormat")).optional().setDocComment(docComment('@default "colorless"'))
    ).add(
      property("log", array(unionType([namedType("LogLevel"), namedType("LogDefinition")]))).optional().setDocComment(docComment`
             @example
             \`\`\`
             // Shorthand for \`emit: 'stdout'\`
             log: ['query', 'info', 'warn', 'error']

             // Emit as events only
             log: [
               { emit: 'event', level: 'query' },
               { emit: 'event', level: 'info' },
               { emit: 'event', level: 'warn' }
               { emit: 'event', level: 'error' }
             ]

            // Emit as events and log to stdout
            log: [
              { emit: 'stdout', level: 'query' },
              { emit: 'stdout', level: 'info' },
              { emit: 'stdout', level: 'warn' }
              { emit: 'stdout', level: 'error' }
            ]
             \`\`\`
             Read more in our [docs](https://pris.ly/d/logging).
          `)
    );
    const transactionOptions = objectType().add(property("maxWait", numberType).optional()).add(property("timeout", numberType).optional());
    if (this.context.dmmf.hasEnumInNamespace("TransactionIsolationLevel", "prisma")) {
      transactionOptions.add(property("isolationLevel", namedType("Prisma.TransactionIsolationLevel")).optional());
    }
    clientOptions.add(
      property("transactionOptions", transactionOptions).optional().setDocComment(docComment`
             The default values for transactionOptions
             maxWait ?= 2000
             timeout ?= 5000
          `)
    );
    if (
      // We don't support a custom adapter with MongoDB for now.
      this.internalDatasources.some((d) => d.provider !== "mongodb")
    ) {
      clientOptions.add(
        property("adapter", namedType("runtime.SqlDriverAdapterFactory")).optional().setDocComment(
          docComment("Instance of a Driver Adapter, e.g., like one provided by `@prisma/adapter-planetscale`")
        )
      );
    }
    clientOptions.add(
      property("accelerateUrl", stringType).optional().setDocComment(
        docComment(
          "Prisma Accelerate URL allowing the client to connect through Accelerate instead of a direct database."
        )
      )
    );
    clientOptions.add(
      property("omit", namedType("Prisma.GlobalOmitConfig")).optional().setDocComment(docComment`
        Global configuration for omitting model fields by default.

        @example
        \`\`\`
        const prisma = new PrismaClient({
          omit: {
            user: {
              password: true
            }
          }
        })
        \`\`\`
      `)
    );
    if (this.context.isSqlProvider()) {
      clientOptions.add(
        property("comments", array(namedType("runtime.SqlCommenterPlugin"))).optional().setDocComment(docComment`
            SQL commenter plugins that add metadata to SQL queries as comments.
            Comments follow the sqlcommenter format: https://google.github.io/sqlcommenter/

            @example
            \`\`\`
            const prisma = new PrismaClient({
              adapter,
              comments: [
                traceContext(),
                queryInsights(),
              ],
            })
            \`\`\`
          `)
      );
    }
    return clientOptions;
  }
};

// ../client-generator-js/src/TSClient/TSClient.ts
var TSClient = class {
  constructor(options2) {
    this.options = options2;
    this.dmmf = new DMMFHelper(options2.dmmf);
    this.genericsInfo = new GenericArgsInfo(this.dmmf);
  }
  dmmf;
  genericsInfo;
  buildParamGraphConfig() {
    const serialized = buildAndSerializeParamGraph(this.options.dmmf);
    const stringsJson = JSON.stringify(JSON.stringify(serialized.strings));
    return `config.parameterizationSchema = {
  strings: JSON.parse(${stringsJson}),
  graph: "${serialized.graph}"
}`;
  }
  toJS() {
    const { edge, wasm, generator, datamodel: inlineSchema, runtimeName, reusedJs, compilerBuild } = this.options;
    if (reusedJs) {
      return `module.exports = { ...require('${reusedJs}') }`;
    }
    const config = {
      previewFeatures: generator.previewFeatures,
      clientVersion: this.options.clientVersion,
      engineVersion: this.options.engineVersion,
      activeProvider: this.options.activeProvider,
      inlineSchema
    };
    const code = `${commonCodeJS({ ...this.options, browser: false })}
${buildRequirePath(edge)}

/**
 * Enums
 */
${this.dmmf.schema.enumTypes.prisma?.map((type) => new Enum(type, true).toJS()).join("\n\n")}
${this.dmmf.datamodel.enums.map((datamodelEnum) => new Enum(datamodelEnumToSchemaEnum(datamodelEnum), false).toJS()).join("\n\n")}

${new Enum(
      {
        name: "ModelName",
        values: this.dmmf.mappings.modelOperations.map((m) => m.model)
      },
      true
    ).toJS()}
/**
 * Create the Client
 */
const config = ${JSON.stringify(config, null, 2)}
${buildRuntimeDataModel(this.dmmf.datamodel, runtimeName)}
${this.buildParamGraphConfig()}
${buildQueryCompilerWasmModule(wasm, runtimeName, compilerBuild)}
${buildDebugInitialization(edge)}
const PrismaClient = getPrismaClient(config)
exports.PrismaClient = PrismaClient
Object.assign(exports, Prisma)
`;
    return code;
  }
  toTS() {
    const { reusedTs } = this.options;
    if (reusedTs) {
      const topExports = moduleExportFrom(`./${reusedTs}`);
      return stringify(topExports);
    }
    const context = new GenerateContext({
      dmmf: this.dmmf,
      genericArgsInfo: this.genericsInfo,
      generator: this.options.generator,
      provider: this.options.activeProvider
    });
    const prismaClientClass = new PrismaClientClass(
      context,
      this.options.datasources,
      this.options.outputDir,
      this.options.runtimeName,
      this.options.browser
    );
    const commonCode = commonCodeTS(this.options);
    const modelAndTypes = Object.values(this.dmmf.typeAndModelMap).reduce((acc, modelOrType) => {
      if (this.dmmf.outputTypeMap.model[modelOrType.name]) {
        acc.push(new Model(modelOrType, context));
      }
      return acc;
    }, []);
    const prismaEnums = this.dmmf.schema.enumTypes.prisma?.map((type) => new Enum(type, true).toTS());
    const modelEnums = [];
    const modelEnumsAliases = [];
    for (const datamodelEnum of this.dmmf.datamodel.enums) {
      modelEnums.push(new Enum(datamodelEnumToSchemaEnum(datamodelEnum), false).toTS());
      modelEnumsAliases.push(
        stringify(
          moduleExport(typeDeclaration(datamodelEnum.name, namedType(`$Enums.${datamodelEnum.name}`)))
        ),
        stringify(
          moduleExport(constDeclaration(datamodelEnum.name, namedType(`typeof $Enums.${datamodelEnum.name}`)))
        )
      );
    }
    const fieldRefs = this.dmmf.schema.fieldRefTypes.prisma?.map((type) => new FieldRefInput(type).toTS()) ?? [];
    const countTypes = this.dmmf.schema.outputObjectTypes.prisma?.filter((t) => t.name.endsWith("CountOutputType")).map((t) => new Count(t, context));
    const code = `
/**
 * Client
**/

${commonCode.tsWithoutNamespace()}

${modelAndTypes.map((m) => m.toTSWithoutNamespace()).join("\n")}
${modelEnums.length > 0 ? `
/**
 * Enums
 */
export namespace $Enums {
  ${modelEnums.join("\n\n")}
}

${modelEnumsAliases.join("\n\n")}
` : ""}
${prismaClientClass.toTSWithoutNamespace()}

export namespace Prisma {
${(0, import_indent_string7.default)(
      `${commonCode.ts()}
${new Enum(
        {
          name: "ModelName",
          values: this.dmmf.mappings.modelOperations.map((m) => m.model)
        },
        true
      ).toTS()}

${prismaClientClass.toTS()}
export type Datasource = {
  url?: string
}

/**
 * Count Types
 */

${countTypes.map((t) => t.toTS()).join("\n")}

/**
 * Models
 */
${modelAndTypes.map((model) => model.toTS()).join("\n")}

/**
 * Enums
 */

${prismaEnums?.join("\n\n")}
${fieldRefs.length > 0 ? `
/**
 * Field references
 */

${fieldRefs.join("\n\n")}` : ""}
/**
 * Deep Input Types
 */

${this.dmmf.inputObjectTypes.prisma?.reduce((acc, inputType) => {
        if (inputType.name.includes("Json") && inputType.name.includes("Filter")) {
          const needsGeneric = this.genericsInfo.typeNeedsGenericModelArg(inputType);
          const innerName = needsGeneric ? `${inputType.name}Base<$PrismaModel>` : `${inputType.name}Base`;
          const typeName = needsGeneric ? `${inputType.name}<$PrismaModel = never>` : inputType.name;
          const baseName = `Required<${innerName}>`;
          acc.push(`export type ${typeName} =
  | PatchUndefined<
      Either<${baseName}, Exclude<keyof ${baseName}, 'path'>>,
      ${baseName}
    >
  | OptionalFlat<Omit<${baseName}, 'path'>>`);
          acc.push(new InputType(inputType, context).overrideName(`${inputType.name}Base`).toTS());
        } else {
          acc.push(new InputType(inputType, context).toTS());
        }
        return acc;
      }, []).join("\n")}

${this.dmmf.inputObjectTypes.model?.map((inputType) => new InputType(inputType, context).toTS()).join("\n") ?? ""}

/**
 * Batch Payload for updateMany & deleteMany & createMany
 */

export type BatchPayload = {
  count: number
}

/**
 * DMMF
 */
export const dmmf: runtime.BaseDMMF
`,
      2
    )}}`;
    return code;
  }
  toBrowserJS() {
    const code = `${commonCodeJS({
      ...this.options,
      runtimeName: "index-browser",
      browser: true
    })}
/**
 * Enums
 */

${this.dmmf.schema.enumTypes.prisma?.map((type) => new Enum(type, true).toJS()).join("\n\n")}
${this.dmmf.schema.enumTypes.model?.map((type) => new Enum(type, false).toJS()).join("\n\n") ?? ""}

${new Enum(
      {
        name: "ModelName",
        values: this.dmmf.mappings.modelOperations.map((m) => m.model)
      },
      true
    ).toJS()}

/**
 * This is a stub Prisma Client that will error at runtime if called.
 */
class PrismaClient {
  constructor() {
    return new Proxy(this, {
      get(target, prop) {
        let message
        const runtime = getRuntime()
        if (runtime.isEdge) {
          message = \`PrismaClient is not configured to run in \${runtime.prettyName}. In order to run Prisma Client on edge runtime, either:
- Use Prisma Accelerate: https://pris.ly/d/accelerate
- Use Driver Adapters: https://pris.ly/d/driver-adapters
\`;
        } else {
          message = 'PrismaClient is unable to run in this browser environment, or has been bundled for the browser (running in \`' + runtime.prettyName + '\`).'
        }

        message += \`
If this is unexpected, please open an issue: https://pris.ly/prisma-prisma-bug-report\`

        throw new Error(message)
      }
    })
  }
}

exports.PrismaClient = PrismaClient

Object.assign(exports, Prisma)
`;
    return code;
  }
};

// ../client-generator-js/src/typedSql/buildDbEnums.ts
var DbEnumsList = class {
  enums;
  constructor(enums) {
    this.enums = enums.map((dmmfEnum) => ({
      name: dmmfEnum.dbName ?? dmmfEnum.name,
      values: dmmfEnum.values.map((dmmfValue) => dmmfValue.dbName ?? dmmfValue.name)
    }));
  }
  isEmpty() {
    return this.enums.length === 0;
  }
  hasEnum(name) {
    return Boolean(this.enums.find((dbEnum) => dbEnum.name === name));
  }
  *validJsIdentifiers() {
    for (const dbEnum of this.enums) {
      if (isValidJsIdentifier(dbEnum.name)) {
        yield dbEnum;
      }
    }
  }
  *invalidJsIdentifiers() {
    for (const dbEnum of this.enums) {
      if (!isValidJsIdentifier(dbEnum.name)) {
        yield dbEnum;
      }
    }
  }
};
function buildDbEnums(list) {
  const file2 = file();
  file2.add(buildInvalidIdentifierEnums(list));
  file2.add(buildValidIdentifierEnums(list));
  return stringify(file2);
}
function buildValidIdentifierEnums(list) {
  const namespace2 = namespace("$DbEnums");
  for (const dbEnum of list.validJsIdentifiers()) {
    namespace2.add(typeDeclaration(dbEnum.name, enumToUnion(dbEnum)));
  }
  return moduleExport(namespace2);
}
function buildInvalidIdentifierEnums(list) {
  const iface = interfaceDeclaration("$DbEnums");
  for (const dbEnum of list.invalidJsIdentifiers()) {
    iface.add(property(dbEnum.name, enumToUnion(dbEnum)));
  }
  return moduleExport(iface);
}
function enumToUnion(dbEnum) {
  return unionType(dbEnum.values.map(stringLiteral));
}
function queryUsesEnums(query, enums) {
  if (enums.isEmpty()) {
    return false;
  }
  return query.parameters.some((param) => enums.hasEnum(param.typ)) || query.resultColumns.some((column) => enums.hasEnum(column.typ));
}

// ../client-generator-js/src/typedSql/buildIndex.ts
function buildIndexTs(queries, enums) {
  const file2 = file();
  if (!enums.isEmpty()) {
    file2.add(moduleExportFrom("./$DbEnums").named("$DbEnums"));
  }
  for (const query of queries) {
    file2.add(moduleExportFrom(`./${query.name}`));
  }
  return stringify(file2);
}
function buildIndexCjs(queries, edgeRuntimeSuffix) {
  const writer = new Writer(0, void 0);
  writer.writeLine('"use strict"');
  for (const { name } of queries) {
    const fileName = edgeRuntimeSuffix ? `${name}.${edgeRuntimeSuffix}` : name;
    writer.writeLine(`exports.${name} = require("./${fileName}.js").${name}`);
  }
  return writer.toString();
}
function buildIndexEsm(queries, edgeRuntimeSuffix) {
  const writer = new Writer(0, void 0);
  for (const { name } of queries) {
    const fileName = edgeRuntimeSuffix ? `${name}.${edgeRuntimeSuffix}` : name;
    writer.writeLine(`export * from "./${fileName}.mjs"`);
  }
  return writer.toString();
}

// ../client-generator-js/src/typedSql/mapTypes.ts
var decimal = namedType("$runtime.Decimal");
var uint8Array = namedType("$runtime.Bytes");
var date = namedType("Date");
var inputJsonValue = namedType("$runtime.InputJsonObject");
var jsonValue = namedType("$runtime.JsonValue");
var bigintIn = unionType([numberType, bigintType]);
var decimalIn = unionType([numberType, decimal]);
var typeMappings = {
  unknown: unknownType,
  string: stringType,
  int: numberType,
  bigint: {
    in: bigintIn,
    out: bigintType
  },
  decimal: {
    in: decimalIn,
    out: decimal
  },
  float: numberType,
  double: numberType,
  enum: stringType,
  // TODO:
  bytes: uint8Array,
  bool: booleanType,
  char: stringType,
  json: {
    in: inputJsonValue,
    out: jsonValue
  },
  xml: stringType,
  uuid: stringType,
  date,
  datetime: date,
  time: date,
  null: nullType,
  "int-array": array(numberType),
  "string-array": array(stringType),
  "json-array": {
    in: array(inputJsonValue),
    out: array(jsonValue)
  },
  "uuid-array": array(stringType),
  "xml-array": array(stringType),
  "bigint-array": {
    in: array(bigintIn),
    out: array(bigintType)
  },
  "float-array": array(numberType),
  "double-array": array(numberType),
  "char-array": array(stringType),
  "bytes-array": array(uint8Array),
  "bool-array": array(booleanType),
  "date-array": array(date),
  "time-array": array(date),
  "datetime-array": array(date),
  "decimal-array": {
    in: array(decimalIn),
    out: array(decimal)
  }
};
function getInputType(introspectionType, nullable, enums) {
  const inn = getMappingConfig(introspectionType, enums).in;
  if (!nullable) {
    return inn;
  } else {
    return new UnionType(inn).addVariant(nullType);
  }
}
function getOutputType(introspectionType, nullable, enums) {
  const out = getMappingConfig(introspectionType, enums).out;
  if (!nullable) {
    return out;
  } else {
    return new UnionType(out).addVariant(nullType);
  }
}
function getMappingConfig(introspectionType, enums) {
  const config = typeMappings[introspectionType];
  if (!config) {
    if (enums.hasEnum(introspectionType)) {
      const type = getEnumType(introspectionType);
      return { in: type, out: type };
    }
    throw new Error("Unknown type");
  }
  if (config instanceof TypeBuilder) {
    return { in: config, out: config };
  }
  return config;
}
function getEnumType(name) {
  if (isValidJsIdentifier(name)) {
    return namedType(`$DbEnums.${name}`);
  }
  return namedType("$DbEnums").subKey(name);
}

// ../client-generator-js/src/typedSql/buildTypedQuery.ts
function buildTypedQueryTs({ query, runtimeBase, runtimeName, enums }) {
  const file2 = file();
  file2.addImport(moduleImport(`${runtimeBase}/${runtimeName}`).asNamespace("$runtime"));
  if (queryUsesEnums(query, enums)) {
    file2.addImport(moduleImport("./$DbEnums").named("$DbEnums"));
  }
  const doc = docComment(query.documentation ?? void 0);
  const factoryType = functionType();
  const parametersType = tupleType();
  for (const param of query.parameters) {
    const paramType = getInputType(param.typ, param.nullable, enums);
    factoryType.addParameter(parameter(param.name, paramType));
    parametersType.add(tupleItem(paramType).setName(param.name));
    if (param.documentation) {
      doc.addText(`@param ${param.name} ${param.documentation}`);
    } else {
      doc.addText(`@param ${param.name}`);
    }
  }
  factoryType.setReturnType(
    namedType("$runtime.TypedSql").addGenericArgument(namedType(`${query.name}.Parameters`)).addGenericArgument(namedType(`${query.name}.Result`))
  );
  file2.add(moduleExport(constDeclaration(query.name, factoryType)).setDocComment(doc));
  const namespace2 = namespace(query.name);
  namespace2.add(moduleExport(typeDeclaration("Parameters", parametersType)));
  namespace2.add(buildResultType(query, enums));
  file2.add(moduleExport(namespace2));
  return stringify(file2);
}
function buildResultType(query, enums) {
  const type = objectType().addMultiple(
    query.resultColumns.map((column) => property(column.name, getOutputType(column.typ, column.nullable, enums)))
  );
  return moduleExport(typeDeclaration("Result", type));
}
function buildTypedQueryCjs({ query, runtimeBase, runtimeName }) {
  const writer = new Writer(0, void 0);
  writer.writeLine('"use strict"');
  writer.writeLine(`const { makeTypedQueryFactory: $mkFactory } = require("${runtimeBase}/${runtimeName}")`);
  writer.writeLine(`exports.${query.name} = /*#__PURE__*/ $mkFactory(${JSON.stringify(query.source)})`);
  return writer.toString();
}
function buildTypedQueryEsm({ query, runtimeBase, runtimeName }) {
  const writer = new Writer(0, void 0);
  writer.writeLine(`import { makeTypedQueryFactory as $mkFactory } from "${runtimeBase}/${runtimeName}"`);
  writer.writeLine(`export const ${query.name} = /*#__PURE__*/ $mkFactory(${JSON.stringify(query.source)})`);
  return writer.toString();
}

// ../client-generator-js/src/typedSql/typedSql.ts
function buildTypedSql({
  queries,
  runtimeBase,
  edgeRuntimeName,
  mainRuntimeName,
  dmmf
}) {
  const fileMap = {};
  const enums = new DbEnumsList(dmmf.datamodel.enums);
  if (!enums.isEmpty()) {
    fileMap["$DbEnums.d.ts"] = buildDbEnums(enums);
  }
  for (const query of queries) {
    const options2 = { query, runtimeBase, runtimeName: mainRuntimeName, enums };
    const edgeOptions = { ...options2, runtimeName: `${edgeRuntimeName}.js` };
    fileMap[`${query.name}.d.ts`] = buildTypedQueryTs(options2);
    fileMap[`${query.name}.js`] = buildTypedQueryCjs(options2);
    fileMap[`${query.name}.${edgeRuntimeName}.js`] = buildTypedQueryCjs(edgeOptions);
    fileMap[`${query.name}.mjs`] = buildTypedQueryEsm(options2);
    fileMap[`${query.name}.${edgeRuntimeName}.mjs`] = buildTypedQueryEsm(edgeOptions);
  }
  fileMap["index.d.ts"] = buildIndexTs(queries, enums);
  fileMap["index.js"] = buildIndexCjs(queries);
  fileMap["index.mjs"] = buildIndexEsm(queries);
  fileMap[`index.${edgeRuntimeName}.mjs`] = buildIndexEsm(queries, edgeRuntimeName);
  fileMap[`index.${edgeRuntimeName}.js`] = buildIndexCjs(queries, edgeRuntimeName);
  return fileMap;
}

// ../client-generator-js/src/utils/addPreamble.ts
var generatedCodePreamble = `
/* !!! This is code generated by Prisma. Do not edit directly. !!!
/* eslint-disable */
// biome-ignore-all lint: generated file
`;
function addPreambleToJSFiles(fileMap) {
  for (const [key, value] of Object.entries(fileMap)) {
    if (typeof value === "string" && (key.endsWith(".js") || key.endsWith(".mjs"))) {
      fileMap[key] = addPreamble(value);
    } else if (typeof value === "object" && value !== null) {
      addPreambleToJSFiles(value);
    }
  }
}
function addPreamble(fileContent) {
  return generatedCodePreamble + fileContent;
}

// ../client-generator-js/src/generateClient.ts
var DenylistError = class extends Error {
  constructor(message) {
    super(message);
    this.stack = void 0;
  }
};
setClassName(DenylistError, "DenylistError");
async function buildClient({
  schemaPath,
  runtimeBase,
  runtimeSourcePath,
  datamodel,
  binaryPaths,
  outputDir,
  generator,
  dmmf,
  datasources,
  engineVersion,
  clientVersion,
  activeProvider,
  typedSql,
  compilerBuild
}) {
  const baseClientOptions = {
    dmmf: getPrismaClientDMMF(dmmf),
    datasources,
    generator,
    binaryPaths,
    schemaPath,
    outputDir,
    runtimeBase,
    runtimeSourcePath,
    clientVersion,
    engineVersion,
    activeProvider,
    datamodel,
    compilerBuild,
    browser: false,
    edge: false,
    wasm: false
  };
  const nodeClientOptions = {
    ...baseClientOptions,
    runtimeName: "client"
  };
  const nodeClient = new TSClient(nodeClientOptions);
  const defaultClient = new TSClient({
    ...nodeClientOptions,
    reusedTs: "index",
    reusedJs: "."
  });
  const trampolineTsClient = new TSClient({
    ...nodeClientOptions,
    reusedTs: "index",
    reusedJs: "#main-entry-point"
  });
  const exportsMapBase = {
    node: "./index.js",
    "edge-light": "./edge.js",
    workerd: "./edge.js",
    worker: "./edge.js",
    browser: "./index-browser.js",
    default: "./index.js"
  };
  const exportsMapDefault = {
    require: exportsMapBase,
    import: exportsMapBase,
    default: exportsMapBase.default
  };
  const pkgJson = {
    name: getUniquePackageName(datamodel),
    main: "index.js",
    types: "index.d.ts",
    browser: "index-browser.js",
    // The order of exports is important:
    // * `./client` before `...clientPkg.exports` allows it to have a higher priority than the `./*` export in `clientPkg.exports`
    // * `.` after `...clientPkg.exports` makes it override the `.` export in `clientPkgs.exports`
    exports: {
      "./client": exportsMapDefault,
      ...package_default.exports,
      // TODO: remove on DA ga
      ".": exportsMapDefault
    },
    version: clientVersion,
    sideEffects: false,
    dependencies: {
      "@prisma/client-runtime-utils": clientVersion
    }
  };
  const fileMap = {};
  fileMap["index.js"] = JS(nodeClient);
  fileMap["index.d.ts"] = TS(nodeClient);
  fileMap["default.js"] = JS(defaultClient);
  fileMap["default.d.ts"] = TS(defaultClient);
  fileMap["index-browser.js"] = BrowserJS(nodeClient);
  fileMap["client.js"] = JS(defaultClient);
  fileMap["client.d.ts"] = TS(defaultClient);
  fileMap["default.js"] = JS(trampolineTsClient);
  fileMap["default.d.ts"] = TS(trampolineTsClient);
  const qcArtifactName = `query_compiler_${compilerBuild}_bg`;
  fileMap["wasm-worker-loader.mjs"] = `export default import('./${qcArtifactName}.wasm')`;
  fileMap["wasm-edge-light-loader.mjs"] = `export default import('./${qcArtifactName}.wasm?module')`;
  pkgJson["browser"] = "default.js";
  pkgJson["imports"] = {
    // when `import('#wasm-compiler-loader')` is called, it will be resolved to the correct file
    "#wasm-compiler-loader": {
      // Keys reference: https://runtime-keys.proposal.wintercg.org/#keys
      /**
       * Vercel Edge Functions / Next.js Middlewares
       */
      "edge-light": "./wasm-edge-light-loader.mjs",
      /**
       * Cloudflare Workers, Cloudflare Pages
       */
      workerd: "./wasm-worker-loader.mjs",
      /**
       * (Old) Cloudflare Workers
       * @millsp It's a fallback, in case both other keys didn't work because we could be on a different edge platform. It's a hypothetical case rather than anything actually tested.
       */
      worker: "./wasm-worker-loader.mjs",
      /**
       * Fallback for every other JavaScript runtime
       */
      default: "./wasm-worker-loader.mjs"
    },
    // when `require('#main-entry-point')` is called, it will be resolved to the correct file
    "#main-entry-point": exportsMapDefault
  };
  const wasmClient = new TSClient({
    ...baseClientOptions,
    runtimeName: "wasm-compiler-edge",
    reusedTs: "default",
    edge: true,
    wasm: true
  });
  fileMap["edge.js"] = JS(wasmClient);
  fileMap["edge.d.ts"] = TS(wasmClient);
  if (typedSql && typedSql.length > 0) {
    const edgeRuntimeName = "wasm-compiler-edge";
    const cjsEdgeIndex = `./sql/index.${edgeRuntimeName}.js`;
    const esmEdgeIndex = `./sql/index.${edgeRuntimeName}.mjs`;
    pkgJson.exports["./sql"] = {
      require: {
        types: "./sql/index.d.ts",
        "edge-light": cjsEdgeIndex,
        workerd: cjsEdgeIndex,
        worker: cjsEdgeIndex,
        node: "./sql/index.js",
        default: "./sql/index.js"
      },
      import: {
        types: "./sql/index.d.ts",
        "edge-light": esmEdgeIndex,
        workerd: esmEdgeIndex,
        worker: esmEdgeIndex,
        node: "./sql/index.mjs",
        default: "./sql/index.mjs"
      },
      default: "./sql/index.js"
    };
    fileMap["sql"] = buildTypedSql({
      dmmf,
      runtimeBase: getTypedSqlRuntimeBase(runtimeBase),
      mainRuntimeName: "client",
      queries: typedSql,
      edgeRuntimeName
    });
  }
  fileMap["package.json"] = JSON.stringify(pkgJson, null, 2);
  addPreambleToJSFiles(fileMap);
  return {
    fileMap,
    // a map of file names to their contents
    prismaClientDmmf: dmmf
    // the DMMF document
  };
}
function getTypedSqlRuntimeBase(runtimeBase) {
  if (!runtimeBase.startsWith(".")) {
    return runtimeBase;
  }
  if (runtimeBase.startsWith("./")) {
    return `.${runtimeBase}`;
  }
  return `../${runtimeBase}`;
}
function getDefaultOutdir(outputDir) {
  if (outputDir.endsWith(import_path4.default.normalize("node_modules/@prisma/client"))) {
    return import_path4.default.join(outputDir, "../../.prisma/client");
  }
  return import_path4.default.join(outputDir, "../../.prisma/client");
}
async function generateClient(options2) {
  const {
    datamodel,
    schemaPath,
    generator,
    dmmf,
    datasources,
    binaryPaths,
    testMode,
    copyRuntime,
    copyRuntimeSourceMaps = false,
    runtimeSourcePath,
    clientVersion,
    engineVersion,
    activeProvider,
    typedSql,
    compilerBuild
  } = options2;
  const { runtimeBase, outputDir } = await getGenerationDirs(options2);
  const { prismaClientDmmf, fileMap } = await buildClient({
    datamodel,
    schemaPath,
    runtimeBase,
    runtimeSourcePath,
    outputDir,
    generator,
    dmmf,
    datasources,
    binaryPaths,
    clientVersion,
    engineVersion,
    activeProvider,
    testMode,
    typedSql,
    compilerBuild
  });
  const provider = datasources[0].provider;
  const denylistsErrors = validateDmmfAgainstDenylists(prismaClientDmmf);
  if (denylistsErrors) {
    let message = `${bold(
      red("Error: ")
    )}The schema at "${schemaPath}" contains reserved keywords.
       Rename the following items:`;
    for (const error of denylistsErrors) {
      message += "\n         - " + error.message;
    }
    message += `
To learn more about how to rename models, check out https://pris.ly/d/naming-models`;
    throw new DenylistError(message);
  }
  await (0, import_fs_extra.ensureDir)(outputDir);
  await writeFileMap(outputDir, fileMap);
  if (copyRuntime || generator.isCustomOutput === true) {
    const copiedRuntimeDir = import_path4.default.join(outputDir, "runtime");
    await (0, import_fs_extra.ensureDir)(copiedRuntimeDir);
    await copyRuntimeFiles({
      from: runtimeSourcePath,
      to: copiedRuntimeDir,
      sourceMaps: copyRuntimeSourceMaps,
      runtimeName: "client"
    });
  }
  const schemaTargetPath = import_path4.default.join(outputDir, "schema.prisma");
  await import_promises2.default.writeFile(schemaTargetPath, datamodel, { encoding: "utf-8" });
  if (isWasmEngineSupported(provider)) {
    const suffix = provider === "postgres" ? "postgresql" : provider;
    const filename = `query_compiler_${compilerBuild}_bg`;
    const wasmJsBundlePath = import_path4.default.join(runtimeSourcePath, `${filename}.${suffix}.wasm-base64.js`);
    const wasmBase64 = require(wasmJsBundlePath).wasm;
    await import_promises2.default.writeFile(import_path4.default.join(outputDir, `${filename}.wasm`), Buffer.from(wasmBase64, "base64"));
    await import_promises2.default.copyFile(import_path4.default.join(runtimeSourcePath, `${filename}.${suffix}.js`), import_path4.default.join(outputDir, `${filename}.js`));
    await import_promises2.default.copyFile(wasmJsBundlePath, import_path4.default.join(outputDir, `${filename}.wasm-base64.js`));
  }
  try {
    const prismaCache = (0, import_env_paths.default)("prisma").cache;
    const signalsPath = import_path4.default.join(prismaCache, "last-generate");
    await import_promises2.default.mkdir(prismaCache, { recursive: true });
    await import_promises2.default.writeFile(signalsPath, Date.now().toString());
  } catch {
  }
}
function writeFileMap(outputDir, fileMap) {
  return Promise.all(
    Object.entries(fileMap).map(async ([fileName, content]) => {
      const absolutePath = import_path4.default.join(outputDir, fileName);
      await import_promises2.default.rm(absolutePath, { recursive: true, force: true });
      if (typeof content === "string") {
        await import_promises2.default.writeFile(absolutePath, content);
      } else {
        await import_promises2.default.mkdir(absolutePath);
        await writeFileMap(absolutePath, content);
      }
    })
  );
}
function isWasmEngineSupported(provider) {
  return provider === "postgresql" || provider === "postgres" || provider === "cockroachdb" || provider === "mysql" || provider === "sqlite" || provider === "sqlserver";
}
function validateDmmfAgainstDenylists(prismaClientDmmf) {
  const errorArray = [];
  const denylists = {
    // A copy of this list is also in prisma-engines. Any edit should be done in both places.
    // https://github.com/prisma/prisma-engines/blob/main/psl/parser-database/src/names/reserved_model_names.rs
    models: [
      // Reserved Prisma keywords
      "PrismaClient",
      "Prisma",
      // JavaScript keywords
      "async",
      "await",
      "break",
      "case",
      "catch",
      "class",
      "const",
      "continue",
      "debugger",
      "default",
      "delete",
      "do",
      "else",
      "enum",
      "export",
      "extends",
      "false",
      "finally",
      "for",
      "function",
      "if",
      "implements",
      "import",
      "in",
      "instanceof",
      "interface",
      "let",
      "new",
      "null",
      "package",
      "private",
      "protected",
      "public",
      "return",
      "super",
      "switch",
      "this",
      "throw",
      "true",
      "try",
      "using",
      "typeof",
      "var",
      "void",
      "while",
      "with",
      "yield"
    ],
    fields: ["AND", "OR", "NOT"],
    dynamic: []
  };
  if (prismaClientDmmf.datamodel.enums) {
    for (const it of prismaClientDmmf.datamodel.enums) {
      if (denylists.models.includes(it.name) || denylists.fields.includes(it.name)) {
        errorArray.push(Error(`"enum ${it.name}"`));
      }
    }
  }
  if (prismaClientDmmf.datamodel.models) {
    for (const it of prismaClientDmmf.datamodel.models) {
      if (denylists.models.includes(it.name) || denylists.fields.includes(it.name)) {
        errorArray.push(Error(`"model ${it.name}"`));
      }
    }
  }
  return errorArray.length > 0 ? errorArray : null;
}
async function getGenerationDirs({
  runtimeBase,
  generator,
  outputDir,
  datamodel,
  schemaPath,
  testMode
}) {
  const isCustomOutput = generator.isCustomOutput === true;
  const normalizedOutputDir = import_path4.default.normalize(outputDir);
  let userRuntimeImport = isCustomOutput ? "./runtime" : "@prisma/client/runtime";
  let userOutputDir = isCustomOutput ? normalizedOutputDir : getDefaultOutdir(normalizedOutputDir);
  if (testMode && runtimeBase) {
    userOutputDir = outputDir;
    userRuntimeImport = pathToPosix(runtimeBase);
  }
  if (isCustomOutput) {
    await verifyOutputDirectory(userOutputDir, datamodel, schemaPath);
  }
  const userPackageRoot = await packageUp({ cwd: import_path4.default.dirname(userOutputDir) });
  const userProjectRoot = userPackageRoot ? import_path4.default.dirname(userPackageRoot) : process.cwd();
  return {
    runtimeBase: userRuntimeImport,
    outputDir: userOutputDir,
    projectRoot: userProjectRoot
  };
}
async function verifyOutputDirectory(directory, datamodel, schemaPath) {
  let content;
  try {
    content = await import_promises2.default.readFile(import_path4.default.join(directory, "package.json"), "utf8");
  } catch (e) {
    if (e.code === "ENOENT") {
      return;
    }
    throw e;
  }
  const { name } = JSON.parse(content);
  if (name === package_default.name) {
    const message = [`Generating client into ${bold(directory)} is not allowed.`];
    message.push("This package is used by `prisma generate` and overwriting its content is dangerous.");
    message.push("");
    message.push("Suggestion:");
    const outputDeclaration = findOutputPathDeclaration(datamodel);
    if (outputDeclaration && outputDeclaration.content.includes(package_default.name)) {
      const outputLine = outputDeclaration.content;
      message.push(`In ${bold(schemaPath)} replace:`);
      message.push("");
      message.push(`${dim(outputDeclaration.lineNumber)} ${replacePackageName(outputLine, red(package_default.name))}`);
      message.push("with");
      message.push(`${dim(outputDeclaration.lineNumber)} ${replacePackageName(outputLine, green(".prisma/client"))}`);
    } else {
      message.push(`Generate client into ${bold(replacePackageName(directory, green(".prisma/client")))} instead`);
    }
    message.push("");
    message.push("You won't need to change your imports.");
    message.push("Imports from `@prisma/client` will be automatically forwarded to `.prisma/client`");
    const error = new Error(message.join("\n"));
    throw error;
  }
}
function replacePackageName(directoryPath, replacement) {
  return directoryPath.replace(package_default.name, replacement);
}
function findOutputPathDeclaration(datamodel) {
  const lines2 = datamodel.split(/\r?\n/);
  for (const [i, line] of lines2.entries()) {
    if (/output\s*=/.test(line)) {
      return { lineNumber: i + 1, content: line.trim() };
    }
  }
  return null;
}
async function copyRuntimeFiles({ from, to, runtimeName, sourceMaps }) {
  const files = ["index-browser.js", "index-browser.d.ts", "wasm-compiler-edge.js"];
  files.push(`${runtimeName}.js`);
  files.push(`${runtimeName}.d.ts`);
  if (sourceMaps) {
    files.push(...files.filter((file2) => file2.endsWith(".js")).map((file2) => `${file2}.map`));
  }
  await Promise.all(
    files.map(async (file2) => {
      const sourcePath = import_path4.default.join(from, file2);
      const targetPath = import_path4.default.join(to, file2);
      if (file2.endsWith(".js")) {
        const content = await import_promises2.default.readFile(sourcePath, "utf-8");
        await import_promises2.default.writeFile(targetPath, addPreamble(content));
      } else {
        await import_promises2.default.copyFile(sourcePath, targetPath);
      }
    })
  );
}
function getUniquePackageName(datamodel) {
  const hash = (0, import_crypto.createHash)("sha256");
  hash.write(datamodel);
  return `${GENERATED_PACKAGE_NAME_PREFIX}${hash.digest().toString("hex")}`;
}
var GENERATED_PACKAGE_NAME_PREFIX = "prisma-client-";

// ../client-generator-js/src/generator.ts
var import_node_path4 = __toESM(require("node:path"));
var import_engines_version = __toESM(require_engines_version());

// ../client-generator-js/package.json
var version = "7.4.2";

// ../client-generator-js/src/resolvePrismaClient.ts
var import_promises5 = __toESM(require("node:fs/promises"));
var import_node_path3 = __toESM(require("node:path"));

// ../../node_modules/.pnpm/@antfu+ni@0.21.12/node_modules/@antfu/ni/dist/shared/ni.f699cf8a.mjs
var import_node_fs = __toESM(require("node:fs"), 1);
var import_node_path2 = __toESM(require("node:path"), 1);
var import_node_process3 = __toESM(require("node:process"), 1);
var import_node_buffer = require("node:buffer");
var import_node_child_process = __toESM(require("node:child_process"), 1);
var import_child_process = __toESM(require("child_process"), 1);
var import_path5 = __toESM(require("path"), 1);
var import_fs = __toESM(require("fs"), 1);
var import_node_url2 = __toESM(require("node:url"), 1);
var import_node_os = __toESM(require("node:os"), 1);
var import_promises3 = require("node:timers/promises");
var import_stream = __toESM(require("stream"), 1);
var import_node_util = require("node:util");
var import_os = __toESM(require("os"), 1);
var import_tty = __toESM(require("tty"), 1);
var import_readline = __toESM(require("readline"), 1);
var import_events = __toESM(require("events"), 1);
var import_promises4 = __toESM(require("fs/promises"), 1);
function npmRun(agent) {
  return (args) => {
    if (args.length > 1)
      return `${agent} run ${args[0]} -- ${args.slice(1).join(" ")}`;
    else
      return `${agent} run ${args[0]}`;
  };
}
var yarn = {
  "agent": "yarn {0}",
  "run": "yarn run {0}",
  "install": "yarn install {0}",
  "frozen": "yarn install --frozen-lockfile",
  "global": "yarn global add {0}",
  "add": "yarn add {0}",
  "upgrade": "yarn upgrade {0}",
  "upgrade-interactive": "yarn upgrade-interactive {0}",
  "execute": "npx {0}",
  "uninstall": "yarn remove {0}",
  "global_uninstall": "yarn global remove {0}"
};
var pnpm = {
  "agent": "pnpm {0}",
  "run": "pnpm run {0}",
  "install": "pnpm i {0}",
  "frozen": "pnpm i --frozen-lockfile",
  "global": "pnpm add -g {0}",
  "add": "pnpm add {0}",
  "upgrade": "pnpm update {0}",
  "upgrade-interactive": "pnpm update -i {0}",
  "execute": "pnpm dlx {0}",
  "uninstall": "pnpm remove {0}",
  "global_uninstall": "pnpm remove --global {0}"
};
var bun = {
  "agent": "bun {0}",
  "run": "bun run {0}",
  "install": "bun install {0}",
  "frozen": "bun install --no-save",
  "global": "bun add -g {0}",
  "add": "bun add {0}",
  "upgrade": "bun update {0}",
  "upgrade-interactive": "bun update {0}",
  "execute": "bunx {0}",
  "uninstall": "bun remove {0}",
  "global_uninstall": "bun remove -g {0}"
};
var AGENTS = {
  "npm": {
    "agent": "npm {0}",
    "run": npmRun("npm"),
    "install": "npm i {0}",
    "frozen": "npm ci",
    "global": "npm i -g {0}",
    "add": "npm i {0}",
    "upgrade": "npm update {0}",
    "upgrade-interactive": null,
    "execute": "npx {0}",
    "uninstall": "npm uninstall {0}",
    "global_uninstall": "npm uninstall -g {0}"
  },
  "yarn": yarn,
  "yarn@berry": {
    ...yarn,
    "frozen": "yarn install --immutable",
    "upgrade": "yarn up {0}",
    "upgrade-interactive": "yarn up -i {0}",
    "execute": "yarn dlx {0}",
    // Yarn 2+ removed 'global', see https://github.com/yarnpkg/berry/issues/821
    "global": "npm i -g {0}",
    "global_uninstall": "npm uninstall -g {0}"
  },
  "pnpm": pnpm,
  // pnpm v6.x or below
  "pnpm@6": {
    ...pnpm,
    run: npmRun("pnpm")
  },
  "bun": bun
};
var agents = Object.keys(AGENTS);
var LOCKS = {
  "bun.lockb": "bun",
  "pnpm-lock.yaml": "pnpm",
  "yarn.lock": "yarn",
  "package-lock.json": "npm",
  "npm-shrinkwrap.json": "npm"
};
var INSTALL_PAGE = {
  "bun": "https://bun.sh",
  "pnpm": "https://pnpm.io/installation",
  "pnpm@6": "https://pnpm.io/6.x/installation",
  "yarn": "https://classic.yarnpkg.com/en/docs/install",
  "yarn@berry": "https://yarnpkg.com/getting-started/install",
  "npm": "https://docs.npmjs.com/cli/v8/configuring-npm/install"
};
var commonjsGlobal = typeof globalThis !== "undefined" ? globalThis : typeof window !== "undefined" ? window : typeof global !== "undefined" ? global : typeof self !== "undefined" ? self : {};
function getDefaultExportFromCjs(x) {
  return x && x.__esModule && Object.prototype.hasOwnProperty.call(x, "default") ? x["default"] : x;
}
var { hasOwnProperty: hasOwnProperty2 } = Object.prototype;
var crossSpawn$1 = { exports: {} };
var windows;
var hasRequiredWindows;
function requireWindows() {
  if (hasRequiredWindows) return windows;
  hasRequiredWindows = 1;
  windows = isexe2;
  isexe2.sync = sync2;
  var fs4 = import_fs.default;
  function checkPathExt2(path10, options2) {
    var pathext = options2.pathExt !== void 0 ? options2.pathExt : process.env.PATHEXT;
    if (!pathext) {
      return true;
    }
    pathext = pathext.split(";");
    if (pathext.indexOf("") !== -1) {
      return true;
    }
    for (var i = 0; i < pathext.length; i++) {
      var p = pathext[i].toLowerCase();
      if (p && path10.substr(-p.length).toLowerCase() === p) {
        return true;
      }
    }
    return false;
  }
  function checkStat2(stat, path10, options2) {
    if (!stat.isSymbolicLink() && !stat.isFile()) {
      return false;
    }
    return checkPathExt2(path10, options2);
  }
  function isexe2(path10, options2, cb) {
    fs4.stat(path10, function(er, stat) {
      cb(er, er ? false : checkStat2(stat, path10, options2));
    });
  }
  function sync2(path10, options2) {
    return checkStat2(fs4.statSync(path10), path10, options2);
  }
  return windows;
}
var mode;
var hasRequiredMode;
function requireMode() {
  if (hasRequiredMode) return mode;
  hasRequiredMode = 1;
  mode = isexe2;
  isexe2.sync = sync2;
  var fs4 = import_fs.default;
  function isexe2(path10, options2, cb) {
    fs4.stat(path10, function(er, stat) {
      cb(er, er ? false : checkStat2(stat, options2));
    });
  }
  function sync2(path10, options2) {
    return checkStat2(fs4.statSync(path10), options2);
  }
  function checkStat2(stat, options2) {
    return stat.isFile() && checkMode2(stat, options2);
  }
  function checkMode2(stat, options2) {
    var mod = stat.mode;
    var uid = stat.uid;
    var gid = stat.gid;
    var myUid = options2.uid !== void 0 ? options2.uid : process.getuid && process.getuid();
    var myGid = options2.gid !== void 0 ? options2.gid : process.getgid && process.getgid();
    var u = parseInt("100", 8);
    var g = parseInt("010", 8);
    var o = parseInt("001", 8);
    var ug = u | g;
    var ret = mod & o || mod & g && gid === myGid || mod & u && uid === myUid || mod & ug && myUid === 0;
    return ret;
  }
  return mode;
}
var core;
if (process.platform === "win32" || commonjsGlobal.TESTING_WINDOWS) {
  core = requireWindows();
} else {
  core = requireMode();
}
var isexe_1 = isexe$4;
isexe$4.sync = sync$2;
function isexe$4(path10, options2, cb) {
  if (typeof options2 === "function") {
    cb = options2;
    options2 = {};
  }
  if (!cb) {
    if (typeof Promise !== "function") {
      throw new TypeError("callback not provided");
    }
    return new Promise(function(resolve3, reject) {
      isexe$4(path10, options2 || {}, function(er, is) {
        if (er) {
          reject(er);
        } else {
          resolve3(is);
        }
      });
    });
  }
  core(path10, options2 || {}, function(er, is) {
    if (er) {
      if (er.code === "EACCES" || options2 && options2.ignoreErrors) {
        er = null;
        is = false;
      }
    }
    cb(er, is);
  });
}
function sync$2(path10, options2) {
  try {
    return core.sync(path10, options2 || {});
  } catch (er) {
    if (options2 && options2.ignoreErrors || er.code === "EACCES") {
      return false;
    } else {
      throw er;
    }
  }
}
var isWindows$1 = process.platform === "win32" || process.env.OSTYPE === "cygwin" || process.env.OSTYPE === "msys";
var path$2 = import_path5.default;
var COLON = isWindows$1 ? ";" : ":";
var isexe$3 = isexe_1;
var getNotFoundError$1 = (cmd) => Object.assign(new Error(`not found: ${cmd}`), { code: "ENOENT" });
var getPathInfo$1 = (cmd, opt) => {
  const colon = opt.colon || COLON;
  const pathEnv = cmd.match(/\//) || isWindows$1 && cmd.match(/\\/) ? [""] : [
    // windows always checks the cwd first
    ...isWindows$1 ? [process.cwd()] : [],
    ...(opt.path || process.env.PATH || /* istanbul ignore next: very unusual */
    "").split(colon)
  ];
  const pathExtExe = isWindows$1 ? opt.pathExt || process.env.PATHEXT || ".EXE;.CMD;.BAT;.COM" : "";
  const pathExt = isWindows$1 ? pathExtExe.split(colon) : [""];
  if (isWindows$1) {
    if (cmd.indexOf(".") !== -1 && pathExt[0] !== "")
      pathExt.unshift("");
  }
  return {
    pathEnv,
    pathExt,
    pathExtExe
  };
};
var which$3 = (cmd, opt, cb) => {
  if (typeof opt === "function") {
    cb = opt;
    opt = {};
  }
  if (!opt)
    opt = {};
  const { pathEnv, pathExt, pathExtExe } = getPathInfo$1(cmd, opt);
  const found = [];
  const step = (i) => new Promise((resolve3, reject) => {
    if (i === pathEnv.length)
      return opt.all && found.length ? resolve3(found) : reject(getNotFoundError$1(cmd));
    const ppRaw = pathEnv[i];
    const pathPart = /^".*"$/.test(ppRaw) ? ppRaw.slice(1, -1) : ppRaw;
    const pCmd = path$2.join(pathPart, cmd);
    const p = !pathPart && /^\.[\\\/]/.test(cmd) ? cmd.slice(0, 2) + pCmd : pCmd;
    resolve3(subStep(p, i, 0));
  });
  const subStep = (p, i, ii) => new Promise((resolve3, reject) => {
    if (ii === pathExt.length)
      return resolve3(step(i + 1));
    const ext = pathExt[ii];
    isexe$3(p + ext, { pathExt: pathExtExe }, (er, is) => {
      if (!er && is) {
        if (opt.all)
          found.push(p + ext);
        else
          return resolve3(p + ext);
      }
      return resolve3(subStep(p, i, ii + 1));
    });
  });
  return cb ? step(0).then((res) => cb(null, res), cb) : step(0);
};
var whichSync$1 = (cmd, opt) => {
  opt = opt || {};
  const { pathEnv, pathExt, pathExtExe } = getPathInfo$1(cmd, opt);
  const found = [];
  for (let i = 0; i < pathEnv.length; i++) {
    const ppRaw = pathEnv[i];
    const pathPart = /^".*"$/.test(ppRaw) ? ppRaw.slice(1, -1) : ppRaw;
    const pCmd = path$2.join(pathPart, cmd);
    const p = !pathPart && /^\.[\\\/]/.test(cmd) ? cmd.slice(0, 2) + pCmd : pCmd;
    for (let j = 0; j < pathExt.length; j++) {
      const cur = p + pathExt[j];
      try {
        const is = isexe$3.sync(cur, { pathExt: pathExtExe });
        if (is) {
          if (opt.all)
            found.push(cur);
          else
            return cur;
        }
      } catch (ex) {
      }
    }
  }
  if (opt.all && found.length)
    return found;
  if (opt.nothrow)
    return null;
  throw getNotFoundError$1(cmd);
};
var which_1 = which$3;
which$3.sync = whichSync$1;
var pathKey$2 = { exports: {} };
var pathKey$1 = (options2 = {}) => {
  const environment = options2.env || process.env;
  const platform = options2.platform || process.platform;
  if (platform !== "win32") {
    return "PATH";
  }
  return Object.keys(environment).reverse().find((key) => key.toUpperCase() === "PATH") || "Path";
};
pathKey$2.exports = pathKey$1;
pathKey$2.exports.default = pathKey$1;
var pathKeyExports = pathKey$2.exports;
var path$1 = import_path5.default;
var which$2 = which_1;
var getPathKey = pathKeyExports;
function resolveCommandAttempt(parsed, withoutPathExt) {
  const env2 = parsed.options.env || process.env;
  const cwd = process.cwd();
  const hasCustomCwd = parsed.options.cwd != null;
  const shouldSwitchCwd = hasCustomCwd && process.chdir !== void 0 && !process.chdir.disabled;
  if (shouldSwitchCwd) {
    try {
      process.chdir(parsed.options.cwd);
    } catch (err) {
    }
  }
  let resolved;
  try {
    resolved = which$2.sync(parsed.command, {
      path: env2[getPathKey({ env: env2 })],
      pathExt: withoutPathExt ? path$1.delimiter : void 0
    });
  } catch (e) {
  } finally {
    if (shouldSwitchCwd) {
      process.chdir(cwd);
    }
  }
  if (resolved) {
    resolved = path$1.resolve(hasCustomCwd ? parsed.options.cwd : "", resolved);
  }
  return resolved;
}
function resolveCommand$1(parsed) {
  return resolveCommandAttempt(parsed) || resolveCommandAttempt(parsed, true);
}
var resolveCommand_1 = resolveCommand$1;
var _escape = {};
var metaCharsRegExp = /([()\][%!^"`<>&|;, *?])/g;
function escapeCommand(arg) {
  arg = arg.replace(metaCharsRegExp, "^$1");
  return arg;
}
function escapeArgument(arg, doubleEscapeMetaChars) {
  arg = `${arg}`;
  arg = arg.replace(/(\\*)"/g, '$1$1\\"');
  arg = arg.replace(/(\\*)$/, "$1$1");
  arg = `"${arg}"`;
  arg = arg.replace(metaCharsRegExp, "^$1");
  if (doubleEscapeMetaChars) {
    arg = arg.replace(metaCharsRegExp, "^$1");
  }
  return arg;
}
_escape.command = escapeCommand;
_escape.argument = escapeArgument;
var shebangRegex$1 = /^#!(.*)/;
var shebangRegex = shebangRegex$1;
var shebangCommand$1 = (string = "") => {
  const match = string.match(shebangRegex);
  if (!match) {
    return null;
  }
  const [path10, argument] = match[0].replace(/#! ?/, "").split(" ");
  const binary = path10.split("/").pop();
  if (binary === "env") {
    return argument;
  }
  return argument ? `${binary} ${argument}` : binary;
};
var fs2 = import_fs.default;
var shebangCommand = shebangCommand$1;
function readShebang$1(command) {
  const size = 150;
  const buffer = Buffer.alloc(size);
  let fd;
  try {
    fd = fs2.openSync(command, "r");
    fs2.readSync(fd, buffer, 0, size, 0);
    fs2.closeSync(fd);
  } catch (e) {
  }
  return shebangCommand(buffer.toString());
}
var readShebang_1 = readShebang$1;
var path5 = import_path5.default;
var resolveCommand = resolveCommand_1;
var escape = _escape;
var readShebang = readShebang_1;
var isWin$1 = process.platform === "win32";
var isExecutableRegExp = /\.(?:com|exe)$/i;
var isCmdShimRegExp = /node_modules[\\/].bin[\\/][^\\/]+\.cmd$/i;
function detectShebang(parsed) {
  parsed.file = resolveCommand(parsed);
  const shebang = parsed.file && readShebang(parsed.file);
  if (shebang) {
    parsed.args.unshift(parsed.file);
    parsed.command = shebang;
    return resolveCommand(parsed);
  }
  return parsed.file;
}
function parseNonShell(parsed) {
  if (!isWin$1) {
    return parsed;
  }
  const commandFile = detectShebang(parsed);
  const needsShell = !isExecutableRegExp.test(commandFile);
  if (parsed.options.forceShell || needsShell) {
    const needsDoubleEscapeMetaChars = isCmdShimRegExp.test(commandFile);
    parsed.command = path5.normalize(parsed.command);
    parsed.command = escape.command(parsed.command);
    parsed.args = parsed.args.map((arg) => escape.argument(arg, needsDoubleEscapeMetaChars));
    const shellCommand = [parsed.command].concat(parsed.args).join(" ");
    parsed.args = ["/d", "/s", "/c", `"${shellCommand}"`];
    parsed.command = process.env.comspec || "cmd.exe";
    parsed.options.windowsVerbatimArguments = true;
  }
  return parsed;
}
function parse$1(command, args, options2) {
  if (args && !Array.isArray(args)) {
    options2 = args;
    args = null;
  }
  args = args ? args.slice(0) : [];
  options2 = Object.assign({}, options2);
  const parsed = {
    command,
    args,
    options: options2,
    file: void 0,
    original: {
      command,
      args
    }
  };
  return options2.shell ? parsed : parseNonShell(parsed);
}
var parse_1 = parse$1;
var isWin = process.platform === "win32";
function notFoundError(original, syscall) {
  return Object.assign(new Error(`${syscall} ${original.command} ENOENT`), {
    code: "ENOENT",
    errno: "ENOENT",
    syscall: `${syscall} ${original.command}`,
    path: original.command,
    spawnargs: original.args
  });
}
function hookChildProcess(cp2, parsed) {
  if (!isWin) {
    return;
  }
  const originalEmit = cp2.emit;
  cp2.emit = function(name, arg1) {
    if (name === "exit") {
      const err = verifyENOENT(arg1, parsed);
      if (err) {
        return originalEmit.call(cp2, "error", err);
      }
    }
    return originalEmit.apply(cp2, arguments);
  };
}
function verifyENOENT(status, parsed) {
  if (isWin && status === 1 && !parsed.file) {
    return notFoundError(parsed.original, "spawn");
  }
  return null;
}
function verifyENOENTSync(status, parsed) {
  if (isWin && status === 1 && !parsed.file) {
    return notFoundError(parsed.original, "spawnSync");
  }
  return null;
}
var enoent$1 = {
  hookChildProcess,
  verifyENOENT,
  verifyENOENTSync,
  notFoundError
};
var cp = import_child_process.default;
var parse = parse_1;
var enoent = enoent$1;
function spawn(command, args, options2) {
  const parsed = parse(command, args, options2);
  const spawned = cp.spawn(parsed.command, parsed.args, parsed.options);
  enoent.hookChildProcess(spawned, parsed);
  return spawned;
}
function spawnSync(command, args, options2) {
  const parsed = parse(command, args, options2);
  const result = cp.spawnSync(parsed.command, parsed.args, parsed.options);
  result.error = result.error || enoent.verifyENOENTSync(result.status, parsed);
  return result;
}
crossSpawn$1.exports = spawn;
crossSpawn$1.exports.spawn = spawn;
crossSpawn$1.exports.sync = spawnSync;
crossSpawn$1.exports._parse = parse;
crossSpawn$1.exports._enoent = enoent;
var crossSpawnExports = crossSpawn$1.exports;
var crossSpawn = /* @__PURE__ */ getDefaultExportFromCjs(crossSpawnExports);
function stripFinalNewline(input) {
  const LF = typeof input === "string" ? "\n" : "\n".charCodeAt();
  const CR = typeof input === "string" ? "\r" : "\r".charCodeAt();
  if (input[input.length - 1] === LF) {
    input = input.slice(0, -1);
  }
  if (input[input.length - 1] === CR) {
    input = input.slice(0, -1);
  }
  return input;
}
function pathKey(options2 = {}) {
  const {
    env: env2 = process.env,
    platform = process.platform
  } = options2;
  if (platform !== "win32") {
    return "PATH";
  }
  return Object.keys(env2).reverse().find((key) => key.toUpperCase() === "PATH") || "Path";
}
function npmRunPath(options2 = {}) {
  const {
    cwd = import_node_process3.default.cwd(),
    path: path_ = import_node_process3.default.env[pathKey()],
    execPath = import_node_process3.default.execPath
  } = options2;
  let previous;
  const cwdString = cwd instanceof URL ? import_node_url2.default.fileURLToPath(cwd) : cwd;
  let cwdPath = import_node_path2.default.resolve(cwdString);
  const result = [];
  while (previous !== cwdPath) {
    result.push(import_node_path2.default.join(cwdPath, "node_modules/.bin"));
    previous = cwdPath;
    cwdPath = import_node_path2.default.resolve(cwdPath, "..");
  }
  result.push(import_node_path2.default.resolve(cwdString, execPath, ".."));
  return [...result, path_].join(import_node_path2.default.delimiter);
}
function npmRunPathEnv({ env: env2 = import_node_process3.default.env, ...options2 } = {}) {
  env2 = { ...env2 };
  const path10 = pathKey({ env: env2 });
  options2.path = env2[path10];
  env2[path10] = npmRunPath(options2);
  return env2;
}
var copyProperty = (to, from, property2, ignoreNonConfigurable) => {
  if (property2 === "length" || property2 === "prototype") {
    return;
  }
  if (property2 === "arguments" || property2 === "caller") {
    return;
  }
  const toDescriptor = Object.getOwnPropertyDescriptor(to, property2);
  const fromDescriptor = Object.getOwnPropertyDescriptor(from, property2);
  if (!canCopyProperty(toDescriptor, fromDescriptor) && ignoreNonConfigurable) {
    return;
  }
  Object.defineProperty(to, property2, fromDescriptor);
};
var canCopyProperty = function(toDescriptor, fromDescriptor) {
  return toDescriptor === void 0 || toDescriptor.configurable || toDescriptor.writable === fromDescriptor.writable && toDescriptor.enumerable === fromDescriptor.enumerable && toDescriptor.configurable === fromDescriptor.configurable && (toDescriptor.writable || toDescriptor.value === fromDescriptor.value);
};
var changePrototype = (to, from) => {
  const fromPrototype = Object.getPrototypeOf(from);
  if (fromPrototype === Object.getPrototypeOf(to)) {
    return;
  }
  Object.setPrototypeOf(to, fromPrototype);
};
var wrappedToString = (withName, fromBody) => `/* Wrapped ${withName}*/
${fromBody}`;
var toStringDescriptor = Object.getOwnPropertyDescriptor(Function.prototype, "toString");
var toStringName = Object.getOwnPropertyDescriptor(Function.prototype.toString, "name");
var changeToString = (to, from, name) => {
  const withName = name === "" ? "" : `with ${name.trim()}() `;
  const newToString = wrappedToString.bind(null, withName, from.toString());
  Object.defineProperty(newToString, "name", toStringName);
  Object.defineProperty(to, "toString", { ...toStringDescriptor, value: newToString });
};
function mimicFunction(to, from, { ignoreNonConfigurable = false } = {}) {
  const { name } = to;
  for (const property2 of Reflect.ownKeys(from)) {
    copyProperty(to, from, property2, ignoreNonConfigurable);
  }
  changePrototype(to, from);
  changeToString(to, from, name);
  return to;
}
var calledFunctions = /* @__PURE__ */ new WeakMap();
var onetime = (function_, options2 = {}) => {
  if (typeof function_ !== "function") {
    throw new TypeError("Expected a function");
  }
  let returnValue;
  let callCount = 0;
  const functionName = function_.displayName || function_.name || "<anonymous>";
  const onetime2 = function(...arguments_) {
    calledFunctions.set(onetime2, ++callCount);
    if (callCount === 1) {
      returnValue = function_.apply(this, arguments_);
      function_ = null;
    } else if (options2.throw === true) {
      throw new Error(`Function \`${functionName}\` can only be called once`);
    }
    return returnValue;
  };
  mimicFunction(onetime2, function_);
  calledFunctions.set(onetime2, callCount);
  return onetime2;
};
onetime.callCount = (function_) => {
  if (!calledFunctions.has(function_)) {
    throw new Error(`The given function \`${function_.name}\` is not wrapped by the \`onetime\` package`);
  }
  return calledFunctions.get(function_);
};
var getRealtimeSignals = () => {
  const length = SIGRTMAX - SIGRTMIN + 1;
  return Array.from({ length }, getRealtimeSignal);
};
var getRealtimeSignal = (value, index) => ({
  name: `SIGRT${index + 1}`,
  number: SIGRTMIN + index,
  action: "terminate",
  description: "Application-specific signal (realtime)",
  standard: "posix"
});
var SIGRTMIN = 34;
var SIGRTMAX = 64;
var SIGNALS = [
  {
    name: "SIGHUP",
    number: 1,
    action: "terminate",
    description: "Terminal closed",
    standard: "posix"
  },
  {
    name: "SIGINT",
    number: 2,
    action: "terminate",
    description: "User interruption with CTRL-C",
    standard: "ansi"
  },
  {
    name: "SIGQUIT",
    number: 3,
    action: "core",
    description: "User interruption with CTRL-\\",
    standard: "posix"
  },
  {
    name: "SIGILL",
    number: 4,
    action: "core",
    description: "Invalid machine instruction",
    standard: "ansi"
  },
  {
    name: "SIGTRAP",
    number: 5,
    action: "core",
    description: "Debugger breakpoint",
    standard: "posix"
  },
  {
    name: "SIGABRT",
    number: 6,
    action: "core",
    description: "Aborted",
    standard: "ansi"
  },
  {
    name: "SIGIOT",
    number: 6,
    action: "core",
    description: "Aborted",
    standard: "bsd"
  },
  {
    name: "SIGBUS",
    number: 7,
    action: "core",
    description: "Bus error due to misaligned, non-existing address or paging error",
    standard: "bsd"
  },
  {
    name: "SIGEMT",
    number: 7,
    action: "terminate",
    description: "Command should be emulated but is not implemented",
    standard: "other"
  },
  {
    name: "SIGFPE",
    number: 8,
    action: "core",
    description: "Floating point arithmetic error",
    standard: "ansi"
  },
  {
    name: "SIGKILL",
    number: 9,
    action: "terminate",
    description: "Forced termination",
    standard: "posix",
    forced: true
  },
  {
    name: "SIGUSR1",
    number: 10,
    action: "terminate",
    description: "Application-specific signal",
    standard: "posix"
  },
  {
    name: "SIGSEGV",
    number: 11,
    action: "core",
    description: "Segmentation fault",
    standard: "ansi"
  },
  {
    name: "SIGUSR2",
    number: 12,
    action: "terminate",
    description: "Application-specific signal",
    standard: "posix"
  },
  {
    name: "SIGPIPE",
    number: 13,
    action: "terminate",
    description: "Broken pipe or socket",
    standard: "posix"
  },
  {
    name: "SIGALRM",
    number: 14,
    action: "terminate",
    description: "Timeout or timer",
    standard: "posix"
  },
  {
    name: "SIGTERM",
    number: 15,
    action: "terminate",
    description: "Termination",
    standard: "ansi"
  },
  {
    name: "SIGSTKFLT",
    number: 16,
    action: "terminate",
    description: "Stack is empty or overflowed",
    standard: "other"
  },
  {
    name: "SIGCHLD",
    number: 17,
    action: "ignore",
    description: "Child process terminated, paused or unpaused",
    standard: "posix"
  },
  {
    name: "SIGCLD",
    number: 17,
    action: "ignore",
    description: "Child process terminated, paused or unpaused",
    standard: "other"
  },
  {
    name: "SIGCONT",
    number: 18,
    action: "unpause",
    description: "Unpaused",
    standard: "posix",
    forced: true
  },
  {
    name: "SIGSTOP",
    number: 19,
    action: "pause",
    description: "Paused",
    standard: "posix",
    forced: true
  },
  {
    name: "SIGTSTP",
    number: 20,
    action: "pause",
    description: 'Paused using CTRL-Z or "suspend"',
    standard: "posix"
  },
  {
    name: "SIGTTIN",
    number: 21,
    action: "pause",
    description: "Background process cannot read terminal input",
    standard: "posix"
  },
  {
    name: "SIGBREAK",
    number: 21,
    action: "terminate",
    description: "User interruption with CTRL-BREAK",
    standard: "other"
  },
  {
    name: "SIGTTOU",
    number: 22,
    action: "pause",
    description: "Background process cannot write to terminal output",
    standard: "posix"
  },
  {
    name: "SIGURG",
    number: 23,
    action: "ignore",
    description: "Socket received out-of-band data",
    standard: "bsd"
  },
  {
    name: "SIGXCPU",
    number: 24,
    action: "core",
    description: "Process timed out",
    standard: "bsd"
  },
  {
    name: "SIGXFSZ",
    number: 25,
    action: "core",
    description: "File too big",
    standard: "bsd"
  },
  {
    name: "SIGVTALRM",
    number: 26,
    action: "terminate",
    description: "Timeout or timer",
    standard: "bsd"
  },
  {
    name: "SIGPROF",
    number: 27,
    action: "terminate",
    description: "Timeout or timer",
    standard: "bsd"
  },
  {
    name: "SIGWINCH",
    number: 28,
    action: "ignore",
    description: "Terminal window size changed",
    standard: "bsd"
  },
  {
    name: "SIGIO",
    number: 29,
    action: "terminate",
    description: "I/O is available",
    standard: "other"
  },
  {
    name: "SIGPOLL",
    number: 29,
    action: "terminate",
    description: "Watched event",
    standard: "other"
  },
  {
    name: "SIGINFO",
    number: 29,
    action: "ignore",
    description: "Request for process information",
    standard: "other"
  },
  {
    name: "SIGPWR",
    number: 30,
    action: "terminate",
    description: "Device running out of power",
    standard: "systemv"
  },
  {
    name: "SIGSYS",
    number: 31,
    action: "core",
    description: "Invalid system call",
    standard: "other"
  },
  {
    name: "SIGUNUSED",
    number: 31,
    action: "terminate",
    description: "Invalid system call",
    standard: "other"
  }
];
var getSignals = () => {
  const realtimeSignals = getRealtimeSignals();
  const signals2 = [...SIGNALS, ...realtimeSignals].map(normalizeSignal);
  return signals2;
};
var normalizeSignal = ({
  name,
  number: defaultNumber,
  description,
  action: action2,
  forced = false,
  standard
}) => {
  const {
    signals: { [name]: constantSignal }
  } = import_node_os.constants;
  const supported = constantSignal !== void 0;
  const number2 = supported ? constantSignal : defaultNumber;
  return { name, number: number2, description, supported, action: action2, forced, standard };
};
var getSignalsByName = () => {
  const signals2 = getSignals();
  return Object.fromEntries(signals2.map(getSignalByName));
};
var getSignalByName = ({
  name,
  number: number2,
  description,
  supported,
  action: action2,
  forced,
  standard
}) => [name, { name, number: number2, description, supported, action: action2, forced, standard }];
var signalsByName = getSignalsByName();
var getSignalsByNumber = () => {
  const signals2 = getSignals();
  const length = SIGRTMAX + 1;
  const signalsA = Array.from(
    { length },
    (value, number2) => getSignalByNumber(number2, signals2)
  );
  return Object.assign({}, ...signalsA);
};
var getSignalByNumber = (number2, signals2) => {
  const signal = findSignalByNumber(number2, signals2);
  if (signal === void 0) {
    return {};
  }
  const { name, description, supported, action: action2, forced, standard } = signal;
  return {
    [number2]: {
      name,
      number: number2,
      description,
      supported,
      action: action2,
      forced,
      standard
    }
  };
};
var findSignalByNumber = (number2, signals2) => {
  const signal = signals2.find(({ name }) => import_node_os.constants.signals[name] === number2);
  if (signal !== void 0) {
    return signal;
  }
  return signals2.find((signalA) => signalA.number === number2);
};
getSignalsByNumber();
var getErrorPrefix = ({ timedOut, timeout, errorCode, signal, signalDescription, exitCode, isCanceled }) => {
  if (timedOut) {
    return `timed out after ${timeout} milliseconds`;
  }
  if (isCanceled) {
    return "was canceled";
  }
  if (errorCode !== void 0) {
    return `failed with ${errorCode}`;
  }
  if (signal !== void 0) {
    return `was killed with ${signal} (${signalDescription})`;
  }
  if (exitCode !== void 0) {
    return `failed with exit code ${exitCode}`;
  }
  return "failed";
};
var makeError = ({
  stdout,
  stderr,
  all,
  error,
  signal,
  exitCode,
  command,
  escapedCommand,
  timedOut,
  isCanceled,
  killed,
  parsed: { options: { timeout, cwd = import_node_process3.default.cwd() } }
}) => {
  exitCode = exitCode === null ? void 0 : exitCode;
  signal = signal === null ? void 0 : signal;
  const signalDescription = signal === void 0 ? void 0 : signalsByName[signal].description;
  const errorCode = error && error.code;
  const prefix = getErrorPrefix({ timedOut, timeout, errorCode, signal, signalDescription, exitCode, isCanceled });
  const execaMessage = `Command ${prefix}: ${command}`;
  const isError = Object.prototype.toString.call(error) === "[object Error]";
  const shortMessage = isError ? `${execaMessage}
${error.message}` : execaMessage;
  const message = [shortMessage, stderr, stdout].filter(Boolean).join("\n");
  if (isError) {
    error.originalMessage = error.message;
    error.message = message;
  } else {
    error = new Error(message);
  }
  error.shortMessage = shortMessage;
  error.command = command;
  error.escapedCommand = escapedCommand;
  error.exitCode = exitCode;
  error.signal = signal;
  error.signalDescription = signalDescription;
  error.stdout = stdout;
  error.stderr = stderr;
  error.cwd = cwd;
  if (all !== void 0) {
    error.all = all;
  }
  if ("bufferedData" in error) {
    delete error.bufferedData;
  }
  error.failed = true;
  error.timedOut = Boolean(timedOut);
  error.isCanceled = isCanceled;
  error.killed = killed && !timedOut;
  return error;
};
var aliases = ["stdin", "stdout", "stderr"];
var hasAlias = (options2) => aliases.some((alias) => options2[alias] !== void 0);
var normalizeStdio = (options2) => {
  if (!options2) {
    return;
  }
  const { stdio } = options2;
  if (stdio === void 0) {
    return aliases.map((alias) => options2[alias]);
  }
  if (hasAlias(options2)) {
    throw new Error(`It's not possible to provide \`stdio\` in combination with one of ${aliases.map((alias) => `\`${alias}\``).join(", ")}`);
  }
  if (typeof stdio === "string") {
    return stdio;
  }
  if (!Array.isArray(stdio)) {
    throw new TypeError(`Expected \`stdio\` to be of type \`string\` or \`Array\`, got \`${typeof stdio}\``);
  }
  const length = Math.max(stdio.length, aliases.length);
  return Array.from({ length }, (value, index) => stdio[index]);
};
var signals = [];
signals.push("SIGHUP", "SIGINT", "SIGTERM");
if (process.platform !== "win32") {
  signals.push(
    "SIGALRM",
    "SIGABRT",
    "SIGVTALRM",
    "SIGXCPU",
    "SIGXFSZ",
    "SIGUSR2",
    "SIGTRAP",
    "SIGSYS",
    "SIGQUIT",
    "SIGIOT"
    // should detect profiler and enable/disable accordingly.
    // see #21
    // 'SIGPROF'
  );
}
if (process.platform === "linux") {
  signals.push("SIGIO", "SIGPOLL", "SIGPWR", "SIGSTKFLT");
}
var processOk = (process4) => !!process4 && typeof process4 === "object" && typeof process4.removeListener === "function" && typeof process4.emit === "function" && typeof process4.reallyExit === "function" && typeof process4.listeners === "function" && typeof process4.kill === "function" && typeof process4.pid === "number" && typeof process4.on === "function";
var kExitEmitter = Symbol.for("signal-exit emitter");
var global$1 = globalThis;
var ObjectDefineProperty = Object.defineProperty.bind(Object);
var Emitter = class {
  emitted = {
    afterExit: false,
    exit: false
  };
  listeners = {
    afterExit: [],
    exit: []
  };
  count = 0;
  id = Math.random();
  constructor() {
    if (global$1[kExitEmitter]) {
      return global$1[kExitEmitter];
    }
    ObjectDefineProperty(global$1, kExitEmitter, {
      value: this,
      writable: false,
      enumerable: false,
      configurable: false
    });
  }
  on(ev, fn) {
    this.listeners[ev].push(fn);
  }
  removeListener(ev, fn) {
    const list = this.listeners[ev];
    const i = list.indexOf(fn);
    if (i === -1) {
      return;
    }
    if (i === 0 && list.length === 1) {
      list.length = 0;
    } else {
      list.splice(i, 1);
    }
  }
  emit(ev, code, signal) {
    if (this.emitted[ev]) {
      return false;
    }
    this.emitted[ev] = true;
    let ret = false;
    for (const fn of this.listeners[ev]) {
      ret = fn(code, signal) === true || ret;
    }
    if (ev === "exit") {
      ret = this.emit("afterExit", code, signal) || ret;
    }
    return ret;
  }
};
var SignalExitBase = class {
};
var signalExitWrap = (handler) => {
  return {
    onExit(cb, opts) {
      return handler.onExit(cb, opts);
    },
    load() {
      return handler.load();
    },
    unload() {
      return handler.unload();
    }
  };
};
var SignalExitFallback = class extends SignalExitBase {
  onExit() {
    return () => {
    };
  }
  load() {
  }
  unload() {
  }
};
var SignalExit = class extends SignalExitBase {
  // "SIGHUP" throws an `ENOSYS` error on Windows,
  // so use a supported signal instead
  /* c8 ignore start */
  #hupSig = process$1.platform === "win32" ? "SIGINT" : "SIGHUP";
  /* c8 ignore stop */
  #emitter = new Emitter();
  #process;
  #originalProcessEmit;
  #originalProcessReallyExit;
  #sigListeners = {};
  #loaded = false;
  constructor(process4) {
    super();
    this.#process = process4;
    this.#sigListeners = {};
    for (const sig of signals) {
      this.#sigListeners[sig] = () => {
        const listeners = this.#process.listeners(sig);
        let { count } = this.#emitter;
        const p = process4;
        if (typeof p.__signal_exit_emitter__ === "object" && typeof p.__signal_exit_emitter__.count === "number") {
          count += p.__signal_exit_emitter__.count;
        }
        if (listeners.length === count) {
          this.unload();
          const ret = this.#emitter.emit("exit", null, sig);
          const s = sig === "SIGHUP" ? this.#hupSig : sig;
          if (!ret)
            process4.kill(process4.pid, s);
        }
      };
    }
    this.#originalProcessReallyExit = process4.reallyExit;
    this.#originalProcessEmit = process4.emit;
  }
  onExit(cb, opts) {
    if (!processOk(this.#process)) {
      return () => {
      };
    }
    if (this.#loaded === false) {
      this.load();
    }
    const ev = opts?.alwaysLast ? "afterExit" : "exit";
    this.#emitter.on(ev, cb);
    return () => {
      this.#emitter.removeListener(ev, cb);
      if (this.#emitter.listeners["exit"].length === 0 && this.#emitter.listeners["afterExit"].length === 0) {
        this.unload();
      }
    };
  }
  load() {
    if (this.#loaded) {
      return;
    }
    this.#loaded = true;
    this.#emitter.count += 1;
    for (const sig of signals) {
      try {
        const fn = this.#sigListeners[sig];
        if (fn)
          this.#process.on(sig, fn);
      } catch (_) {
      }
    }
    this.#process.emit = (ev, ...a) => {
      return this.#processEmit(ev, ...a);
    };
    this.#process.reallyExit = (code) => {
      return this.#processReallyExit(code);
    };
  }
  unload() {
    if (!this.#loaded) {
      return;
    }
    this.#loaded = false;
    signals.forEach((sig) => {
      const listener = this.#sigListeners[sig];
      if (!listener) {
        throw new Error("Listener not defined for signal: " + sig);
      }
      try {
        this.#process.removeListener(sig, listener);
      } catch (_) {
      }
    });
    this.#process.emit = this.#originalProcessEmit;
    this.#process.reallyExit = this.#originalProcessReallyExit;
    this.#emitter.count -= 1;
  }
  #processReallyExit(code) {
    if (!processOk(this.#process)) {
      return 0;
    }
    this.#process.exitCode = code || 0;
    this.#emitter.emit("exit", this.#process.exitCode, null);
    return this.#originalProcessReallyExit.call(this.#process, this.#process.exitCode);
  }
  #processEmit(ev, ...args) {
    const og = this.#originalProcessEmit;
    if (ev === "exit" && processOk(this.#process)) {
      if (typeof args[0] === "number") {
        this.#process.exitCode = args[0];
      }
      const ret = og.call(this.#process, ev, ...args);
      this.#emitter.emit("exit", this.#process.exitCode, null);
      return ret;
    } else {
      return og.call(this.#process, ev, ...args);
    }
  }
};
var process$1 = globalThis.process;
var {
  /**
   * Called when the process is exiting, whether via signal, explicit
   * exit, or running out of stuff to do.
   *
   * If the global process object is not suitable for instrumentation,
   * then this will be a no-op.
   *
   * Returns a function that may be used to unload signal-exit.
   */
  onExit,
  /**
   * Load the listeners.  Likely you never need to call this, unless
   * doing a rather deep integration with signal-exit functionality.
   * Mostly exposed for the benefit of testing.
   *
   * @internal
   */
  load,
  /**
   * Unload the listeners.  Likely you never need to call this, unless
   * doing a rather deep integration with signal-exit functionality.
   * Mostly exposed for the benefit of testing.
   *
   * @internal
   */
  unload
} = signalExitWrap(processOk(process$1) ? new SignalExit(process$1) : new SignalExitFallback());
var DEFAULT_FORCE_KILL_TIMEOUT = 1e3 * 5;
var spawnedKill = (kill, signal = "SIGTERM", options2 = {}) => {
  const killResult = kill(signal);
  setKillTimeout(kill, signal, options2, killResult);
  return killResult;
};
var setKillTimeout = (kill, signal, options2, killResult) => {
  if (!shouldForceKill(signal, options2, killResult)) {
    return;
  }
  const timeout = getForceKillAfterTimeout(options2);
  const t = setTimeout(() => {
    kill("SIGKILL");
  }, timeout);
  if (t.unref) {
    t.unref();
  }
};
var shouldForceKill = (signal, { forceKillAfterTimeout }, killResult) => isSigterm(signal) && forceKillAfterTimeout !== false && killResult;
var isSigterm = (signal) => signal === import_node_os.default.constants.signals.SIGTERM || typeof signal === "string" && signal.toUpperCase() === "SIGTERM";
var getForceKillAfterTimeout = ({ forceKillAfterTimeout = true }) => {
  if (forceKillAfterTimeout === true) {
    return DEFAULT_FORCE_KILL_TIMEOUT;
  }
  if (!Number.isFinite(forceKillAfterTimeout) || forceKillAfterTimeout < 0) {
    throw new TypeError(`Expected the \`forceKillAfterTimeout\` option to be a non-negative integer, got \`${forceKillAfterTimeout}\` (${typeof forceKillAfterTimeout})`);
  }
  return forceKillAfterTimeout;
};
var spawnedCancel = (spawned, context) => {
  const killResult = spawned.kill();
  if (killResult) {
    context.isCanceled = true;
  }
};
var timeoutKill = (spawned, signal, reject) => {
  spawned.kill(signal);
  reject(Object.assign(new Error("Timed out"), { timedOut: true, signal }));
};
var setupTimeout = (spawned, { timeout, killSignal = "SIGTERM" }, spawnedPromise) => {
  if (timeout === 0 || timeout === void 0) {
    return spawnedPromise;
  }
  let timeoutId;
  const timeoutPromise = new Promise((resolve3, reject) => {
    timeoutId = setTimeout(() => {
      timeoutKill(spawned, killSignal, reject);
    }, timeout);
  });
  const safeSpawnedPromise = spawnedPromise.finally(() => {
    clearTimeout(timeoutId);
  });
  return Promise.race([timeoutPromise, safeSpawnedPromise]);
};
var validateTimeout = ({ timeout }) => {
  if (timeout !== void 0 && (!Number.isFinite(timeout) || timeout < 0)) {
    throw new TypeError(`Expected the \`timeout\` option to be a non-negative integer, got \`${timeout}\` (${typeof timeout})`);
  }
};
var setExitHandler = async (spawned, { cleanup, detached }, timedPromise) => {
  if (!cleanup || detached) {
    return timedPromise;
  }
  const removeExitHandler = onExit(() => {
    spawned.kill();
  });
  return timedPromise.finally(() => {
    removeExitHandler();
  });
};
function isStream(stream) {
  return stream !== null && typeof stream === "object" && typeof stream.pipe === "function";
}
function isWritableStream(stream) {
  return isStream(stream) && stream.writable !== false && typeof stream._write === "function" && typeof stream._writableState === "object";
}
var isExecaChildProcess = (target) => target instanceof import_node_child_process.ChildProcess && typeof target.then === "function";
var pipeToTarget = (spawned, streamName, target) => {
  if (typeof target === "string") {
    spawned[streamName].pipe((0, import_node_fs.createWriteStream)(target));
    return spawned;
  }
  if (isWritableStream(target)) {
    spawned[streamName].pipe(target);
    return spawned;
  }
  if (!isExecaChildProcess(target)) {
    throw new TypeError("The second argument must be a string, a stream or an Execa child process.");
  }
  if (!isWritableStream(target.stdin)) {
    throw new TypeError("The target child process's stdin must be available.");
  }
  spawned[streamName].pipe(target.stdin);
  return target;
};
var addPipeMethods = (spawned) => {
  if (spawned.stdout !== null) {
    spawned.pipeStdout = pipeToTarget.bind(void 0, spawned, "stdout");
  }
  if (spawned.stderr !== null) {
    spawned.pipeStderr = pipeToTarget.bind(void 0, spawned, "stderr");
  }
  if (spawned.all !== void 0) {
    spawned.pipeAll = pipeToTarget.bind(void 0, spawned, "all");
  }
};
var getStreamContents = async (stream, { init: init3, convertChunk, getSize, truncateChunk, addChunk, getFinalChunk, finalize }, { maxBuffer = Number.POSITIVE_INFINITY } = {}) => {
  if (!isAsyncIterable(stream)) {
    throw new Error("The first argument must be a Readable, a ReadableStream, or an async iterable.");
  }
  const state = init3();
  state.length = 0;
  try {
    for await (const chunk of stream) {
      const chunkType = getChunkType(chunk);
      const convertedChunk = convertChunk[chunkType](chunk, state);
      appendChunk({ convertedChunk, state, getSize, truncateChunk, addChunk, maxBuffer });
    }
    appendFinalChunk({ state, convertChunk, getSize, truncateChunk, addChunk, getFinalChunk, maxBuffer });
    return finalize(state);
  } catch (error) {
    error.bufferedData = finalize(state);
    throw error;
  }
};
var appendFinalChunk = ({ state, getSize, truncateChunk, addChunk, getFinalChunk, maxBuffer }) => {
  const convertedChunk = getFinalChunk(state);
  if (convertedChunk !== void 0) {
    appendChunk({ convertedChunk, state, getSize, truncateChunk, addChunk, maxBuffer });
  }
};
var appendChunk = ({ convertedChunk, state, getSize, truncateChunk, addChunk, maxBuffer }) => {
  const chunkSize = getSize(convertedChunk);
  const newLength = state.length + chunkSize;
  if (newLength <= maxBuffer) {
    addNewChunk(convertedChunk, state, addChunk, newLength);
    return;
  }
  const truncatedChunk = truncateChunk(convertedChunk, maxBuffer - state.length);
  if (truncatedChunk !== void 0) {
    addNewChunk(truncatedChunk, state, addChunk, maxBuffer);
  }
  throw new MaxBufferError();
};
var addNewChunk = (convertedChunk, state, addChunk, newLength) => {
  state.contents = addChunk(convertedChunk, state, newLength);
  state.length = newLength;
};
var isAsyncIterable = (stream) => typeof stream === "object" && stream !== null && typeof stream[Symbol.asyncIterator] === "function";
var getChunkType = (chunk) => {
  const typeOfChunk = typeof chunk;
  if (typeOfChunk === "string") {
    return "string";
  }
  if (typeOfChunk !== "object" || chunk === null) {
    return "others";
  }
  if (globalThis.Buffer?.isBuffer(chunk)) {
    return "buffer";
  }
  const prototypeName = objectToString.call(chunk);
  if (prototypeName === "[object ArrayBuffer]") {
    return "arrayBuffer";
  }
  if (prototypeName === "[object DataView]") {
    return "dataView";
  }
  if (Number.isInteger(chunk.byteLength) && Number.isInteger(chunk.byteOffset) && objectToString.call(chunk.buffer) === "[object ArrayBuffer]") {
    return "typedArray";
  }
  return "others";
};
var { toString: objectToString } = Object.prototype;
var MaxBufferError = class extends Error {
  name = "MaxBufferError";
  constructor() {
    super("maxBuffer exceeded");
  }
};
var identity = (value) => value;
var noop$1 = () => void 0;
var getContentsProp = ({ contents }) => contents;
var throwObjectStream = (chunk) => {
  throw new Error(`Streams in object mode are not supported: ${String(chunk)}`);
};
var getLengthProp = (convertedChunk) => convertedChunk.length;
async function getStreamAsArrayBuffer(stream, options2) {
  return getStreamContents(stream, arrayBufferMethods, options2);
}
var initArrayBuffer = () => ({ contents: new ArrayBuffer(0) });
var useTextEncoder = (chunk) => textEncoder.encode(chunk);
var textEncoder = new TextEncoder();
var useUint8Array = (chunk) => new Uint8Array(chunk);
var useUint8ArrayWithOffset = (chunk) => new Uint8Array(chunk.buffer, chunk.byteOffset, chunk.byteLength);
var truncateArrayBufferChunk = (convertedChunk, chunkSize) => convertedChunk.slice(0, chunkSize);
var addArrayBufferChunk = (convertedChunk, { contents, length: previousLength }, length) => {
  const newContents = hasArrayBufferResize() ? resizeArrayBuffer(contents, length) : resizeArrayBufferSlow(contents, length);
  new Uint8Array(newContents).set(convertedChunk, previousLength);
  return newContents;
};
var resizeArrayBufferSlow = (contents, length) => {
  if (length <= contents.byteLength) {
    return contents;
  }
  const arrayBuffer = new ArrayBuffer(getNewContentsLength(length));
  new Uint8Array(arrayBuffer).set(new Uint8Array(contents), 0);
  return arrayBuffer;
};
var resizeArrayBuffer = (contents, length) => {
  if (length <= contents.maxByteLength) {
    contents.resize(length);
    return contents;
  }
  const arrayBuffer = new ArrayBuffer(length, { maxByteLength: getNewContentsLength(length) });
  new Uint8Array(arrayBuffer).set(new Uint8Array(contents), 0);
  return arrayBuffer;
};
var getNewContentsLength = (length) => SCALE_FACTOR ** Math.ceil(Math.log(length) / Math.log(SCALE_FACTOR));
var SCALE_FACTOR = 2;
var finalizeArrayBuffer = ({ contents, length }) => hasArrayBufferResize() ? contents : contents.slice(0, length);
var hasArrayBufferResize = () => "resize" in ArrayBuffer.prototype;
var arrayBufferMethods = {
  init: initArrayBuffer,
  convertChunk: {
    string: useTextEncoder,
    buffer: useUint8Array,
    arrayBuffer: useUint8Array,
    dataView: useUint8ArrayWithOffset,
    typedArray: useUint8ArrayWithOffset,
    others: throwObjectStream
  },
  getSize: getLengthProp,
  truncateChunk: truncateArrayBufferChunk,
  addChunk: addArrayBufferChunk,
  getFinalChunk: noop$1,
  finalize: finalizeArrayBuffer
};
async function getStreamAsBuffer(stream, options2) {
  if (!("Buffer" in globalThis)) {
    throw new Error("getStreamAsBuffer() is only supported in Node.js");
  }
  try {
    return arrayBufferToNodeBuffer(await getStreamAsArrayBuffer(stream, options2));
  } catch (error) {
    if (error.bufferedData !== void 0) {
      error.bufferedData = arrayBufferToNodeBuffer(error.bufferedData);
    }
    throw error;
  }
}
var arrayBufferToNodeBuffer = (arrayBuffer) => globalThis.Buffer.from(arrayBuffer);
async function getStreamAsString(stream, options2) {
  return getStreamContents(stream, stringMethods, options2);
}
var initString = () => ({ contents: "", textDecoder: new TextDecoder() });
var useTextDecoder = (chunk, { textDecoder }) => textDecoder.decode(chunk, { stream: true });
var addStringChunk = (convertedChunk, { contents }) => contents + convertedChunk;
var truncateStringChunk = (convertedChunk, chunkSize) => convertedChunk.slice(0, chunkSize);
var getFinalStringChunk = ({ textDecoder }) => {
  const finalChunk = textDecoder.decode();
  return finalChunk === "" ? void 0 : finalChunk;
};
var stringMethods = {
  init: initString,
  convertChunk: {
    string: identity,
    buffer: useTextDecoder,
    arrayBuffer: useTextDecoder,
    dataView: useTextDecoder,
    typedArray: useTextDecoder,
    others: throwObjectStream
  },
  getSize: getLengthProp,
  truncateChunk: truncateStringChunk,
  addChunk: addStringChunk,
  getFinalChunk: getFinalStringChunk,
  finalize: getContentsProp
};
var { PassThrough } = import_stream.default;
var mergeStream = function() {
  var sources = [];
  var output = new PassThrough({ objectMode: true });
  output.setMaxListeners(0);
  output.add = add;
  output.isEmpty = isEmpty;
  output.on("unpipe", remove2);
  Array.prototype.slice.call(arguments).forEach(add);
  return output;
  function add(source) {
    if (Array.isArray(source)) {
      source.forEach(add);
      return this;
    }
    sources.push(source);
    source.once("end", remove2.bind(null, source));
    source.once("error", output.emit.bind(output, "error"));
    source.pipe(output, { end: false });
    return this;
  }
  function isEmpty() {
    return sources.length == 0;
  }
  function remove2(source) {
    sources = sources.filter(function(it) {
      return it !== source;
    });
    if (!sources.length && output.readable) {
      output.end();
    }
  }
};
var mergeStream$1 = /* @__PURE__ */ getDefaultExportFromCjs(mergeStream);
var validateInputOptions = (input) => {
  if (input !== void 0) {
    throw new TypeError("The `input` and `inputFile` options cannot be both set.");
  }
};
var getInput = ({ input, inputFile }) => {
  if (typeof inputFile !== "string") {
    return input;
  }
  validateInputOptions(input);
  return (0, import_node_fs.createReadStream)(inputFile);
};
var handleInput = (spawned, options2) => {
  const input = getInput(options2);
  if (input === void 0) {
    return;
  }
  if (isStream(input)) {
    input.pipe(spawned.stdin);
  } else {
    spawned.stdin.end(input);
  }
};
var makeAllStream = (spawned, { all }) => {
  if (!all || !spawned.stdout && !spawned.stderr) {
    return;
  }
  const mixed = mergeStream$1();
  if (spawned.stdout) {
    mixed.add(spawned.stdout);
  }
  if (spawned.stderr) {
    mixed.add(spawned.stderr);
  }
  return mixed;
};
var getBufferedData = async (stream, streamPromise) => {
  if (!stream || streamPromise === void 0) {
    return;
  }
  await (0, import_promises3.setTimeout)(0);
  stream.destroy();
  try {
    return await streamPromise;
  } catch (error) {
    return error.bufferedData;
  }
};
var getStreamPromise = (stream, { encoding, buffer, maxBuffer }) => {
  if (!stream || !buffer) {
    return;
  }
  if (encoding === "utf8" || encoding === "utf-8") {
    return getStreamAsString(stream, { maxBuffer });
  }
  if (encoding === null || encoding === "buffer") {
    return getStreamAsBuffer(stream, { maxBuffer });
  }
  return applyEncoding(stream, maxBuffer, encoding);
};
var applyEncoding = async (stream, maxBuffer, encoding) => {
  const buffer = await getStreamAsBuffer(stream, { maxBuffer });
  return buffer.toString(encoding);
};
var getSpawnedResult = async ({ stdout, stderr, all }, { encoding, buffer, maxBuffer }, processDone) => {
  const stdoutPromise = getStreamPromise(stdout, { encoding, buffer, maxBuffer });
  const stderrPromise = getStreamPromise(stderr, { encoding, buffer, maxBuffer });
  const allPromise = getStreamPromise(all, { encoding, buffer, maxBuffer: maxBuffer * 2 });
  try {
    return await Promise.all([processDone, stdoutPromise, stderrPromise, allPromise]);
  } catch (error) {
    return Promise.all([
      { error, signal: error.signal, timedOut: error.timedOut },
      getBufferedData(stdout, stdoutPromise),
      getBufferedData(stderr, stderrPromise),
      getBufferedData(all, allPromise)
    ]);
  }
};
var nativePromisePrototype = (async () => {
})().constructor.prototype;
var descriptors = ["then", "catch", "finally"].map((property2) => [
  property2,
  Reflect.getOwnPropertyDescriptor(nativePromisePrototype, property2)
]);
var mergePromise = (spawned, promise2) => {
  for (const [property2, descriptor] of descriptors) {
    const value = typeof promise2 === "function" ? (...args) => Reflect.apply(descriptor.value, promise2(), args) : descriptor.value.bind(promise2);
    Reflect.defineProperty(spawned, property2, { ...descriptor, value });
  }
};
var getSpawnedPromise = (spawned) => new Promise((resolve3, reject) => {
  spawned.on("exit", (exitCode, signal) => {
    resolve3({ exitCode, signal });
  });
  spawned.on("error", (error) => {
    reject(error);
  });
  if (spawned.stdin) {
    spawned.stdin.on("error", (error) => {
      reject(error);
    });
  }
});
var normalizeArgs = (file2, args = []) => {
  if (!Array.isArray(args)) {
    return [file2];
  }
  return [file2, ...args];
};
var NO_ESCAPE_REGEXP = /^[\w.-]+$/;
var escapeArg = (arg) => {
  if (typeof arg !== "string" || NO_ESCAPE_REGEXP.test(arg)) {
    return arg;
  }
  return `"${arg.replaceAll('"', '\\"')}"`;
};
var joinCommand = (file2, args) => normalizeArgs(file2, args).join(" ");
var getEscapedCommand = (file2, args) => normalizeArgs(file2, args).map((arg) => escapeArg(arg)).join(" ");
var SPACES_REGEXP = / +/g;
var parseCommand = (command) => {
  const tokens = [];
  for (const token of command.trim().split(SPACES_REGEXP)) {
    const previousToken = tokens.at(-1);
    if (previousToken && previousToken.endsWith("\\")) {
      tokens[tokens.length - 1] = `${previousToken.slice(0, -1)} ${token}`;
    } else {
      tokens.push(token);
    }
  }
  return tokens;
};
var verboseDefault = (0, import_node_util.debuglog)("execa").enabled;
var padField = (field, padding) => String(field).padStart(padding, "0");
var getTimestamp = () => {
  const date3 = /* @__PURE__ */ new Date();
  return `${padField(date3.getHours(), 2)}:${padField(date3.getMinutes(), 2)}:${padField(date3.getSeconds(), 2)}.${padField(date3.getMilliseconds(), 3)}`;
};
var logCommand = (escapedCommand, { verbose }) => {
  if (!verbose) {
    return;
  }
  import_node_process3.default.stderr.write(`[${getTimestamp()}] ${escapedCommand}
`);
};
var DEFAULT_MAX_BUFFER = 1e3 * 1e3 * 100;
var getEnv = ({ env: envOption, extendEnv, preferLocal, localDir, execPath }) => {
  const env2 = extendEnv ? { ...import_node_process3.default.env, ...envOption } : envOption;
  if (preferLocal) {
    return npmRunPathEnv({ env: env2, cwd: localDir, execPath });
  }
  return env2;
};
var handleArguments = (file2, args, options2 = {}) => {
  const parsed = crossSpawn._parse(file2, args, options2);
  file2 = parsed.command;
  args = parsed.args;
  options2 = parsed.options;
  options2 = {
    maxBuffer: DEFAULT_MAX_BUFFER,
    buffer: true,
    stripFinalNewline: true,
    extendEnv: true,
    preferLocal: false,
    localDir: options2.cwd || import_node_process3.default.cwd(),
    execPath: import_node_process3.default.execPath,
    encoding: "utf8",
    reject: true,
    cleanup: true,
    all: false,
    windowsHide: true,
    verbose: verboseDefault,
    ...options2
  };
  options2.env = getEnv(options2);
  options2.stdio = normalizeStdio(options2);
  if (import_node_process3.default.platform === "win32" && import_node_path2.default.basename(file2, ".exe") === "cmd") {
    args.unshift("/q");
  }
  return { file: file2, args, options: options2, parsed };
};
var handleOutput = (options2, value, error) => {
  if (typeof value !== "string" && !import_node_buffer.Buffer.isBuffer(value)) {
    return error === void 0 ? void 0 : "";
  }
  if (options2.stripFinalNewline) {
    return stripFinalNewline(value);
  }
  return value;
};
function execa(file2, args, options2) {
  const parsed = handleArguments(file2, args, options2);
  const command = joinCommand(file2, args);
  const escapedCommand = getEscapedCommand(file2, args);
  logCommand(escapedCommand, parsed.options);
  validateTimeout(parsed.options);
  let spawned;
  try {
    spawned = import_node_child_process.default.spawn(parsed.file, parsed.args, parsed.options);
  } catch (error) {
    const dummySpawned = new import_node_child_process.default.ChildProcess();
    const errorPromise = Promise.reject(makeError({
      error,
      stdout: "",
      stderr: "",
      all: "",
      command,
      escapedCommand,
      parsed,
      timedOut: false,
      isCanceled: false,
      killed: false
    }));
    mergePromise(dummySpawned, errorPromise);
    return dummySpawned;
  }
  const spawnedPromise = getSpawnedPromise(spawned);
  const timedPromise = setupTimeout(spawned, parsed.options, spawnedPromise);
  const processDone = setExitHandler(spawned, parsed.options, timedPromise);
  const context = { isCanceled: false };
  spawned.kill = spawnedKill.bind(null, spawned.kill.bind(spawned));
  spawned.cancel = spawnedCancel.bind(null, spawned, context);
  const handlePromise = async () => {
    const [{ error, exitCode, signal, timedOut }, stdoutResult, stderrResult, allResult] = await getSpawnedResult(spawned, parsed.options, processDone);
    const stdout = handleOutput(parsed.options, stdoutResult);
    const stderr = handleOutput(parsed.options, stderrResult);
    const all = handleOutput(parsed.options, allResult);
    if (error || exitCode !== 0 || signal !== null) {
      const returnedError = makeError({
        error,
        exitCode,
        signal,
        stdout,
        stderr,
        all,
        command,
        escapedCommand,
        parsed,
        timedOut,
        isCanceled: parsed.options.signal ? parsed.options.signal.aborted : false,
        killed: spawned.killed
      });
      if (!parsed.options.reject) {
        return returnedError;
      }
      throw returnedError;
    }
    return {
      command,
      escapedCommand,
      exitCode: 0,
      stdout,
      stderr,
      all,
      failed: false,
      timedOut: false,
      isCanceled: false,
      killed: false
    };
  };
  const handlePromiseOnce = onetime(handlePromise);
  handleInput(spawned, parsed.options);
  spawned.all = makeAllStream(spawned, parsed.options);
  addPipeMethods(spawned);
  mergePromise(spawned, handlePromiseOnce);
  return spawned;
}
function execaCommand(command, options2) {
  const [file2, ...args] = parseCommand(command);
  return execa(file2, args, options2);
}
var Node = class {
  value;
  next;
  constructor(value) {
    this.value = value;
  }
};
var Queue = class {
  #head;
  #tail;
  #size;
  constructor() {
    this.clear();
  }
  enqueue(value) {
    const node = new Node(value);
    if (this.#head) {
      this.#tail.next = node;
      this.#tail = node;
    } else {
      this.#head = node;
      this.#tail = node;
    }
    this.#size++;
  }
  dequeue() {
    const current = this.#head;
    if (!current) {
      return;
    }
    this.#head = this.#head.next;
    this.#size--;
    return current.value;
  }
  clear() {
    this.#head = void 0;
    this.#tail = void 0;
    this.#size = 0;
  }
  get size() {
    return this.#size;
  }
  *[Symbol.iterator]() {
    let current = this.#head;
    while (current) {
      yield current.value;
      current = current.next;
    }
  }
};
function pLimit(concurrency) {
  if (!((Number.isInteger(concurrency) || concurrency === Number.POSITIVE_INFINITY) && concurrency > 0)) {
    throw new TypeError("Expected `concurrency` to be a number from 1 and up");
  }
  const queue = new Queue();
  let activeCount = 0;
  const next = () => {
    activeCount--;
    if (queue.size > 0) {
      queue.dequeue()();
    }
  };
  const run2 = async (fn, resolve3, args) => {
    activeCount++;
    const result = (async () => fn(...args))();
    resolve3(result);
    try {
      await result;
    } catch {
    }
    next();
  };
  const enqueue = (fn, resolve3, args) => {
    queue.enqueue(run2.bind(void 0, fn, resolve3, args));
    (async () => {
      await Promise.resolve();
      if (activeCount < concurrency && queue.size > 0) {
        queue.dequeue()();
      }
    })();
  };
  const generator = (fn, ...args) => new Promise((resolve3) => {
    enqueue(fn, resolve3, args);
  });
  Object.defineProperties(generator, {
    activeCount: {
      get: () => activeCount
    },
    pendingCount: {
      get: () => queue.size
    },
    clearQueue: {
      value: () => {
        queue.clear();
      }
    }
  });
  return generator;
}
var EndError = class extends Error {
  constructor(value) {
    super();
    this.value = value;
  }
};
var testElement = async (element, tester) => tester(await element);
var finder = async (element) => {
  const values = await Promise.all(element);
  if (values[1] === true) {
    throw new EndError(values[0]);
  }
  return false;
};
async function pLocate(iterable, tester, {
  concurrency = Number.POSITIVE_INFINITY,
  preserveOrder = true
} = {}) {
  const limit = pLimit(concurrency);
  const items = [...iterable].map((element) => [element, limit(testElement, element, tester)]);
  const checkLimit = pLimit(preserveOrder ? 1 : Number.POSITIVE_INFINITY);
  try {
    await Promise.all(items.map((element) => checkLimit(finder, element)));
  } catch (error) {
    if (error instanceof EndError) {
      return error.value;
    }
    throw error;
  }
}
var typeMappings2 = {
  directory: "isDirectory",
  file: "isFile"
};
function checkType(type) {
  if (Object.hasOwnProperty.call(typeMappings2, type)) {
    return;
  }
  throw new Error(`Invalid type specified: ${type}`);
}
var matchType = (type, stat) => stat[typeMappings2[type]]();
var toPath$1 = (urlOrPath) => urlOrPath instanceof URL ? (0, import_node_url2.fileURLToPath)(urlOrPath) : urlOrPath;
async function locatePath(paths2, {
  cwd = import_node_process3.default.cwd(),
  type = "file",
  allowSymlinks = true,
  concurrency,
  preserveOrder
} = {}) {
  checkType(type);
  cwd = toPath$1(cwd);
  const statFunction = allowSymlinks ? import_node_fs.promises.stat : import_node_fs.promises.lstat;
  return pLocate(paths2, async (path_) => {
    try {
      const stat = await statFunction(import_node_path2.default.resolve(cwd, path_));
      return matchType(type, stat);
    } catch {
      return false;
    }
  }, { concurrency, preserveOrder });
}
var toPath2 = (urlOrPath) => urlOrPath instanceof URL ? (0, import_node_url2.fileURLToPath)(urlOrPath) : urlOrPath;
var findUpStop = Symbol("findUpStop");
async function findUpMultiple(name, options2 = {}) {
  let directory = import_node_path2.default.resolve(toPath2(options2.cwd) || "");
  const { root } = import_node_path2.default.parse(directory);
  const stopAt = import_node_path2.default.resolve(directory, options2.stopAt || root);
  const limit = options2.limit || Number.POSITIVE_INFINITY;
  const paths2 = [name].flat();
  const runMatcher = async (locateOptions) => {
    if (typeof name !== "function") {
      return locatePath(paths2, locateOptions);
    }
    const foundPath = await name(locateOptions.cwd);
    if (typeof foundPath === "string") {
      return locatePath([foundPath], locateOptions);
    }
    return foundPath;
  };
  const matches = [];
  while (true) {
    const foundPath = await runMatcher({ ...options2, cwd: directory });
    if (foundPath === findUpStop) {
      break;
    }
    if (foundPath) {
      matches.push(import_node_path2.default.resolve(directory, foundPath));
    }
    if (directory === stopAt || matches.length >= limit) {
      break;
    }
    directory = import_node_path2.default.dirname(directory);
  }
  return matches;
}
async function findUp2(name, options2 = {}) {
  const matches = await findUpMultiple(name, { ...options2, limit: 1 });
  return matches[0];
}
var ESC$1 = "\x1B[";
var OSC = "\x1B]";
var BEL = "\x07";
var SEP = ";";
var isTerminalApp = process.env.TERM_PROGRAM === "Apple_Terminal";
var ansiEscapes = {};
ansiEscapes.cursorTo = (x, y) => {
  if (typeof x !== "number") {
    throw new TypeError("The `x` argument is required");
  }
  if (typeof y !== "number") {
    return ESC$1 + (x + 1) + "G";
  }
  return ESC$1 + (y + 1) + ";" + (x + 1) + "H";
};
ansiEscapes.cursorMove = (x, y) => {
  if (typeof x !== "number") {
    throw new TypeError("The `x` argument is required");
  }
  let returnValue = "";
  if (x < 0) {
    returnValue += ESC$1 + -x + "D";
  } else if (x > 0) {
    returnValue += ESC$1 + x + "C";
  }
  if (y < 0) {
    returnValue += ESC$1 + -y + "A";
  } else if (y > 0) {
    returnValue += ESC$1 + y + "B";
  }
  return returnValue;
};
ansiEscapes.cursorUp = (count = 1) => ESC$1 + count + "A";
ansiEscapes.cursorDown = (count = 1) => ESC$1 + count + "B";
ansiEscapes.cursorForward = (count = 1) => ESC$1 + count + "C";
ansiEscapes.cursorBackward = (count = 1) => ESC$1 + count + "D";
ansiEscapes.cursorLeft = ESC$1 + "G";
ansiEscapes.cursorSavePosition = isTerminalApp ? "\x1B7" : ESC$1 + "s";
ansiEscapes.cursorRestorePosition = isTerminalApp ? "\x1B8" : ESC$1 + "u";
ansiEscapes.cursorGetPosition = ESC$1 + "6n";
ansiEscapes.cursorNextLine = ESC$1 + "E";
ansiEscapes.cursorPrevLine = ESC$1 + "F";
ansiEscapes.cursorHide = ESC$1 + "?25l";
ansiEscapes.cursorShow = ESC$1 + "?25h";
ansiEscapes.eraseLines = (count) => {
  let clear2 = "";
  for (let i = 0; i < count; i++) {
    clear2 += ansiEscapes.eraseLine + (i < count - 1 ? ansiEscapes.cursorUp() : "");
  }
  if (count) {
    clear2 += ansiEscapes.cursorLeft;
  }
  return clear2;
};
ansiEscapes.eraseEndLine = ESC$1 + "K";
ansiEscapes.eraseStartLine = ESC$1 + "1K";
ansiEscapes.eraseLine = ESC$1 + "2K";
ansiEscapes.eraseDown = ESC$1 + "J";
ansiEscapes.eraseUp = ESC$1 + "1J";
ansiEscapes.eraseScreen = ESC$1 + "2J";
ansiEscapes.scrollUp = ESC$1 + "S";
ansiEscapes.scrollDown = ESC$1 + "T";
ansiEscapes.clearScreen = "\x1Bc";
ansiEscapes.clearTerminal = process.platform === "win32" ? `${ansiEscapes.eraseScreen}${ESC$1}0f` : (
  // 1. Erases the screen (Only done in case `2` is not supported)
  // 2. Erases the whole screen including scrollback buffer
  // 3. Moves cursor to the top-left position
  // More info: https://www.real-world-systems.com/docs/ANSIcode.html
  `${ansiEscapes.eraseScreen}${ESC$1}3J${ESC$1}H`
);
ansiEscapes.beep = BEL;
ansiEscapes.link = (text2, url2) => {
  return [
    OSC,
    "8",
    SEP,
    SEP,
    url2,
    BEL,
    text2,
    OSC,
    "8",
    SEP,
    SEP,
    BEL
  ].join("");
};
ansiEscapes.image = (buffer, options2 = {}) => {
  let returnValue = `${OSC}1337;File=inline=1`;
  if (options2.width) {
    returnValue += `;width=${options2.width}`;
  }
  if (options2.height) {
    returnValue += `;height=${options2.height}`;
  }
  if (options2.preserveAspectRatio === false) {
    returnValue += ";preserveAspectRatio=0";
  }
  return returnValue + ":" + buffer.toString("base64") + BEL;
};
ansiEscapes.iTerm = {
  setCwd: (cwd = process.cwd()) => `${OSC}50;CurrentDir=${cwd}${BEL}`,
  annotation: (message, options2 = {}) => {
    let returnValue = `${OSC}1337;`;
    const hasX = typeof options2.x !== "undefined";
    const hasY = typeof options2.y !== "undefined";
    if ((hasX || hasY) && !(hasX && hasY && typeof options2.length !== "undefined")) {
      throw new Error("`x`, `y` and `length` must be defined when `x` or `y` is defined");
    }
    message = message.replace(/\|/g, "");
    returnValue += options2.isHidden ? "AddHiddenAnnotation=" : "AddAnnotation=";
    if (options2.length > 0) {
      returnValue += (hasX ? [message, options2.length, options2.x, options2.y] : [options2.length, message]).join("|");
    } else {
      returnValue += message;
    }
    return returnValue + BEL;
  }
};
var hasFlag$2 = (flag, argv = process.argv) => {
  const prefix = flag.startsWith("-") ? "" : flag.length === 1 ? "-" : "--";
  const position = argv.indexOf(prefix + flag);
  const terminatorPosition = argv.indexOf("--");
  return position !== -1 && (terminatorPosition === -1 || position < terminatorPosition);
};
var os = import_os.default;
var tty = import_tty.default;
var hasFlag$1 = hasFlag$2;
var { env } = process;
var forceColor;
if (hasFlag$1("no-color") || hasFlag$1("no-colors") || hasFlag$1("color=false") || hasFlag$1("color=never")) {
  forceColor = 0;
} else if (hasFlag$1("color") || hasFlag$1("colors") || hasFlag$1("color=true") || hasFlag$1("color=always")) {
  forceColor = 1;
}
if ("FORCE_COLOR" in env) {
  if (env.FORCE_COLOR === "true") {
    forceColor = 1;
  } else if (env.FORCE_COLOR === "false") {
    forceColor = 0;
  } else {
    forceColor = env.FORCE_COLOR.length === 0 ? 1 : Math.min(parseInt(env.FORCE_COLOR, 10), 3);
  }
}
function translateLevel(level) {
  if (level === 0) {
    return false;
  }
  return {
    level,
    hasBasic: true,
    has256: level >= 2,
    has16m: level >= 3
  };
}
function supportsColor$1(haveStream, streamIsTTY) {
  if (forceColor === 0) {
    return 0;
  }
  if (hasFlag$1("color=16m") || hasFlag$1("color=full") || hasFlag$1("color=truecolor")) {
    return 3;
  }
  if (hasFlag$1("color=256")) {
    return 2;
  }
  if (haveStream && !streamIsTTY && forceColor === void 0) {
    return 0;
  }
  const min = forceColor || 0;
  if (env.TERM === "dumb") {
    return min;
  }
  if (process.platform === "win32") {
    const osRelease = os.release().split(".");
    if (Number(osRelease[0]) >= 10 && Number(osRelease[2]) >= 10586) {
      return Number(osRelease[2]) >= 14931 ? 3 : 2;
    }
    return 1;
  }
  if ("CI" in env) {
    if (["TRAVIS", "CIRCLECI", "APPVEYOR", "GITLAB_CI", "GITHUB_ACTIONS", "BUILDKITE"].some((sign) => sign in env) || env.CI_NAME === "codeship") {
      return 1;
    }
    return min;
  }
  if ("TEAMCITY_VERSION" in env) {
    return /^(9\.(0*[1-9]\d*)\.|\d{2,}\.)/.test(env.TEAMCITY_VERSION) ? 1 : 0;
  }
  if (env.COLORTERM === "truecolor") {
    return 3;
  }
  if ("TERM_PROGRAM" in env) {
    const version2 = parseInt((env.TERM_PROGRAM_VERSION || "").split(".")[0], 10);
    switch (env.TERM_PROGRAM) {
      case "iTerm.app":
        return version2 >= 3 ? 3 : 2;
      case "Apple_Terminal":
        return 2;
    }
  }
  if (/-256(color)?$/i.test(env.TERM)) {
    return 2;
  }
  if (/^screen|^xterm|^vt100|^vt220|^rxvt|color|ansi|cygwin|linux/i.test(env.TERM)) {
    return 1;
  }
  if ("COLORTERM" in env) {
    return 1;
  }
  return min;
}
function getSupportLevel(stream) {
  const level = supportsColor$1(stream, stream && stream.isTTY);
  return translateLevel(level);
}
var supportsColor_1 = {
  supportsColor: getSupportLevel,
  stdout: translateLevel(supportsColor$1(true, tty.isatty(1))),
  stderr: translateLevel(supportsColor$1(true, tty.isatty(2)))
};
var supportsColor = supportsColor_1;
var hasFlag2 = hasFlag$2;
function parseVersion(versionString) {
  if (/^\d{3,4}$/.test(versionString)) {
    const m = /(\d{1,2})(\d{2})/.exec(versionString);
    return {
      major: 0,
      minor: parseInt(m[1], 10),
      patch: parseInt(m[2], 10)
    };
  }
  const versions = (versionString || "").split(".").map((n) => parseInt(n, 10));
  return {
    major: versions[0],
    minor: versions[1],
    patch: versions[2]
  };
}
function supportsHyperlink(stream) {
  const { env: env2 } = process;
  if ("FORCE_HYPERLINK" in env2) {
    return !(env2.FORCE_HYPERLINK.length > 0 && parseInt(env2.FORCE_HYPERLINK, 10) === 0);
  }
  if (hasFlag2("no-hyperlink") || hasFlag2("no-hyperlinks") || hasFlag2("hyperlink=false") || hasFlag2("hyperlink=never")) {
    return false;
  }
  if (hasFlag2("hyperlink=true") || hasFlag2("hyperlink=always")) {
    return true;
  }
  if (!supportsColor.supportsColor(stream)) {
    return false;
  }
  if (stream && !stream.isTTY) {
    return false;
  }
  if (process.platform === "win32") {
    return false;
  }
  if ("NETLIFY" in env2) {
    return true;
  }
  if ("CI" in env2) {
    return false;
  }
  if ("TEAMCITY_VERSION" in env2) {
    return false;
  }
  if ("TERM_PROGRAM" in env2) {
    const version2 = parseVersion(env2.TERM_PROGRAM_VERSION);
    switch (env2.TERM_PROGRAM) {
      case "iTerm.app":
        if (version2.major === 3) {
          return version2.minor >= 1;
        }
        return version2.major > 3;
    }
  }
  if ("VTE_VERSION" in env2) {
    if (env2.VTE_VERSION === "0.50.0") {
      return false;
    }
    const version2 = parseVersion(env2.VTE_VERSION);
    return version2.major > 0 || version2.minor >= 50;
  }
  return false;
}
var supportsHyperlinks = {
  supportsHyperlink,
  stdout: supportsHyperlink(process.stdout),
  stderr: supportsHyperlink(process.stderr)
};
var supportsHyperlinks$1 = /* @__PURE__ */ getDefaultExportFromCjs(supportsHyperlinks);
function terminalLink(text2, url2, { target = "stdout", ...options2 } = {}) {
  if (!supportsHyperlinks$1[target]) {
    if (options2.fallback === false) {
      return text2;
    }
    return typeof options2.fallback === "function" ? options2.fallback(text2, url2) : `${text2} (\u200B${url2}\u200B)`;
  }
  return ansiEscapes.link(text2, url2);
}
terminalLink.isSupported = supportsHyperlinks$1.stdout;
terminalLink.stderr = (text2, url2, options2 = {}) => terminalLink(text2, url2, { target: "stderr", ...options2 });
terminalLink.stderr.isSupported = supportsHyperlinks$1.stderr;
var prompts$3 = {};
var FORCE_COLOR$1;
var NODE_DISABLE_COLORS$1;
var NO_COLOR$1;
var TERM$1;
var isTTY$1 = true;
if (typeof process !== "undefined") {
  ({ FORCE_COLOR: FORCE_COLOR$1, NODE_DISABLE_COLORS: NODE_DISABLE_COLORS$1, NO_COLOR: NO_COLOR$1, TERM: TERM$1 } = process.env || {});
  isTTY$1 = process.stdout && process.stdout.isTTY;
}
var $$1 = {
  enabled: !NODE_DISABLE_COLORS$1 && NO_COLOR$1 == null && TERM$1 !== "dumb" && (FORCE_COLOR$1 != null && FORCE_COLOR$1 !== "0" || isTTY$1),
  // modifiers
  reset: init$1(0, 0),
  bold: init$1(1, 22),
  dim: init$1(2, 22),
  italic: init$1(3, 23),
  underline: init$1(4, 24),
  inverse: init$1(7, 27),
  hidden: init$1(8, 28),
  strikethrough: init$1(9, 29),
  // colors
  black: init$1(30, 39),
  red: init$1(31, 39),
  green: init$1(32, 39),
  yellow: init$1(33, 39),
  blue: init$1(34, 39),
  magenta: init$1(35, 39),
  cyan: init$1(36, 39),
  white: init$1(37, 39),
  gray: init$1(90, 39),
  grey: init$1(90, 39),
  // background colors
  bgBlack: init$1(40, 49),
  bgRed: init$1(41, 49),
  bgGreen: init$1(42, 49),
  bgYellow: init$1(43, 49),
  bgBlue: init$1(44, 49),
  bgMagenta: init$1(45, 49),
  bgCyan: init$1(46, 49),
  bgWhite: init$1(47, 49)
};
function run$2(arr, str) {
  let i = 0, tmp, beg = "", end = "";
  for (; i < arr.length; i++) {
    tmp = arr[i];
    beg += tmp.open;
    end += tmp.close;
    if (!!~str.indexOf(tmp.close)) {
      str = str.replace(tmp.rgx, tmp.close + tmp.open);
    }
  }
  return beg + str + end;
}
function chain$1(has, keys) {
  let ctx = { has, keys };
  ctx.reset = $$1.reset.bind(ctx);
  ctx.bold = $$1.bold.bind(ctx);
  ctx.dim = $$1.dim.bind(ctx);
  ctx.italic = $$1.italic.bind(ctx);
  ctx.underline = $$1.underline.bind(ctx);
  ctx.inverse = $$1.inverse.bind(ctx);
  ctx.hidden = $$1.hidden.bind(ctx);
  ctx.strikethrough = $$1.strikethrough.bind(ctx);
  ctx.black = $$1.black.bind(ctx);
  ctx.red = $$1.red.bind(ctx);
  ctx.green = $$1.green.bind(ctx);
  ctx.yellow = $$1.yellow.bind(ctx);
  ctx.blue = $$1.blue.bind(ctx);
  ctx.magenta = $$1.magenta.bind(ctx);
  ctx.cyan = $$1.cyan.bind(ctx);
  ctx.white = $$1.white.bind(ctx);
  ctx.gray = $$1.gray.bind(ctx);
  ctx.grey = $$1.grey.bind(ctx);
  ctx.bgBlack = $$1.bgBlack.bind(ctx);
  ctx.bgRed = $$1.bgRed.bind(ctx);
  ctx.bgGreen = $$1.bgGreen.bind(ctx);
  ctx.bgYellow = $$1.bgYellow.bind(ctx);
  ctx.bgBlue = $$1.bgBlue.bind(ctx);
  ctx.bgMagenta = $$1.bgMagenta.bind(ctx);
  ctx.bgCyan = $$1.bgCyan.bind(ctx);
  ctx.bgWhite = $$1.bgWhite.bind(ctx);
  return ctx;
}
function init$1(open, close) {
  let blk = {
    open: `\x1B[${open}m`,
    close: `\x1B[${close}m`,
    rgx: new RegExp(`\\x1b\\[${close}m`, "g")
  };
  return function(txt) {
    if (this !== void 0 && this.has !== void 0) {
      !!~this.has.indexOf(open) || (this.has.push(open), this.keys.push(blk));
      return txt === void 0 ? this : $$1.enabled ? run$2(this.keys, txt + "") : txt + "";
    }
    return txt === void 0 ? chain$1([open], [blk]) : $$1.enabled ? run$2([blk], txt + "") : txt + "";
  };
}
var kleur = $$1;
var action$1 = (key, isSelect) => {
  if (key.meta && key.name !== "escape") return;
  if (key.ctrl) {
    if (key.name === "a") return "first";
    if (key.name === "c") return "abort";
    if (key.name === "d") return "abort";
    if (key.name === "e") return "last";
    if (key.name === "g") return "reset";
    if (key.name === "n") return "down";
    if (key.name === "p") return "up";
    return;
  }
  if (isSelect) {
    if (key.name === "j") return "down";
    if (key.name === "k") return "up";
  }
  if (key.name === "return") return "submit";
  if (key.name === "enter") return "submit";
  if (key.name === "backspace") return "delete";
  if (key.name === "delete") return "deleteForward";
  if (key.name === "abort") return "abort";
  if (key.name === "escape") return "exit";
  if (key.name === "tab") return "next";
  if (key.name === "pagedown") return "nextPage";
  if (key.name === "pageup") return "prevPage";
  if (key.name === "home") return "home";
  if (key.name === "end") return "end";
  if (key.name === "up") return "up";
  if (key.name === "down") return "down";
  if (key.name === "right") return "right";
  if (key.name === "left") return "left";
  return false;
};
var strip$2 = (str) => {
  const pattern = [
    "[\\u001B\\u009B][[\\]()#;?]*(?:(?:(?:(?:;[-a-zA-Z\\d\\/#&.:=?%@~_]+)*|[a-zA-Z\\d]+(?:;[-a-zA-Z\\d\\/#&.:=?%@~_]*)*)?\\u0007)",
    "(?:(?:\\d{1,4}(?:;\\d{0,4})*)?[\\dA-PRZcf-ntqry=><~]))"
  ].join("|");
  const RGX = new RegExp(pattern, "g");
  return typeof str === "string" ? str.replace(RGX, "") : str;
};
var ESC = "\x1B";
var CSI = `${ESC}[`;
var beep$1 = "\x07";
var cursor$b = {
  to(x, y) {
    if (!y) return `${CSI}${x + 1}G`;
    return `${CSI}${y + 1};${x + 1}H`;
  },
  move(x, y) {
    let ret = "";
    if (x < 0) ret += `${CSI}${-x}D`;
    else if (x > 0) ret += `${CSI}${x}C`;
    if (y < 0) ret += `${CSI}${-y}A`;
    else if (y > 0) ret += `${CSI}${y}B`;
    return ret;
  },
  up: (count = 1) => `${CSI}${count}A`,
  down: (count = 1) => `${CSI}${count}B`,
  forward: (count = 1) => `${CSI}${count}C`,
  backward: (count = 1) => `${CSI}${count}D`,
  nextLine: (count = 1) => `${CSI}E`.repeat(count),
  prevLine: (count = 1) => `${CSI}F`.repeat(count),
  left: `${CSI}G`,
  hide: `${CSI}?25l`,
  show: `${CSI}?25h`,
  save: `${ESC}7`,
  restore: `${ESC}8`
};
var scroll = {
  up: (count = 1) => `${CSI}S`.repeat(count),
  down: (count = 1) => `${CSI}T`.repeat(count)
};
var erase$7 = {
  screen: `${CSI}2J`,
  up: (count = 1) => `${CSI}1J`.repeat(count),
  down: (count = 1) => `${CSI}J`.repeat(count),
  line: `${CSI}2K`,
  lineEnd: `${CSI}K`,
  lineStart: `${CSI}1K`,
  lines(count) {
    let clear2 = "";
    for (let i = 0; i < count; i++)
      clear2 += this.line + (i < count - 1 ? cursor$b.up() : "");
    if (count)
      clear2 += cursor$b.left;
    return clear2;
  }
};
var src = { cursor: cursor$b, scroll, erase: erase$7, beep: beep$1 };
var strip$1 = strip$2;
var { erase: erase$6, cursor: cursor$a } = src;
var width = (str) => [...strip$1(str)].length;
var clear$9 = function(prompt2, perLine) {
  if (!perLine) return erase$6.line + cursor$a.to(0);
  let rows = 0;
  const lines2 = prompt2.split(/\r?\n/);
  for (let line of lines2) {
    rows += 1 + Math.floor(Math.max(width(line) - 1, 0) / perLine);
  }
  return erase$6.lines(rows);
};
var main = {
  arrowUp: "\u2191",
  arrowDown: "\u2193",
  arrowLeft: "\u2190",
  arrowRight: "\u2192",
  radioOn: "\u25C9",
  radioOff: "\u25EF",
  tick: "\u2714",
  cross: "\u2716",
  ellipsis: "\u2026",
  pointerSmall: "\u203A",
  line: "\u2500",
  pointer: "\u276F"
};
var win = {
  arrowUp: main.arrowUp,
  arrowDown: main.arrowDown,
  arrowLeft: main.arrowLeft,
  arrowRight: main.arrowRight,
  radioOn: "(*)",
  radioOff: "( )",
  tick: "\u221A",
  cross: "\xD7",
  ellipsis: "...",
  pointerSmall: "\xBB",
  line: "\u2500",
  pointer: ">"
};
var figures$8 = process.platform === "win32" ? win : main;
var figures_1 = figures$8;
var c = kleur;
var figures$7 = figures_1;
var styles = Object.freeze({
  password: { scale: 1, render: (input) => "*".repeat(input.length) },
  emoji: { scale: 2, render: (input) => "\u{1F603}".repeat(input.length) },
  invisible: { scale: 0, render: (input) => "" },
  default: { scale: 1, render: (input) => `${input}` }
});
var render = (type) => styles[type] || styles.default;
var symbols = Object.freeze({
  aborted: c.red(figures$7.cross),
  done: c.green(figures$7.tick),
  exited: c.yellow(figures$7.cross),
  default: c.cyan("?")
});
var symbol = (done, aborted, exited) => aborted ? symbols.aborted : exited ? symbols.exited : done ? symbols.done : symbols.default;
var delimiter$1 = (completing) => c.gray(completing ? figures$7.ellipsis : figures$7.pointerSmall);
var item = (expandable, expanded) => c.gray(expandable ? expanded ? figures$7.pointerSmall : "+" : figures$7.line);
var style$9 = {
  styles,
  render,
  symbols,
  symbol,
  delimiter: delimiter$1,
  item
};
var strip = strip$2;
var lines$2 = function(msg, perLine) {
  let lines2 = String(strip(msg) || "").split(/\r?\n/);
  if (!perLine) return lines2.length;
  return lines2.map((l) => Math.ceil(l.length / perLine)).reduce((a, b) => a + b);
};
var wrap$3 = (msg, opts = {}) => {
  const tab = Number.isSafeInteger(parseInt(opts.margin)) ? new Array(parseInt(opts.margin)).fill(" ").join("") : opts.margin || "";
  const width2 = opts.width;
  return (msg || "").split(/\r?\n/g).map((line) => line.split(/\s+/g).reduce((arr, w) => {
    if (w.length + tab.length >= width2 || arr[arr.length - 1].length + w.length + 1 < width2)
      arr[arr.length - 1] += ` ${w}`;
    else arr.push(`${tab}${w}`);
    return arr;
  }, [tab]).join("\n")).join("\n");
};
var entriesToDisplay$3 = (cursor2, total, maxVisible) => {
  maxVisible = maxVisible || total;
  let startIndex = Math.min(total - maxVisible, cursor2 - Math.floor(maxVisible / 2));
  if (startIndex < 0) startIndex = 0;
  let endIndex = Math.min(startIndex + maxVisible, total);
  return { startIndex, endIndex };
};
var util = {
  action: action$1,
  clear: clear$9,
  style: style$9,
  strip: strip$2,
  figures: figures_1,
  lines: lines$2,
  wrap: wrap$3,
  entriesToDisplay: entriesToDisplay$3
};
var readline2 = import_readline.default;
var { action } = util;
var EventEmitter = import_events.default;
var { beep, cursor: cursor$9 } = src;
var color$9 = kleur;
var Prompt$8 = class Prompt extends EventEmitter {
  constructor(opts = {}) {
    super();
    this.firstRender = true;
    this.in = opts.stdin || process.stdin;
    this.out = opts.stdout || process.stdout;
    this.onRender = (opts.onRender || (() => void 0)).bind(this);
    const rl = readline2.createInterface({ input: this.in, escapeCodeTimeout: 50 });
    readline2.emitKeypressEvents(this.in, rl);
    if (this.in.isTTY) this.in.setRawMode(true);
    const isSelect = ["SelectPrompt", "MultiselectPrompt"].indexOf(this.constructor.name) > -1;
    const keypress = (str, key) => {
      let a = action(key, isSelect);
      if (a === false) {
        this._ && this._(str, key);
      } else if (typeof this[a] === "function") {
        this[a](key);
      } else {
        this.bell();
      }
    };
    this.close = () => {
      this.out.write(cursor$9.show);
      this.in.removeListener("keypress", keypress);
      if (this.in.isTTY) this.in.setRawMode(false);
      rl.close();
      this.emit(this.aborted ? "abort" : this.exited ? "exit" : "submit", this.value);
      this.closed = true;
    };
    this.in.on("keypress", keypress);
  }
  fire() {
    this.emit("state", {
      value: this.value,
      aborted: !!this.aborted,
      exited: !!this.exited
    });
  }
  bell() {
    this.out.write(beep);
  }
  render() {
    this.onRender(color$9);
    if (this.firstRender) this.firstRender = false;
  }
};
var prompt$1 = Prompt$8;
var color$8 = kleur;
var Prompt$7 = prompt$1;
var { erase: erase$5, cursor: cursor$8 } = src;
var { style: style$8, clear: clear$8, lines: lines$1, figures: figures$6 } = util;
var TextPrompt = class extends Prompt$7 {
  constructor(opts = {}) {
    super(opts);
    this.transform = style$8.render(opts.style);
    this.scale = this.transform.scale;
    this.msg = opts.message;
    this.initial = opts.initial || ``;
    this.validator = opts.validate || (() => true);
    this.value = ``;
    this.errorMsg = opts.error || `Please Enter A Valid Value`;
    this.cursor = Number(!!this.initial);
    this.cursorOffset = 0;
    this.clear = clear$8(``, this.out.columns);
    this.render();
  }
  set value(v) {
    if (!v && this.initial) {
      this.placeholder = true;
      this.rendered = color$8.gray(this.transform.render(this.initial));
    } else {
      this.placeholder = false;
      this.rendered = this.transform.render(v);
    }
    this._value = v;
    this.fire();
  }
  get value() {
    return this._value;
  }
  reset() {
    this.value = ``;
    this.cursor = Number(!!this.initial);
    this.cursorOffset = 0;
    this.fire();
    this.render();
  }
  exit() {
    this.abort();
  }
  abort() {
    this.value = this.value || this.initial;
    this.done = this.aborted = true;
    this.error = false;
    this.red = false;
    this.fire();
    this.render();
    this.out.write("\n");
    this.close();
  }
  async validate() {
    let valid = await this.validator(this.value);
    if (typeof valid === `string`) {
      this.errorMsg = valid;
      valid = false;
    }
    this.error = !valid;
  }
  async submit() {
    this.value = this.value || this.initial;
    this.cursorOffset = 0;
    this.cursor = this.rendered.length;
    await this.validate();
    if (this.error) {
      this.red = true;
      this.fire();
      this.render();
      return;
    }
    this.done = true;
    this.aborted = false;
    this.fire();
    this.render();
    this.out.write("\n");
    this.close();
  }
  next() {
    if (!this.placeholder) return this.bell();
    this.value = this.initial;
    this.cursor = this.rendered.length;
    this.fire();
    this.render();
  }
  moveCursor(n) {
    if (this.placeholder) return;
    this.cursor = this.cursor + n;
    this.cursorOffset += n;
  }
  _(c2, key) {
    let s1 = this.value.slice(0, this.cursor);
    let s2 = this.value.slice(this.cursor);
    this.value = `${s1}${c2}${s2}`;
    this.red = false;
    this.cursor = this.placeholder ? 0 : s1.length + 1;
    this.render();
  }
  delete() {
    if (this.isCursorAtStart()) return this.bell();
    let s1 = this.value.slice(0, this.cursor - 1);
    let s2 = this.value.slice(this.cursor);
    this.value = `${s1}${s2}`;
    this.red = false;
    if (this.isCursorAtStart()) {
      this.cursorOffset = 0;
    } else {
      this.cursorOffset++;
      this.moveCursor(-1);
    }
    this.render();
  }
  deleteForward() {
    if (this.cursor * this.scale >= this.rendered.length || this.placeholder) return this.bell();
    let s1 = this.value.slice(0, this.cursor);
    let s2 = this.value.slice(this.cursor + 1);
    this.value = `${s1}${s2}`;
    this.red = false;
    if (this.isCursorAtEnd()) {
      this.cursorOffset = 0;
    } else {
      this.cursorOffset++;
    }
    this.render();
  }
  first() {
    this.cursor = 0;
    this.render();
  }
  last() {
    this.cursor = this.value.length;
    this.render();
  }
  left() {
    if (this.cursor <= 0 || this.placeholder) return this.bell();
    this.moveCursor(-1);
    this.render();
  }
  right() {
    if (this.cursor * this.scale >= this.rendered.length || this.placeholder) return this.bell();
    this.moveCursor(1);
    this.render();
  }
  isCursorAtStart() {
    return this.cursor === 0 || this.placeholder && this.cursor === 1;
  }
  isCursorAtEnd() {
    return this.cursor === this.rendered.length || this.placeholder && this.cursor === this.rendered.length + 1;
  }
  render() {
    if (this.closed) return;
    if (!this.firstRender) {
      if (this.outputError)
        this.out.write(cursor$8.down(lines$1(this.outputError, this.out.columns) - 1) + clear$8(this.outputError, this.out.columns));
      this.out.write(clear$8(this.outputText, this.out.columns));
    }
    super.render();
    this.outputError = "";
    this.outputText = [
      style$8.symbol(this.done, this.aborted),
      color$8.bold(this.msg),
      style$8.delimiter(this.done),
      this.red ? color$8.red(this.rendered) : this.rendered
    ].join(` `);
    if (this.error) {
      this.outputError += this.errorMsg.split(`
`).reduce((a, l, i) => a + `
${i ? " " : figures$6.pointerSmall} ${color$8.red().italic(l)}`, ``);
    }
    this.out.write(erase$5.line + cursor$8.to(0) + this.outputText + cursor$8.save + this.outputError + cursor$8.restore + cursor$8.move(this.cursorOffset, 0));
  }
};
var text = TextPrompt;
var color$7 = kleur;
var Prompt$6 = prompt$1;
var { style: style$7, clear: clear$7, figures: figures$5, wrap: wrap$2, entriesToDisplay: entriesToDisplay$2 } = util;
var { cursor: cursor$7 } = src;
var SelectPrompt = class extends Prompt$6 {
  constructor(opts = {}) {
    super(opts);
    this.msg = opts.message;
    this.hint = opts.hint || "- Use arrow-keys. Return to submit.";
    this.warn = opts.warn || "- This option is disabled";
    this.cursor = opts.initial || 0;
    this.choices = opts.choices.map((ch, idx) => {
      if (typeof ch === "string")
        ch = { title: ch, value: idx };
      return {
        title: ch && (ch.title || ch.value || ch),
        value: ch && (ch.value === void 0 ? idx : ch.value),
        description: ch && ch.description,
        selected: ch && ch.selected,
        disabled: ch && ch.disabled
      };
    });
    this.optionsPerPage = opts.optionsPerPage || 10;
    this.value = (this.choices[this.cursor] || {}).value;
    this.clear = clear$7("", this.out.columns);
    this.render();
  }
  moveCursor(n) {
    this.cursor = n;
    this.value = this.choices[n].value;
    this.fire();
  }
  reset() {
    this.moveCursor(0);
    this.fire();
    this.render();
  }
  exit() {
    this.abort();
  }
  abort() {
    this.done = this.aborted = true;
    this.fire();
    this.render();
    this.out.write("\n");
    this.close();
  }
  submit() {
    if (!this.selection.disabled) {
      this.done = true;
      this.aborted = false;
      this.fire();
      this.render();
      this.out.write("\n");
      this.close();
    } else
      this.bell();
  }
  first() {
    this.moveCursor(0);
    this.render();
  }
  last() {
    this.moveCursor(this.choices.length - 1);
    this.render();
  }
  up() {
    if (this.cursor === 0) {
      this.moveCursor(this.choices.length - 1);
    } else {
      this.moveCursor(this.cursor - 1);
    }
    this.render();
  }
  down() {
    if (this.cursor === this.choices.length - 1) {
      this.moveCursor(0);
    } else {
      this.moveCursor(this.cursor + 1);
    }
    this.render();
  }
  next() {
    this.moveCursor((this.cursor + 1) % this.choices.length);
    this.render();
  }
  _(c2, key) {
    if (c2 === " ") return this.submit();
  }
  get selection() {
    return this.choices[this.cursor];
  }
  render() {
    if (this.closed) return;
    if (this.firstRender) this.out.write(cursor$7.hide);
    else this.out.write(clear$7(this.outputText, this.out.columns));
    super.render();
    let { startIndex, endIndex } = entriesToDisplay$2(this.cursor, this.choices.length, this.optionsPerPage);
    this.outputText = [
      style$7.symbol(this.done, this.aborted),
      color$7.bold(this.msg),
      style$7.delimiter(false),
      this.done ? this.selection.title : this.selection.disabled ? color$7.yellow(this.warn) : color$7.gray(this.hint)
    ].join(" ");
    if (!this.done) {
      this.outputText += "\n";
      for (let i = startIndex; i < endIndex; i++) {
        let title, prefix, desc = "", v = this.choices[i];
        if (i === startIndex && startIndex > 0) {
          prefix = figures$5.arrowUp;
        } else if (i === endIndex - 1 && endIndex < this.choices.length) {
          prefix = figures$5.arrowDown;
        } else {
          prefix = " ";
        }
        if (v.disabled) {
          title = this.cursor === i ? color$7.gray().underline(v.title) : color$7.strikethrough().gray(v.title);
          prefix = (this.cursor === i ? color$7.bold().gray(figures$5.pointer) + " " : "  ") + prefix;
        } else {
          title = this.cursor === i ? color$7.cyan().underline(v.title) : v.title;
          prefix = (this.cursor === i ? color$7.cyan(figures$5.pointer) + " " : "  ") + prefix;
          if (v.description && this.cursor === i) {
            desc = ` - ${v.description}`;
            if (prefix.length + title.length + desc.length >= this.out.columns || v.description.split(/\r?\n/).length > 1) {
              desc = "\n" + wrap$2(v.description, { margin: 3, width: this.out.columns });
            }
          }
        }
        this.outputText += `${prefix} ${title}${color$7.gray(desc)}
`;
      }
    }
    this.out.write(this.outputText);
  }
};
var select = SelectPrompt;
var color$6 = kleur;
var Prompt$5 = prompt$1;
var { style: style$6, clear: clear$6 } = util;
var { cursor: cursor$6, erase: erase$4 } = src;
var TogglePrompt = class extends Prompt$5 {
  constructor(opts = {}) {
    super(opts);
    this.msg = opts.message;
    this.value = !!opts.initial;
    this.active = opts.active || "on";
    this.inactive = opts.inactive || "off";
    this.initialValue = this.value;
    this.render();
  }
  reset() {
    this.value = this.initialValue;
    this.fire();
    this.render();
  }
  exit() {
    this.abort();
  }
  abort() {
    this.done = this.aborted = true;
    this.fire();
    this.render();
    this.out.write("\n");
    this.close();
  }
  submit() {
    this.done = true;
    this.aborted = false;
    this.fire();
    this.render();
    this.out.write("\n");
    this.close();
  }
  deactivate() {
    if (this.value === false) return this.bell();
    this.value = false;
    this.render();
  }
  activate() {
    if (this.value === true) return this.bell();
    this.value = true;
    this.render();
  }
  delete() {
    this.deactivate();
  }
  left() {
    this.deactivate();
  }
  right() {
    this.activate();
  }
  down() {
    this.deactivate();
  }
  up() {
    this.activate();
  }
  next() {
    this.value = !this.value;
    this.fire();
    this.render();
  }
  _(c2, key) {
    if (c2 === " ") {
      this.value = !this.value;
    } else if (c2 === "1") {
      this.value = true;
    } else if (c2 === "0") {
      this.value = false;
    } else return this.bell();
    this.render();
  }
  render() {
    if (this.closed) return;
    if (this.firstRender) this.out.write(cursor$6.hide);
    else this.out.write(clear$6(this.outputText, this.out.columns));
    super.render();
    this.outputText = [
      style$6.symbol(this.done, this.aborted),
      color$6.bold(this.msg),
      style$6.delimiter(this.done),
      this.value ? this.inactive : color$6.cyan().underline(this.inactive),
      color$6.gray("/"),
      this.value ? color$6.cyan().underline(this.active) : this.active
    ].join(" ");
    this.out.write(erase$4.line + cursor$6.to(0) + this.outputText);
  }
};
var toggle = TogglePrompt;
var DatePart$9 = class DatePart {
  constructor({ token, date: date3, parts, locales }) {
    this.token = token;
    this.date = date3 || /* @__PURE__ */ new Date();
    this.parts = parts || [this];
    this.locales = locales || {};
  }
  up() {
  }
  down() {
  }
  next() {
    const currentIdx = this.parts.indexOf(this);
    return this.parts.find((part, idx) => idx > currentIdx && part instanceof DatePart);
  }
  setTo(val) {
  }
  prev() {
    let parts = [].concat(this.parts).reverse();
    const currentIdx = parts.indexOf(this);
    return parts.find((part, idx) => idx > currentIdx && part instanceof DatePart);
  }
  toString() {
    return String(this.date);
  }
};
var datepart = DatePart$9;
var DatePart$8 = datepart;
var Meridiem$1 = class Meridiem extends DatePart$8 {
  constructor(opts = {}) {
    super(opts);
  }
  up() {
    this.date.setHours((this.date.getHours() + 12) % 24);
  }
  down() {
    this.up();
  }
  toString() {
    let meridiem2 = this.date.getHours() > 12 ? "pm" : "am";
    return /\A/.test(this.token) ? meridiem2.toUpperCase() : meridiem2;
  }
};
var meridiem = Meridiem$1;
var DatePart$7 = datepart;
var pos = (n) => {
  n = n % 10;
  return n === 1 ? "st" : n === 2 ? "nd" : n === 3 ? "rd" : "th";
};
var Day$1 = class Day extends DatePart$7 {
  constructor(opts = {}) {
    super(opts);
  }
  up() {
    this.date.setDate(this.date.getDate() + 1);
  }
  down() {
    this.date.setDate(this.date.getDate() - 1);
  }
  setTo(val) {
    this.date.setDate(parseInt(val.substr(-2)));
  }
  toString() {
    let date3 = this.date.getDate();
    let day2 = this.date.getDay();
    return this.token === "DD" ? String(date3).padStart(2, "0") : this.token === "Do" ? date3 + pos(date3) : this.token === "d" ? day2 + 1 : this.token === "ddd" ? this.locales.weekdaysShort[day2] : this.token === "dddd" ? this.locales.weekdays[day2] : date3;
  }
};
var day = Day$1;
var DatePart$6 = datepart;
var Hours$1 = class Hours extends DatePart$6 {
  constructor(opts = {}) {
    super(opts);
  }
  up() {
    this.date.setHours(this.date.getHours() + 1);
  }
  down() {
    this.date.setHours(this.date.getHours() - 1);
  }
  setTo(val) {
    this.date.setHours(parseInt(val.substr(-2)));
  }
  toString() {
    let hours2 = this.date.getHours();
    if (/h/.test(this.token))
      hours2 = hours2 % 12 || 12;
    return this.token.length > 1 ? String(hours2).padStart(2, "0") : hours2;
  }
};
var hours = Hours$1;
var DatePart$5 = datepart;
var Milliseconds$1 = class Milliseconds extends DatePart$5 {
  constructor(opts = {}) {
    super(opts);
  }
  up() {
    this.date.setMilliseconds(this.date.getMilliseconds() + 1);
  }
  down() {
    this.date.setMilliseconds(this.date.getMilliseconds() - 1);
  }
  setTo(val) {
    this.date.setMilliseconds(parseInt(val.substr(-this.token.length)));
  }
  toString() {
    return String(this.date.getMilliseconds()).padStart(4, "0").substr(0, this.token.length);
  }
};
var milliseconds = Milliseconds$1;
var DatePart$4 = datepart;
var Minutes$1 = class Minutes extends DatePart$4 {
  constructor(opts = {}) {
    super(opts);
  }
  up() {
    this.date.setMinutes(this.date.getMinutes() + 1);
  }
  down() {
    this.date.setMinutes(this.date.getMinutes() - 1);
  }
  setTo(val) {
    this.date.setMinutes(parseInt(val.substr(-2)));
  }
  toString() {
    let m = this.date.getMinutes();
    return this.token.length > 1 ? String(m).padStart(2, "0") : m;
  }
};
var minutes = Minutes$1;
var DatePart$3 = datepart;
var Month$1 = class Month extends DatePart$3 {
  constructor(opts = {}) {
    super(opts);
  }
  up() {
    this.date.setMonth(this.date.getMonth() + 1);
  }
  down() {
    this.date.setMonth(this.date.getMonth() - 1);
  }
  setTo(val) {
    val = parseInt(val.substr(-2)) - 1;
    this.date.setMonth(val < 0 ? 0 : val);
  }
  toString() {
    let month2 = this.date.getMonth();
    let tl = this.token.length;
    return tl === 2 ? String(month2 + 1).padStart(2, "0") : tl === 3 ? this.locales.monthsShort[month2] : tl === 4 ? this.locales.months[month2] : String(month2 + 1);
  }
};
var month = Month$1;
var DatePart$2 = datepart;
var Seconds$1 = class Seconds extends DatePart$2 {
  constructor(opts = {}) {
    super(opts);
  }
  up() {
    this.date.setSeconds(this.date.getSeconds() + 1);
  }
  down() {
    this.date.setSeconds(this.date.getSeconds() - 1);
  }
  setTo(val) {
    this.date.setSeconds(parseInt(val.substr(-2)));
  }
  toString() {
    let s = this.date.getSeconds();
    return this.token.length > 1 ? String(s).padStart(2, "0") : s;
  }
};
var seconds = Seconds$1;
var DatePart$1 = datepart;
var Year$1 = class Year extends DatePart$1 {
  constructor(opts = {}) {
    super(opts);
  }
  up() {
    this.date.setFullYear(this.date.getFullYear() + 1);
  }
  down() {
    this.date.setFullYear(this.date.getFullYear() - 1);
  }
  setTo(val) {
    this.date.setFullYear(val.substr(-4));
  }
  toString() {
    let year2 = String(this.date.getFullYear()).padStart(4, "0");
    return this.token.length === 2 ? year2.substr(-2) : year2;
  }
};
var year = Year$1;
var dateparts = {
  DatePart: datepart,
  Meridiem: meridiem,
  Day: day,
  Hours: hours,
  Milliseconds: milliseconds,
  Minutes: minutes,
  Month: month,
  Seconds: seconds,
  Year: year
};
var color$5 = kleur;
var Prompt$4 = prompt$1;
var { style: style$5, clear: clear$5, figures: figures$4 } = util;
var { erase: erase$3, cursor: cursor$5 } = src;
var { DatePart: DatePart2, Meridiem: Meridiem2, Day: Day2, Hours: Hours2, Milliseconds: Milliseconds2, Minutes: Minutes2, Month: Month2, Seconds: Seconds2, Year: Year2 } = dateparts;
var regex = /\\(.)|"((?:\\["\\]|[^"])+)"|(D[Do]?|d{3,4}|d)|(M{1,4})|(YY(?:YY)?)|([aA])|([Hh]{1,2})|(m{1,2})|(s{1,2})|(S{1,4})|./g;
var regexGroups = {
  1: ({ token }) => token.replace(/\\(.)/g, "$1"),
  2: (opts) => new Day2(opts),
  // Day // TODO
  3: (opts) => new Month2(opts),
  // Month
  4: (opts) => new Year2(opts),
  // Year
  5: (opts) => new Meridiem2(opts),
  // AM/PM // TODO (special)
  6: (opts) => new Hours2(opts),
  // Hours
  7: (opts) => new Minutes2(opts),
  // Minutes
  8: (opts) => new Seconds2(opts),
  // Seconds
  9: (opts) => new Milliseconds2(opts)
  // Fractional seconds
};
var dfltLocales = {
  months: "January,February,March,April,May,June,July,August,September,October,November,December".split(","),
  monthsShort: "Jan,Feb,Mar,Apr,May,Jun,Jul,Aug,Sep,Oct,Nov,Dec".split(","),
  weekdays: "Sunday,Monday,Tuesday,Wednesday,Thursday,Friday,Saturday".split(","),
  weekdaysShort: "Sun,Mon,Tue,Wed,Thu,Fri,Sat".split(",")
};
var DatePrompt = class extends Prompt$4 {
  constructor(opts = {}) {
    super(opts);
    this.msg = opts.message;
    this.cursor = 0;
    this.typed = "";
    this.locales = Object.assign(dfltLocales, opts.locales);
    this._date = opts.initial || /* @__PURE__ */ new Date();
    this.errorMsg = opts.error || "Please Enter A Valid Value";
    this.validator = opts.validate || (() => true);
    this.mask = opts.mask || "YYYY-MM-DD HH:mm:ss";
    this.clear = clear$5("", this.out.columns);
    this.render();
  }
  get value() {
    return this.date;
  }
  get date() {
    return this._date;
  }
  set date(date3) {
    if (date3) this._date.setTime(date3.getTime());
  }
  set mask(mask) {
    let result;
    this.parts = [];
    while (result = regex.exec(mask)) {
      let match = result.shift();
      let idx = result.findIndex((gr) => gr != null);
      this.parts.push(idx in regexGroups ? regexGroups[idx]({ token: result[idx] || match, date: this.date, parts: this.parts, locales: this.locales }) : result[idx] || match);
    }
    let parts = this.parts.reduce((arr, i) => {
      if (typeof i === "string" && typeof arr[arr.length - 1] === "string")
        arr[arr.length - 1] += i;
      else arr.push(i);
      return arr;
    }, []);
    this.parts.splice(0);
    this.parts.push(...parts);
    this.reset();
  }
  moveCursor(n) {
    this.typed = "";
    this.cursor = n;
    this.fire();
  }
  reset() {
    this.moveCursor(this.parts.findIndex((p) => p instanceof DatePart2));
    this.fire();
    this.render();
  }
  exit() {
    this.abort();
  }
  abort() {
    this.done = this.aborted = true;
    this.error = false;
    this.fire();
    this.render();
    this.out.write("\n");
    this.close();
  }
  async validate() {
    let valid = await this.validator(this.value);
    if (typeof valid === "string") {
      this.errorMsg = valid;
      valid = false;
    }
    this.error = !valid;
  }
  async submit() {
    await this.validate();
    if (this.error) {
      this.color = "red";
      this.fire();
      this.render();
      return;
    }
    this.done = true;
    this.aborted = false;
    this.fire();
    this.render();
    this.out.write("\n");
    this.close();
  }
  up() {
    this.typed = "";
    this.parts[this.cursor].up();
    this.render();
  }
  down() {
    this.typed = "";
    this.parts[this.cursor].down();
    this.render();
  }
  left() {
    let prev = this.parts[this.cursor].prev();
    if (prev == null) return this.bell();
    this.moveCursor(this.parts.indexOf(prev));
    this.render();
  }
  right() {
    let next = this.parts[this.cursor].next();
    if (next == null) return this.bell();
    this.moveCursor(this.parts.indexOf(next));
    this.render();
  }
  next() {
    let next = this.parts[this.cursor].next();
    this.moveCursor(next ? this.parts.indexOf(next) : this.parts.findIndex((part) => part instanceof DatePart2));
    this.render();
  }
  _(c2) {
    if (/\d/.test(c2)) {
      this.typed += c2;
      this.parts[this.cursor].setTo(this.typed);
      this.render();
    }
  }
  render() {
    if (this.closed) return;
    if (this.firstRender) this.out.write(cursor$5.hide);
    else this.out.write(clear$5(this.outputText, this.out.columns));
    super.render();
    this.outputText = [
      style$5.symbol(this.done, this.aborted),
      color$5.bold(this.msg),
      style$5.delimiter(false),
      this.parts.reduce((arr, p, idx) => arr.concat(idx === this.cursor && !this.done ? color$5.cyan().underline(p.toString()) : p), []).join("")
    ].join(" ");
    if (this.error) {
      this.outputText += this.errorMsg.split("\n").reduce(
        (a, l, i) => a + `
${i ? ` ` : figures$4.pointerSmall} ${color$5.red().italic(l)}`,
        ``
      );
    }
    this.out.write(erase$3.line + cursor$5.to(0) + this.outputText);
  }
};
var date2 = DatePrompt;
var color$4 = kleur;
var Prompt$3 = prompt$1;
var { cursor: cursor$4, erase: erase$2 } = src;
var { style: style$4, figures: figures$3, clear: clear$4, lines } = util;
var isNumber = /[0-9]/;
var isDef = (any) => any !== void 0;
var round = (number2, precision) => {
  let factor = Math.pow(10, precision);
  return Math.round(number2 * factor) / factor;
};
var NumberPrompt = class extends Prompt$3 {
  constructor(opts = {}) {
    super(opts);
    this.transform = style$4.render(opts.style);
    this.msg = opts.message;
    this.initial = isDef(opts.initial) ? opts.initial : "";
    this.float = !!opts.float;
    this.round = opts.round || 2;
    this.inc = opts.increment || 1;
    this.min = isDef(opts.min) ? opts.min : -Infinity;
    this.max = isDef(opts.max) ? opts.max : Infinity;
    this.errorMsg = opts.error || `Please Enter A Valid Value`;
    this.validator = opts.validate || (() => true);
    this.color = `cyan`;
    this.value = ``;
    this.typed = ``;
    this.lastHit = 0;
    this.render();
  }
  set value(v) {
    if (!v && v !== 0) {
      this.placeholder = true;
      this.rendered = color$4.gray(this.transform.render(`${this.initial}`));
      this._value = ``;
    } else {
      this.placeholder = false;
      this.rendered = this.transform.render(`${round(v, this.round)}`);
      this._value = round(v, this.round);
    }
    this.fire();
  }
  get value() {
    return this._value;
  }
  parse(x) {
    return this.float ? parseFloat(x) : parseInt(x);
  }
  valid(c2) {
    return c2 === `-` || c2 === `.` && this.float || isNumber.test(c2);
  }
  reset() {
    this.typed = ``;
    this.value = ``;
    this.fire();
    this.render();
  }
  exit() {
    this.abort();
  }
  abort() {
    let x = this.value;
    this.value = x !== `` ? x : this.initial;
    this.done = this.aborted = true;
    this.error = false;
    this.fire();
    this.render();
    this.out.write(`
`);
    this.close();
  }
  async validate() {
    let valid = await this.validator(this.value);
    if (typeof valid === `string`) {
      this.errorMsg = valid;
      valid = false;
    }
    this.error = !valid;
  }
  async submit() {
    await this.validate();
    if (this.error) {
      this.color = `red`;
      this.fire();
      this.render();
      return;
    }
    let x = this.value;
    this.value = x !== `` ? x : this.initial;
    this.done = true;
    this.aborted = false;
    this.error = false;
    this.fire();
    this.render();
    this.out.write(`
`);
    this.close();
  }
  up() {
    this.typed = ``;
    if (this.value === "") {
      this.value = this.min - this.inc;
    }
    if (this.value >= this.max) return this.bell();
    this.value += this.inc;
    this.color = `cyan`;
    this.fire();
    this.render();
  }
  down() {
    this.typed = ``;
    if (this.value === "") {
      this.value = this.min + this.inc;
    }
    if (this.value <= this.min) return this.bell();
    this.value -= this.inc;
    this.color = `cyan`;
    this.fire();
    this.render();
  }
  delete() {
    let val = this.value.toString();
    if (val.length === 0) return this.bell();
    this.value = this.parse(val = val.slice(0, -1)) || ``;
    if (this.value !== "" && this.value < this.min) {
      this.value = this.min;
    }
    this.color = `cyan`;
    this.fire();
    this.render();
  }
  next() {
    this.value = this.initial;
    this.fire();
    this.render();
  }
  _(c2, key) {
    if (!this.valid(c2)) return this.bell();
    const now = Date.now();
    if (now - this.lastHit > 1e3) this.typed = ``;
    this.typed += c2;
    this.lastHit = now;
    this.color = `cyan`;
    if (c2 === `.`) return this.fire();
    this.value = Math.min(this.parse(this.typed), this.max);
    if (this.value > this.max) this.value = this.max;
    if (this.value < this.min) this.value = this.min;
    this.fire();
    this.render();
  }
  render() {
    if (this.closed) return;
    if (!this.firstRender) {
      if (this.outputError)
        this.out.write(cursor$4.down(lines(this.outputError, this.out.columns) - 1) + clear$4(this.outputError, this.out.columns));
      this.out.write(clear$4(this.outputText, this.out.columns));
    }
    super.render();
    this.outputError = "";
    this.outputText = [
      style$4.symbol(this.done, this.aborted),
      color$4.bold(this.msg),
      style$4.delimiter(this.done),
      !this.done || !this.done && !this.placeholder ? color$4[this.color]().underline(this.rendered) : this.rendered
    ].join(` `);
    if (this.error) {
      this.outputError += this.errorMsg.split(`
`).reduce((a, l, i) => a + `
${i ? ` ` : figures$3.pointerSmall} ${color$4.red().italic(l)}`, ``);
    }
    this.out.write(erase$2.line + cursor$4.to(0) + this.outputText + cursor$4.save + this.outputError + cursor$4.restore);
  }
};
var number = NumberPrompt;
var color$3 = kleur;
var { cursor: cursor$3 } = src;
var Prompt$2 = prompt$1;
var { clear: clear$3, figures: figures$2, style: style$3, wrap: wrap$1, entriesToDisplay: entriesToDisplay$1 } = util;
var MultiselectPrompt$1 = class MultiselectPrompt extends Prompt$2 {
  constructor(opts = {}) {
    super(opts);
    this.msg = opts.message;
    this.cursor = opts.cursor || 0;
    this.scrollIndex = opts.cursor || 0;
    this.hint = opts.hint || "";
    this.warn = opts.warn || "- This option is disabled -";
    this.minSelected = opts.min;
    this.showMinError = false;
    this.maxChoices = opts.max;
    this.instructions = opts.instructions;
    this.optionsPerPage = opts.optionsPerPage || 10;
    this.value = opts.choices.map((ch, idx) => {
      if (typeof ch === "string")
        ch = { title: ch, value: idx };
      return {
        title: ch && (ch.title || ch.value || ch),
        description: ch && ch.description,
        value: ch && (ch.value === void 0 ? idx : ch.value),
        selected: ch && ch.selected,
        disabled: ch && ch.disabled
      };
    });
    this.clear = clear$3("", this.out.columns);
    if (!opts.overrideRender) {
      this.render();
    }
  }
  reset() {
    this.value.map((v) => !v.selected);
    this.cursor = 0;
    this.fire();
    this.render();
  }
  selected() {
    return this.value.filter((v) => v.selected);
  }
  exit() {
    this.abort();
  }
  abort() {
    this.done = this.aborted = true;
    this.fire();
    this.render();
    this.out.write("\n");
    this.close();
  }
  submit() {
    const selected = this.value.filter((e) => e.selected);
    if (this.minSelected && selected.length < this.minSelected) {
      this.showMinError = true;
      this.render();
    } else {
      this.done = true;
      this.aborted = false;
      this.fire();
      this.render();
      this.out.write("\n");
      this.close();
    }
  }
  first() {
    this.cursor = 0;
    this.render();
  }
  last() {
    this.cursor = this.value.length - 1;
    this.render();
  }
  next() {
    this.cursor = (this.cursor + 1) % this.value.length;
    this.render();
  }
  up() {
    if (this.cursor === 0) {
      this.cursor = this.value.length - 1;
    } else {
      this.cursor--;
    }
    this.render();
  }
  down() {
    if (this.cursor === this.value.length - 1) {
      this.cursor = 0;
    } else {
      this.cursor++;
    }
    this.render();
  }
  left() {
    this.value[this.cursor].selected = false;
    this.render();
  }
  right() {
    if (this.value.filter((e) => e.selected).length >= this.maxChoices) return this.bell();
    this.value[this.cursor].selected = true;
    this.render();
  }
  handleSpaceToggle() {
    const v = this.value[this.cursor];
    if (v.selected) {
      v.selected = false;
      this.render();
    } else if (v.disabled || this.value.filter((e) => e.selected).length >= this.maxChoices) {
      return this.bell();
    } else {
      v.selected = true;
      this.render();
    }
  }
  toggleAll() {
    if (this.maxChoices !== void 0 || this.value[this.cursor].disabled) {
      return this.bell();
    }
    const newSelected = !this.value[this.cursor].selected;
    this.value.filter((v) => !v.disabled).forEach((v) => v.selected = newSelected);
    this.render();
  }
  _(c2, key) {
    if (c2 === " ") {
      this.handleSpaceToggle();
    } else if (c2 === "a") {
      this.toggleAll();
    } else {
      return this.bell();
    }
  }
  renderInstructions() {
    if (this.instructions === void 0 || this.instructions) {
      if (typeof this.instructions === "string") {
        return this.instructions;
      }
      return `
Instructions:
    ${figures$2.arrowUp}/${figures$2.arrowDown}: Highlight option
    ${figures$2.arrowLeft}/${figures$2.arrowRight}/[space]: Toggle selection
` + (this.maxChoices === void 0 ? `    a: Toggle all
` : "") + `    enter/return: Complete answer`;
    }
    return "";
  }
  renderOption(cursor2, v, i, arrowIndicator) {
    const prefix = (v.selected ? color$3.green(figures$2.radioOn) : figures$2.radioOff) + " " + arrowIndicator + " ";
    let title, desc;
    if (v.disabled) {
      title = cursor2 === i ? color$3.gray().underline(v.title) : color$3.strikethrough().gray(v.title);
    } else {
      title = cursor2 === i ? color$3.cyan().underline(v.title) : v.title;
      if (cursor2 === i && v.description) {
        desc = ` - ${v.description}`;
        if (prefix.length + title.length + desc.length >= this.out.columns || v.description.split(/\r?\n/).length > 1) {
          desc = "\n" + wrap$1(v.description, { margin: prefix.length, width: this.out.columns });
        }
      }
    }
    return prefix + title + color$3.gray(desc || "");
  }
  // shared with autocompleteMultiselect
  paginateOptions(options2) {
    if (options2.length === 0) {
      return color$3.red("No matches for this query.");
    }
    let { startIndex, endIndex } = entriesToDisplay$1(this.cursor, options2.length, this.optionsPerPage);
    let prefix, styledOptions = [];
    for (let i = startIndex; i < endIndex; i++) {
      if (i === startIndex && startIndex > 0) {
        prefix = figures$2.arrowUp;
      } else if (i === endIndex - 1 && endIndex < options2.length) {
        prefix = figures$2.arrowDown;
      } else {
        prefix = " ";
      }
      styledOptions.push(this.renderOption(this.cursor, options2[i], i, prefix));
    }
    return "\n" + styledOptions.join("\n");
  }
  // shared with autocomleteMultiselect
  renderOptions(options2) {
    if (!this.done) {
      return this.paginateOptions(options2);
    }
    return "";
  }
  renderDoneOrInstructions() {
    if (this.done) {
      return this.value.filter((e) => e.selected).map((v) => v.title).join(", ");
    }
    const output = [color$3.gray(this.hint), this.renderInstructions()];
    if (this.value[this.cursor].disabled) {
      output.push(color$3.yellow(this.warn));
    }
    return output.join(" ");
  }
  render() {
    if (this.closed) return;
    if (this.firstRender) this.out.write(cursor$3.hide);
    super.render();
    let prompt2 = [
      style$3.symbol(this.done, this.aborted),
      color$3.bold(this.msg),
      style$3.delimiter(false),
      this.renderDoneOrInstructions()
    ].join(" ");
    if (this.showMinError) {
      prompt2 += color$3.red(`You must select a minimum of ${this.minSelected} choices.`);
      this.showMinError = false;
    }
    prompt2 += this.renderOptions(this.value);
    this.out.write(this.clear + prompt2);
    this.clear = clear$3(prompt2, this.out.columns);
  }
};
var multiselect = MultiselectPrompt$1;
var color$2 = kleur;
var Prompt$1 = prompt$1;
var { erase: erase$1, cursor: cursor$2 } = src;
var { style: style$2, clear: clear$2, figures: figures$1, wrap, entriesToDisplay } = util;
var getVal = (arr, i) => arr[i] && (arr[i].value || arr[i].title || arr[i]);
var getTitle = (arr, i) => arr[i] && (arr[i].title || arr[i].value || arr[i]);
var getIndex = (arr, valOrTitle) => {
  const index = arr.findIndex((el) => el.value === valOrTitle || el.title === valOrTitle);
  return index > -1 ? index : void 0;
};
var AutocompletePrompt = class extends Prompt$1 {
  constructor(opts = {}) {
    super(opts);
    this.msg = opts.message;
    this.suggest = opts.suggest;
    this.choices = opts.choices;
    this.initial = typeof opts.initial === "number" ? opts.initial : getIndex(opts.choices, opts.initial);
    this.select = this.initial || opts.cursor || 0;
    this.i18n = { noMatches: opts.noMatches || "no matches found" };
    this.fallback = opts.fallback || this.initial;
    this.clearFirst = opts.clearFirst || false;
    this.suggestions = [];
    this.input = "";
    this.limit = opts.limit || 10;
    this.cursor = 0;
    this.transform = style$2.render(opts.style);
    this.scale = this.transform.scale;
    this.render = this.render.bind(this);
    this.complete = this.complete.bind(this);
    this.clear = clear$2("", this.out.columns);
    this.complete(this.render);
    this.render();
  }
  set fallback(fb) {
    this._fb = Number.isSafeInteger(parseInt(fb)) ? parseInt(fb) : fb;
  }
  get fallback() {
    let choice;
    if (typeof this._fb === "number")
      choice = this.choices[this._fb];
    else if (typeof this._fb === "string")
      choice = { title: this._fb };
    return choice || this._fb || { title: this.i18n.noMatches };
  }
  moveSelect(i) {
    this.select = i;
    if (this.suggestions.length > 0)
      this.value = getVal(this.suggestions, i);
    else this.value = this.fallback.value;
    this.fire();
  }
  async complete(cb) {
    const p = this.completing = this.suggest(this.input, this.choices);
    const suggestions = await p;
    if (this.completing !== p) return;
    this.suggestions = suggestions.map((s, i, arr) => ({ title: getTitle(arr, i), value: getVal(arr, i), description: s.description }));
    this.completing = false;
    const l = Math.max(suggestions.length - 1, 0);
    this.moveSelect(Math.min(l, this.select));
    cb && cb();
  }
  reset() {
    this.input = "";
    this.complete(() => {
      this.moveSelect(this.initial !== void 0 ? this.initial : 0);
      this.render();
    });
    this.render();
  }
  exit() {
    if (this.clearFirst && this.input.length > 0) {
      this.reset();
    } else {
      this.done = this.exited = true;
      this.aborted = false;
      this.fire();
      this.render();
      this.out.write("\n");
      this.close();
    }
  }
  abort() {
    this.done = this.aborted = true;
    this.exited = false;
    this.fire();
    this.render();
    this.out.write("\n");
    this.close();
  }
  submit() {
    this.done = true;
    this.aborted = this.exited = false;
    this.fire();
    this.render();
    this.out.write("\n");
    this.close();
  }
  _(c2, key) {
    let s1 = this.input.slice(0, this.cursor);
    let s2 = this.input.slice(this.cursor);
    this.input = `${s1}${c2}${s2}`;
    this.cursor = s1.length + 1;
    this.complete(this.render);
    this.render();
  }
  delete() {
    if (this.cursor === 0) return this.bell();
    let s1 = this.input.slice(0, this.cursor - 1);
    let s2 = this.input.slice(this.cursor);
    this.input = `${s1}${s2}`;
    this.complete(this.render);
    this.cursor = this.cursor - 1;
    this.render();
  }
  deleteForward() {
    if (this.cursor * this.scale >= this.rendered.length) return this.bell();
    let s1 = this.input.slice(0, this.cursor);
    let s2 = this.input.slice(this.cursor + 1);
    this.input = `${s1}${s2}`;
    this.complete(this.render);
    this.render();
  }
  first() {
    this.moveSelect(0);
    this.render();
  }
  last() {
    this.moveSelect(this.suggestions.length - 1);
    this.render();
  }
  up() {
    if (this.select === 0) {
      this.moveSelect(this.suggestions.length - 1);
    } else {
      this.moveSelect(this.select - 1);
    }
    this.render();
  }
  down() {
    if (this.select === this.suggestions.length - 1) {
      this.moveSelect(0);
    } else {
      this.moveSelect(this.select + 1);
    }
    this.render();
  }
  next() {
    if (this.select === this.suggestions.length - 1) {
      this.moveSelect(0);
    } else this.moveSelect(this.select + 1);
    this.render();
  }
  nextPage() {
    this.moveSelect(Math.min(this.select + this.limit, this.suggestions.length - 1));
    this.render();
  }
  prevPage() {
    this.moveSelect(Math.max(this.select - this.limit, 0));
    this.render();
  }
  left() {
    if (this.cursor <= 0) return this.bell();
    this.cursor = this.cursor - 1;
    this.render();
  }
  right() {
    if (this.cursor * this.scale >= this.rendered.length) return this.bell();
    this.cursor = this.cursor + 1;
    this.render();
  }
  renderOption(v, hovered, isStart, isEnd) {
    let desc;
    let prefix = isStart ? figures$1.arrowUp : isEnd ? figures$1.arrowDown : " ";
    let title = hovered ? color$2.cyan().underline(v.title) : v.title;
    prefix = (hovered ? color$2.cyan(figures$1.pointer) + " " : "  ") + prefix;
    if (v.description) {
      desc = ` - ${v.description}`;
      if (prefix.length + title.length + desc.length >= this.out.columns || v.description.split(/\r?\n/).length > 1) {
        desc = "\n" + wrap(v.description, { margin: 3, width: this.out.columns });
      }
    }
    return prefix + " " + title + color$2.gray(desc || "");
  }
  render() {
    if (this.closed) return;
    if (this.firstRender) this.out.write(cursor$2.hide);
    else this.out.write(clear$2(this.outputText, this.out.columns));
    super.render();
    let { startIndex, endIndex } = entriesToDisplay(this.select, this.choices.length, this.limit);
    this.outputText = [
      style$2.symbol(this.done, this.aborted, this.exited),
      color$2.bold(this.msg),
      style$2.delimiter(this.completing),
      this.done && this.suggestions[this.select] ? this.suggestions[this.select].title : this.rendered = this.transform.render(this.input)
    ].join(" ");
    if (!this.done) {
      const suggestions = this.suggestions.slice(startIndex, endIndex).map((item2, i) => this.renderOption(
        item2,
        this.select === i + startIndex,
        i === 0 && startIndex > 0,
        i + startIndex === endIndex - 1 && endIndex < this.choices.length
      )).join("\n");
      this.outputText += `
` + (suggestions || color$2.gray(this.fallback.title));
    }
    this.out.write(erase$1.line + cursor$2.to(0) + this.outputText);
  }
};
var autocomplete = AutocompletePrompt;
var color$1 = kleur;
var { cursor: cursor$1 } = src;
var MultiselectPrompt2 = multiselect;
var { clear: clear$1, style: style$1, figures } = util;
var AutocompleteMultiselectPrompt = class extends MultiselectPrompt2 {
  constructor(opts = {}) {
    opts.overrideRender = true;
    super(opts);
    this.inputValue = "";
    this.clear = clear$1("", this.out.columns);
    this.filteredOptions = this.value;
    this.render();
  }
  last() {
    this.cursor = this.filteredOptions.length - 1;
    this.render();
  }
  next() {
    this.cursor = (this.cursor + 1) % this.filteredOptions.length;
    this.render();
  }
  up() {
    if (this.cursor === 0) {
      this.cursor = this.filteredOptions.length - 1;
    } else {
      this.cursor--;
    }
    this.render();
  }
  down() {
    if (this.cursor === this.filteredOptions.length - 1) {
      this.cursor = 0;
    } else {
      this.cursor++;
    }
    this.render();
  }
  left() {
    this.filteredOptions[this.cursor].selected = false;
    this.render();
  }
  right() {
    if (this.value.filter((e) => e.selected).length >= this.maxChoices) return this.bell();
    this.filteredOptions[this.cursor].selected = true;
    this.render();
  }
  delete() {
    if (this.inputValue.length) {
      this.inputValue = this.inputValue.substr(0, this.inputValue.length - 1);
      this.updateFilteredOptions();
    }
  }
  updateFilteredOptions() {
    const currentHighlight = this.filteredOptions[this.cursor];
    this.filteredOptions = this.value.filter((v) => {
      if (this.inputValue) {
        if (typeof v.title === "string") {
          if (v.title.toLowerCase().includes(this.inputValue.toLowerCase())) {
            return true;
          }
        }
        if (typeof v.value === "string") {
          if (v.value.toLowerCase().includes(this.inputValue.toLowerCase())) {
            return true;
          }
        }
        return false;
      }
      return true;
    });
    const newHighlightIndex = this.filteredOptions.findIndex((v) => v === currentHighlight);
    this.cursor = newHighlightIndex < 0 ? 0 : newHighlightIndex;
    this.render();
  }
  handleSpaceToggle() {
    const v = this.filteredOptions[this.cursor];
    if (v.selected) {
      v.selected = false;
      this.render();
    } else if (v.disabled || this.value.filter((e) => e.selected).length >= this.maxChoices) {
      return this.bell();
    } else {
      v.selected = true;
      this.render();
    }
  }
  handleInputChange(c2) {
    this.inputValue = this.inputValue + c2;
    this.updateFilteredOptions();
  }
  _(c2, key) {
    if (c2 === " ") {
      this.handleSpaceToggle();
    } else {
      this.handleInputChange(c2);
    }
  }
  renderInstructions() {
    if (this.instructions === void 0 || this.instructions) {
      if (typeof this.instructions === "string") {
        return this.instructions;
      }
      return `
Instructions:
    ${figures.arrowUp}/${figures.arrowDown}: Highlight option
    ${figures.arrowLeft}/${figures.arrowRight}/[space]: Toggle selection
    [a,b,c]/delete: Filter choices
    enter/return: Complete answer
`;
    }
    return "";
  }
  renderCurrentInput() {
    return `
Filtered results for: ${this.inputValue ? this.inputValue : color$1.gray("Enter something to filter")}
`;
  }
  renderOption(cursor2, v, i, arrowIndicator) {
    const prefix = (v.selected ? color$1.green(figures.radioOn) : figures.radioOff) + " " + arrowIndicator + " ";
    let title;
    if (v.disabled) title = cursor2 === i ? color$1.gray().underline(v.title) : color$1.strikethrough().gray(v.title);
    else title = cursor2 === i ? color$1.cyan().underline(v.title) : v.title;
    return prefix + title;
  }
  renderDoneOrInstructions() {
    if (this.done) {
      return this.value.filter((e) => e.selected).map((v) => v.title).join(", ");
    }
    const output = [color$1.gray(this.hint), this.renderInstructions(), this.renderCurrentInput()];
    if (this.filteredOptions.length && this.filteredOptions[this.cursor].disabled) {
      output.push(color$1.yellow(this.warn));
    }
    return output.join(" ");
  }
  render() {
    if (this.closed) return;
    if (this.firstRender) this.out.write(cursor$1.hide);
    super.render();
    let prompt2 = [
      style$1.symbol(this.done, this.aborted),
      color$1.bold(this.msg),
      style$1.delimiter(false),
      this.renderDoneOrInstructions()
    ].join(" ");
    if (this.showMinError) {
      prompt2 += color$1.red(`You must select a minimum of ${this.minSelected} choices.`);
      this.showMinError = false;
    }
    prompt2 += this.renderOptions(this.filteredOptions);
    this.out.write(this.clear + prompt2);
    this.clear = clear$1(prompt2, this.out.columns);
  }
};
var autocompleteMultiselect = AutocompleteMultiselectPrompt;
var color = kleur;
var Prompt2 = prompt$1;
var { style, clear } = util;
var { erase, cursor } = src;
var ConfirmPrompt = class extends Prompt2 {
  constructor(opts = {}) {
    super(opts);
    this.msg = opts.message;
    this.value = opts.initial;
    this.initialValue = !!opts.initial;
    this.yesMsg = opts.yes || "yes";
    this.yesOption = opts.yesOption || "(Y/n)";
    this.noMsg = opts.no || "no";
    this.noOption = opts.noOption || "(y/N)";
    this.render();
  }
  reset() {
    this.value = this.initialValue;
    this.fire();
    this.render();
  }
  exit() {
    this.abort();
  }
  abort() {
    this.done = this.aborted = true;
    this.fire();
    this.render();
    this.out.write("\n");
    this.close();
  }
  submit() {
    this.value = this.value || false;
    this.done = true;
    this.aborted = false;
    this.fire();
    this.render();
    this.out.write("\n");
    this.close();
  }
  _(c2, key) {
    if (c2.toLowerCase() === "y") {
      this.value = true;
      return this.submit();
    }
    if (c2.toLowerCase() === "n") {
      this.value = false;
      return this.submit();
    }
    return this.bell();
  }
  render() {
    if (this.closed) return;
    if (this.firstRender) this.out.write(cursor.hide);
    else this.out.write(clear(this.outputText, this.out.columns));
    super.render();
    this.outputText = [
      style.symbol(this.done, this.aborted),
      color.bold(this.msg),
      style.delimiter(this.done),
      this.done ? this.value ? this.yesMsg : this.noMsg : color.gray(this.initialValue ? this.yesOption : this.noOption)
    ].join(" ");
    this.out.write(erase.line + cursor.to(0) + this.outputText);
  }
};
var confirm = ConfirmPrompt;
var elements = {
  TextPrompt: text,
  SelectPrompt: select,
  TogglePrompt: toggle,
  DatePrompt: date2,
  NumberPrompt: number,
  MultiselectPrompt: multiselect,
  AutocompletePrompt: autocomplete,
  AutocompleteMultiselectPrompt: autocompleteMultiselect,
  ConfirmPrompt: confirm
};
(function(exports2) {
  const $3 = exports2;
  const el = elements;
  const noop2 = (v) => v;
  function toPrompt(type, args, opts = {}) {
    return new Promise((res, rej) => {
      const p = new el[type](args);
      const onAbort = opts.onAbort || noop2;
      const onSubmit = opts.onSubmit || noop2;
      const onExit2 = opts.onExit || noop2;
      p.on("state", args.onState || noop2);
      p.on("submit", (x) => res(onSubmit(x)));
      p.on("exit", (x) => res(onExit2(x)));
      p.on("abort", (x) => rej(onAbort(x)));
    });
  }
  $3.text = (args) => toPrompt("TextPrompt", args);
  $3.password = (args) => {
    args.style = "password";
    return $3.text(args);
  };
  $3.invisible = (args) => {
    args.style = "invisible";
    return $3.text(args);
  };
  $3.number = (args) => toPrompt("NumberPrompt", args);
  $3.date = (args) => toPrompt("DatePrompt", args);
  $3.confirm = (args) => toPrompt("ConfirmPrompt", args);
  $3.list = (args) => {
    const sep2 = args.separator || ",";
    return toPrompt("TextPrompt", args, {
      onSubmit: (str) => str.split(sep2).map((s) => s.trim())
    });
  };
  $3.toggle = (args) => toPrompt("TogglePrompt", args);
  $3.select = (args) => toPrompt("SelectPrompt", args);
  $3.multiselect = (args) => {
    args.choices = [].concat(args.choices || []);
    const toSelected = (items) => items.filter((item2) => item2.selected).map((item2) => item2.value);
    return toPrompt("MultiselectPrompt", args, {
      onAbort: toSelected,
      onSubmit: toSelected
    });
  };
  $3.autocompleteMultiselect = (args) => {
    args.choices = [].concat(args.choices || []);
    const toSelected = (items) => items.filter((item2) => item2.selected).map((item2) => item2.value);
    return toPrompt("AutocompleteMultiselectPrompt", args, {
      onAbort: toSelected,
      onSubmit: toSelected
    });
  };
  const byTitle = (input, choices) => Promise.resolve(
    choices.filter((item2) => item2.title.slice(0, input.length).toLowerCase() === input.toLowerCase())
  );
  $3.autocomplete = (args) => {
    args.suggest = args.suggest || byTitle;
    args.choices = [].concat(args.choices || []);
    return toPrompt("AutocompletePrompt", args);
  };
})(prompts$3);
var prompts$2 = prompts$3;
var passOn = ["suggest", "format", "onState", "validate", "onRender", "type"];
var noop = () => {
};
async function prompt(questions = [], { onSubmit = noop, onCancel = noop } = {}) {
  const answers = {};
  const override2 = prompt._override || {};
  questions = [].concat(questions);
  let answer, question, quit, name, type, lastPrompt;
  const getFormattedAnswer = async (question2, answer2, skipValidation = false) => {
    if (!skipValidation && question2.validate && question2.validate(answer2) !== true) {
      return;
    }
    return question2.format ? await question2.format(answer2, answers) : answer2;
  };
  for (question of questions) {
    ({ name, type } = question);
    if (typeof type === "function") {
      type = await type(answer, { ...answers }, question);
      question["type"] = type;
    }
    if (!type) continue;
    for (let key in question) {
      if (passOn.includes(key)) continue;
      let value = question[key];
      question[key] = typeof value === "function" ? await value(answer, { ...answers }, lastPrompt) : value;
    }
    lastPrompt = question;
    if (typeof question.message !== "string") {
      throw new Error("prompt message is required");
    }
    ({ name, type } = question);
    if (prompts$2[type] === void 0) {
      throw new Error(`prompt type (${type}) is not defined`);
    }
    if (override2[question.name] !== void 0) {
      answer = await getFormattedAnswer(question, override2[question.name]);
      if (answer !== void 0) {
        answers[name] = answer;
        continue;
      }
    }
    try {
      answer = prompt._injected ? getInjectedAnswer(prompt._injected, question.initial) : await prompts$2[type](question);
      answers[name] = answer = await getFormattedAnswer(question, answer, true);
      quit = await onSubmit(question, answer, answers);
    } catch (err) {
      quit = !await onCancel(question, answers);
    }
    if (quit) return answers;
  }
  return answers;
}
function getInjectedAnswer(injected, deafultValue) {
  const answer = injected.shift();
  if (answer instanceof Error) {
    throw answer;
  }
  return answer === void 0 ? deafultValue : answer;
}
function inject(answers) {
  prompt._injected = (prompt._injected || []).concat(answers);
}
function override(answers) {
  prompt._override = Object.assign({}, answers);
}
var lib$1 = Object.assign(prompt, { prompt, prompts: prompts$2, inject, override });
var prompts = lib$1;
var prompts$1 = /* @__PURE__ */ getDefaultExportFromCjs(prompts);
var cjs = {};
var posix$1 = {};
Object.defineProperty(posix$1, "__esModule", { value: true });
posix$1.sync = posix$1.isexe = void 0;
var fs_1$1 = import_fs.default;
var promises_1$1 = import_promises4.default;
var isexe$2 = async (path10, options2 = {}) => {
  const { ignoreErrors = false } = options2;
  try {
    return checkStat$1(await (0, promises_1$1.stat)(path10), options2);
  } catch (e) {
    const er = e;
    if (ignoreErrors || er.code === "EACCES")
      return false;
    throw er;
  }
};
posix$1.isexe = isexe$2;
var sync$1 = (path10, options2 = {}) => {
  const { ignoreErrors = false } = options2;
  try {
    return checkStat$1((0, fs_1$1.statSync)(path10), options2);
  } catch (e) {
    const er = e;
    if (ignoreErrors || er.code === "EACCES")
      return false;
    throw er;
  }
};
posix$1.sync = sync$1;
var checkStat$1 = (stat, options2) => stat.isFile() && checkMode(stat, options2);
var checkMode = (stat, options2) => {
  const myUid = options2.uid ?? process.getuid?.();
  const myGroups = options2.groups ?? process.getgroups?.() ?? [];
  const myGid = options2.gid ?? process.getgid?.() ?? myGroups[0];
  if (myUid === void 0 || myGid === void 0) {
    throw new Error("cannot get uid or gid");
  }
  const groups = /* @__PURE__ */ new Set([myGid, ...myGroups]);
  const mod = stat.mode;
  const uid = stat.uid;
  const gid = stat.gid;
  const u = parseInt("100", 8);
  const g = parseInt("010", 8);
  const o = parseInt("001", 8);
  const ug = u | g;
  return !!(mod & o || mod & g && groups.has(gid) || mod & u && uid === myUid || mod & ug && myUid === 0);
};
var win32 = {};
Object.defineProperty(win32, "__esModule", { value: true });
win32.sync = win32.isexe = void 0;
var fs_1 = import_fs.default;
var promises_1 = import_promises4.default;
var isexe$1 = async (path10, options2 = {}) => {
  const { ignoreErrors = false } = options2;
  try {
    return checkStat(await (0, promises_1.stat)(path10), path10, options2);
  } catch (e) {
    const er = e;
    if (ignoreErrors || er.code === "EACCES")
      return false;
    throw er;
  }
};
win32.isexe = isexe$1;
var sync = (path10, options2 = {}) => {
  const { ignoreErrors = false } = options2;
  try {
    return checkStat((0, fs_1.statSync)(path10), path10, options2);
  } catch (e) {
    const er = e;
    if (ignoreErrors || er.code === "EACCES")
      return false;
    throw er;
  }
};
win32.sync = sync;
var checkPathExt = (path10, options2) => {
  const { pathExt = process.env.PATHEXT || "" } = options2;
  const peSplit = pathExt.split(";");
  if (peSplit.indexOf("") !== -1) {
    return true;
  }
  for (let i = 0; i < peSplit.length; i++) {
    const p = peSplit[i].toLowerCase();
    const ext = path10.substring(path10.length - p.length).toLowerCase();
    if (p && ext === p) {
      return true;
    }
  }
  return false;
};
var checkStat = (stat, path10, options2) => stat.isFile() && checkPathExt(path10, options2);
var options = {};
Object.defineProperty(options, "__esModule", { value: true });
(function(exports2) {
  var __createBinding = commonjsGlobal && commonjsGlobal.__createBinding || (Object.create ? function(o, m, k, k2) {
    if (k2 === void 0) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() {
        return m[k];
      } };
    }
    Object.defineProperty(o, k2, desc);
  } : function(o, m, k, k2) {
    if (k2 === void 0) k2 = k;
    o[k2] = m[k];
  });
  var __setModuleDefault = commonjsGlobal && commonjsGlobal.__setModuleDefault || (Object.create ? function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
  } : function(o, v) {
    o["default"] = v;
  });
  var __importStar = commonjsGlobal && commonjsGlobal.__importStar || function(mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) {
      for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    }
    __setModuleDefault(result, mod);
    return result;
  };
  var __exportStar = commonjsGlobal && commonjsGlobal.__exportStar || function(m, exports3) {
    for (var p in m) if (p !== "default" && !Object.prototype.hasOwnProperty.call(exports3, p)) __createBinding(exports3, m, p);
  };
  Object.defineProperty(exports2, "__esModule", { value: true });
  exports2.sync = exports2.isexe = exports2.posix = exports2.win32 = void 0;
  const posix2 = __importStar(posix$1);
  exports2.posix = posix2;
  const win32$1 = __importStar(win32);
  exports2.win32 = win32$1;
  __exportStar(options, exports2);
  const platform = process.env._ISEXE_TEST_PLATFORM_ || process.platform;
  const impl = platform === "win32" ? win32$1 : posix2;
  exports2.isexe = impl.isexe;
  exports2.sync = impl.sync;
})(cjs);
var { isexe, sync: isexeSync } = cjs;
var { join, delimiter, sep, posix } = import_path5.default;
var isWindows = process.platform === "win32";
var rSlash = new RegExp(`[${posix.sep}${sep === posix.sep ? "" : sep}]`.replace(/(\\)/g, "\\$1"));
var rRel = new RegExp(`^\\.${rSlash.source}`);
var getNotFoundError = (cmd) => Object.assign(new Error(`not found: ${cmd}`), { code: "ENOENT" });
var getPathInfo = (cmd, {
  path: optPath = process.env.PATH,
  pathExt: optPathExt = process.env.PATHEXT,
  delimiter: optDelimiter = delimiter
}) => {
  const pathEnv = cmd.match(rSlash) ? [""] : [
    // windows always checks the cwd first
    ...isWindows ? [process.cwd()] : [],
    ...(optPath || /* istanbul ignore next: very unusual */
    "").split(optDelimiter)
  ];
  if (isWindows) {
    const pathExtExe = optPathExt || [".EXE", ".CMD", ".BAT", ".COM"].join(optDelimiter);
    const pathExt = pathExtExe.split(optDelimiter).flatMap((item2) => [item2, item2.toLowerCase()]);
    if (cmd.includes(".") && pathExt[0] !== "") {
      pathExt.unshift("");
    }
    return { pathEnv, pathExt, pathExtExe };
  }
  return { pathEnv, pathExt: [""] };
};
var getPathPart = (raw, cmd) => {
  const pathPart = /^".*"$/.test(raw) ? raw.slice(1, -1) : raw;
  const prefix = !pathPart && rRel.test(cmd) ? cmd.slice(0, 2) : "";
  return prefix + join(pathPart, cmd);
};
var which = async (cmd, opt = {}) => {
  const { pathEnv, pathExt, pathExtExe } = getPathInfo(cmd, opt);
  const found = [];
  for (const envPart of pathEnv) {
    const p = getPathPart(envPart, cmd);
    for (const ext of pathExt) {
      const withExt = p + ext;
      const is = await isexe(withExt, { pathExt: pathExtExe, ignoreErrors: true });
      if (is) {
        if (!opt.all) {
          return withExt;
        }
        found.push(withExt);
      }
    }
  }
  if (opt.all && found.length) {
    return found;
  }
  if (opt.nothrow) {
    return null;
  }
  throw getNotFoundError(cmd);
};
var whichSync = (cmd, opt = {}) => {
  const { pathEnv, pathExt, pathExtExe } = getPathInfo(cmd, opt);
  const found = [];
  for (const pathEnvPart of pathEnv) {
    const p = getPathPart(pathEnvPart, cmd);
    for (const ext of pathExt) {
      const withExt = p + ext;
      const is = isexeSync(withExt, { pathExt: pathExtExe, ignoreErrors: true });
      if (is) {
        if (!opt.all) {
          return withExt;
        }
        found.push(withExt);
      }
    }
  }
  if (opt.all && found.length) {
    return found;
  }
  if (opt.nothrow) {
    return null;
  }
  throw getNotFoundError(cmd);
};
var lib = which;
which.sync = whichSync;
var which$1 = /* @__PURE__ */ getDefaultExportFromCjs(lib);
var CLI_TEMP_DIR = (0, import_node_path2.join)(import_node_os.default.tmpdir(), "antfu-ni");
function cmdExists(cmd) {
  return which$1.sync(cmd, { nothrow: true }) !== null;
}
async function detect({ autoInstall, programmatic, cwd } = {}) {
  let agent = null;
  let version2 = null;
  const lockPath = await findUp2(Object.keys(LOCKS), { cwd });
  let packageJsonPath;
  if (lockPath)
    packageJsonPath = import_node_path2.default.resolve(lockPath, "../package.json");
  else
    packageJsonPath = await findUp2("package.json", { cwd });
  if (packageJsonPath && import_node_fs.default.existsSync(packageJsonPath)) {
    try {
      const pkg = JSON.parse(import_node_fs.default.readFileSync(packageJsonPath, "utf8"));
      if (typeof pkg.packageManager === "string") {
        const [name, ver] = pkg.packageManager.replace(/^\^/, "").split("@");
        version2 = ver;
        if (name === "yarn" && Number.parseInt(ver) > 1) {
          agent = "yarn@berry";
          version2 = "berry";
        } else if (name === "pnpm" && Number.parseInt(ver) < 7) {
          agent = "pnpm@6";
        } else if (name in AGENTS) {
          agent = name;
        } else if (!programmatic) {
          console.warn("[ni] Unknown packageManager:", pkg.packageManager);
        }
      }
    } catch {
    }
  }
  if (!agent && lockPath)
    agent = LOCKS[import_node_path2.default.basename(lockPath)];
  if (agent && !cmdExists(agent.split("@")[0]) && !programmatic) {
    if (!autoInstall) {
      console.warn(`[ni] Detected ${agent} but it doesn't seem to be installed.
`);
      if (import_node_process3.default.env.CI)
        import_node_process3.default.exit(1);
      const link = terminalLink(agent, INSTALL_PAGE[agent]);
      const { tryInstall } = await prompts$1({
        name: "tryInstall",
        type: "confirm",
        message: `Would you like to globally install ${link}?`
      });
      if (!tryInstall)
        import_node_process3.default.exit(1);
    }
    await execaCommand(`npm i -g ${agent.split("@")[0]}${version2 ? `@${version2}` : ""}`, { stdio: "inherit", cwd });
  }
  return agent;
}
var customRcPath = import_node_process3.default.env.NI_CONFIG_FILE;
var home = import_node_process3.default.platform === "win32" ? import_node_process3.default.env.USERPROFILE : import_node_process3.default.env.HOME;
var defaultRcPath = import_node_path2.default.join(home || "~/", ".nirc");
var UnsupportedCommand = class extends Error {
  constructor({ agent, command }) {
    super(`Command "${command}" is not support by agent "${agent}"`);
  }
};
function getCommand(agent, command, args = []) {
  if (!(agent in AGENTS))
    throw new Error(`Unsupported agent "${agent}"`);
  const c2 = AGENTS[agent][command];
  if (typeof c2 === "function")
    return c2(args);
  if (!c2)
    throw new UnsupportedCommand({ agent, command });
  const quote = (arg) => !arg.startsWith("--") && arg.includes(" ") ? JSON.stringify(arg) : arg;
  return c2.replace("{0}", args.map(quote).join(" ")).trim();
}
var FORCE_COLOR2;
var NODE_DISABLE_COLORS2;
var NO_COLOR2;
var TERM2;
var isTTY2 = true;
if (typeof process !== "undefined") {
  ({ FORCE_COLOR: FORCE_COLOR2, NODE_DISABLE_COLORS: NODE_DISABLE_COLORS2, NO_COLOR: NO_COLOR2, TERM: TERM2 } = process.env || {});
  isTTY2 = process.stdout && process.stdout.isTTY;
}
var $2 = {
  enabled: !NODE_DISABLE_COLORS2 && NO_COLOR2 == null && TERM2 !== "dumb" && (FORCE_COLOR2 != null && FORCE_COLOR2 !== "0" || isTTY2),
  // modifiers
  reset: init2(0, 0),
  bold: init2(1, 22),
  dim: init2(2, 22),
  italic: init2(3, 23),
  underline: init2(4, 24),
  inverse: init2(7, 27),
  hidden: init2(8, 28),
  strikethrough: init2(9, 29),
  // colors
  black: init2(30, 39),
  red: init2(31, 39),
  green: init2(32, 39),
  yellow: init2(33, 39),
  blue: init2(34, 39),
  magenta: init2(35, 39),
  cyan: init2(36, 39),
  white: init2(37, 39),
  gray: init2(90, 39),
  grey: init2(90, 39),
  // background colors
  bgBlack: init2(40, 49),
  bgRed: init2(41, 49),
  bgGreen: init2(42, 49),
  bgYellow: init2(43, 49),
  bgBlue: init2(44, 49),
  bgMagenta: init2(45, 49),
  bgCyan: init2(46, 49),
  bgWhite: init2(47, 49)
};
function run$1(arr, str) {
  let i = 0, tmp, beg = "", end = "";
  for (; i < arr.length; i++) {
    tmp = arr[i];
    beg += tmp.open;
    end += tmp.close;
    if (!!~str.indexOf(tmp.close)) {
      str = str.replace(tmp.rgx, tmp.close + tmp.open);
    }
  }
  return beg + str + end;
}
function chain(has, keys) {
  let ctx = { has, keys };
  ctx.reset = $2.reset.bind(ctx);
  ctx.bold = $2.bold.bind(ctx);
  ctx.dim = $2.dim.bind(ctx);
  ctx.italic = $2.italic.bind(ctx);
  ctx.underline = $2.underline.bind(ctx);
  ctx.inverse = $2.inverse.bind(ctx);
  ctx.hidden = $2.hidden.bind(ctx);
  ctx.strikethrough = $2.strikethrough.bind(ctx);
  ctx.black = $2.black.bind(ctx);
  ctx.red = $2.red.bind(ctx);
  ctx.green = $2.green.bind(ctx);
  ctx.yellow = $2.yellow.bind(ctx);
  ctx.blue = $2.blue.bind(ctx);
  ctx.magenta = $2.magenta.bind(ctx);
  ctx.cyan = $2.cyan.bind(ctx);
  ctx.white = $2.white.bind(ctx);
  ctx.gray = $2.gray.bind(ctx);
  ctx.grey = $2.grey.bind(ctx);
  ctx.bgBlack = $2.bgBlack.bind(ctx);
  ctx.bgRed = $2.bgRed.bind(ctx);
  ctx.bgGreen = $2.bgGreen.bind(ctx);
  ctx.bgYellow = $2.bgYellow.bind(ctx);
  ctx.bgBlue = $2.bgBlue.bind(ctx);
  ctx.bgMagenta = $2.bgMagenta.bind(ctx);
  ctx.bgCyan = $2.bgCyan.bind(ctx);
  ctx.bgWhite = $2.bgWhite.bind(ctx);
  return ctx;
}
function init2(open, close) {
  let blk = {
    open: `\x1B[${open}m`,
    close: `\x1B[${close}m`,
    rgx: new RegExp(`\\x1b\\[${close}m`, "g")
  };
  return function(txt) {
    if (this !== void 0 && this.has !== void 0) {
      !!~this.has.indexOf(open) || (this.has.push(open), this.keys.push(blk));
      return txt === void 0 ? this : $2.enabled ? run$1(this.keys, txt + "") : txt + "";
    }
    return txt === void 0 ? chain([open], [blk]) : $2.enabled ? run$1([blk], txt + "") : txt + "";
  };
}

// ../client-generator-js/src/resolvePrismaClient.ts
var debug = Debug("prisma:generator");
async function resolvePrismaClient(baseDir) {
  const prismaClientDir = await findPrismaClientDir(baseDir);
  debug("baseDir", baseDir);
  if (!prismaClientDir) {
    throw new Error(
      `Could not resolve @prisma/client.
Please try to install it with ${bold(
        green(await getPackageCmd(baseDir, "install", "@prisma/client"))
      )} and rerun ${bold(await getPackageCmd(baseDir, "execute", "prisma generate"))} \u{1F64F}.`
    );
  }
  return prismaClientDir;
}
async function findPrismaClientDir(baseDir) {
  const resolveOpts = { basedir: baseDir, preserveSymlinks: true };
  const cliDir = await resolvePkg("prisma", resolveOpts);
  const clientDir = await resolvePkg("@prisma/client", resolveOpts);
  const resolvedClientDir = clientDir && await import_promises5.default.realpath(clientDir);
  debug("prismaCliDir", cliDir);
  debug("prismaClientDir", clientDir);
  if (cliDir === void 0) return resolvedClientDir;
  if (clientDir === void 0) return resolvedClientDir;
  const relDir = import_node_path3.default.relative(cliDir, clientDir).split(import_node_path3.default.sep);
  if (relDir[0] !== ".." || relDir[1] === "..") return void 0;
  return resolvedClientDir;
}
async function getPackageCmd(cwd, cmd, ...args) {
  const agent = await detect({ cwd, autoInstall: false, programmatic: true });
  return getCommand(agent ?? "npm", cmd, args);
}

// ../client-generator-js/src/generator.ts
var PrismaClientJsGenerator = class {
  name = "prisma-client-js";
  #shouldResolvePrismaClient;
  #runtimePath;
  #cachedPrismaClientPath;
  constructor({ shouldResolvePrismaClient = true, runtimePath } = {}) {
    this.#shouldResolvePrismaClient = shouldResolvePrismaClient;
    this.#runtimePath = runtimePath;
  }
  async getManifest(config) {
    const defaultOutput = this.#shouldResolvePrismaClient ? await this.#getPrismaClientPath(config) : ".prisma/client";
    return {
      defaultOutput,
      prettyName: "Prisma Client",
      version,
      requiresEngines: [],
      requiresEngineVersion: import_engines_version.enginesVersion
    };
  }
  async generate(options2) {
    const outputDir = parseEnvValue(options2.generator.output);
    await generateClient({
      datamodel: options2.datamodel,
      schemaPath: options2.schemaPath,
      binaryPaths: options2.binaryPaths,
      datasources: options2.datasources,
      outputDir,
      copyRuntime: Boolean(options2.generator.config.copyRuntime),
      copyRuntimeSourceMaps: Boolean(process.env.PRISMA_COPY_RUNTIME_SOURCEMAPS),
      runtimeSourcePath: await this.#getRuntimePath(options2.generator),
      dmmf: options2.dmmf,
      generator: options2.generator,
      engineVersion: options2.version,
      clientVersion: version,
      activeProvider: options2.datasources[0]?.activeProvider,
      typedSql: options2.typedSql,
      compilerBuild: parseCompilerBuildFromUnknown(options2.generator.config.compilerBuild)
    });
  }
  async #getPrismaClientPath(config) {
    if (this.#cachedPrismaClientPath) {
      return this.#cachedPrismaClientPath;
    }
    this.#cachedPrismaClientPath = await resolvePrismaClient(import_node_path4.default.dirname(config.sourceFilePath));
    return this.#cachedPrismaClientPath;
  }
  async #getRuntimePath(config) {
    if (this.#runtimePath) {
      return this.#runtimePath;
    }
    this.#runtimePath = import_node_path4.default.join(await this.#getPrismaClientPath(config), "runtime");
    return this.#runtimePath;
  }
};
function parseCompilerBuildFromUnknown(value) {
  if (value === void 0) {
    return "fast";
  }
  if (value === "small" || value === "fast") {
    return value;
  }
  throw new Error(`Invalid compiler build: ${JSON.stringify(value)}, expected one of: "fast", "small"`);
}

// ../client-generator-js/src/utils/types/dmmfToTypes.ts
var import_node_path5 = __toESM(require("node:path"));
function dmmfToTypes(dmmf) {
  return new TSClient({
    dmmf,
    datasources: [],
    clientVersion: "",
    engineVersion: "",
    runtimeBase: "@prisma/client",
    runtimeName: "client",
    runtimeSourcePath: import_node_path5.default.join(__dirname, "../../../runtime"),
    schemaPath: "",
    outputDir: "",
    activeProvider: "",
    binaryPaths: {},
    generator: {
      binaryTargets: [],
      config: {},
      name: "prisma-client-js",
      output: null,
      provider: { value: "prisma-client-js", fromEnvVar: null },
      previewFeatures: [],
      isCustomOutput: false,
      sourceFilePath: "schema.prisma"
    },
    datamodel: "",
    browser: false,
    edge: false,
    wasm: false,
    compilerBuild: "fast"
  }).toTS();
}

// src/generation/generator.ts
if (process.argv[1] === __filename) {
  const generator = new PrismaClientJsGenerator({
    shouldResolvePrismaClient: false,
    runtimePath: import_node_path6.default.join(__dirname, "..", "runtime")
  });
  generatorHandler({
    onManifest(config) {
      return generator.getManifest(config);
    },
    onGenerate(options2) {
      return generator.generate(options2);
    }
  });
}
// Annotate the CommonJS export names for ESM import in node:
0 && (module.exports = {
  dmmfToTypes,
  externalToInternalDmmf
});
