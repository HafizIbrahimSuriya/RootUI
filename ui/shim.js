/*
 * RootUI compatibility layer.
 * The original eDEX-UI front-end runs inside Electron with Node integration. RootUI runs the same
 * front-end inside a plain webview, with a Rust back-end. This file recreates the small part of
 * Node/Electron that the front-end uses (`require`, `fs`, `electron.ipcRenderer`, `remote`, ...)
 * on top of the local Rust server (HTTP API + websockets).
 */
(function () {
    "use strict";
    const V = window.__vendor || {};

    // ---------- sync / async helpers ----------
    function xhr(method, url, body) {
        const x = new XMLHttpRequest();
        x.open(method, url, false);
        x.send(body === undefined ? null : body);
        if (x.status < 200 || x.status >= 300) throw new Error(`RootUI request failed (${x.status}): ${url}`);
        return x.responseText;
    }
    const getJSON = url => JSON.parse(xhr("GET", url));
    const post = (url, obj) => fetch(url, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(obj || {}) }).catch(() => {});

    // No browser context menu (Reload / Back / Inspect ...) anywhere in the UI.
    window.addEventListener("contextmenu", e => e.preventDefault(), true);
    document.addEventListener("contextmenu", e => e.preventDefault(), true);

    const info = getJSON("/api/info");
    window.__info = info;
    window.__cwd = info.cwd;
    window.global = window;
    window.module = { exports: {} }; // class files end with `module.exports = {...}` (Node-style)
    window.exports = window.module.exports;
    window.__dirname = "/__app";
    window.__filename = "/__app/ui.html";

    // ---------- process ----------
    const hrtime = prev => {
        const t = performance.now();
        let s = Math.floor(t / 1000), ns = Math.floor((t % 1000) * 1e6);
        if (prev) { s -= prev[0]; ns -= prev[1]; if (ns < 0) { s--; ns += 1e9; } }
        return [s, ns];
    };
    hrtime.bigint = () => BigInt(Math.floor(performance.now() * 1e6));
    window.process = {
        platform: info.platform, arch: info.arch, pid: info.pid, env: {}, hrtime,
        argv: ["rootui", ...info.argv], cwd: () => info.cwd, release: { name: "rootui" },
        versions: { electron: "N/A (RootUI " + info.version + ", Rust + WebView)", node: "N/A" },
        nextTick: fn => setTimeout(fn, 0)
    };

    // ---------- IPC (renderer <-> Rust) over one websocket ----------
    const listeners = {};
    const queue = [];
    const ws = new WebSocket(`ws://127.0.0.1:${location.port}/ipc?tz=${new Date().getTimezoneOffset()}`);
    ws.onopen = () => { while (queue.length) ws.send(queue.shift()); };
    ws.onmessage = ev => {
        let m; try { m = JSON.parse(ev.data); } catch (e) { return; }
        const ls = listeners[m.c];
        if (!ls) return;
        for (const l of ls.slice()) {
            if (l.once) ipcRenderer.removeListener(m.c, l.fn);
            try { l.fn({ sender: ipcRenderer }, ...m.a); } catch (e) { console.error(e); }
        }
    };
    const ipcRenderer = {
        send(channel, ...args) {
            const msg = JSON.stringify({ c: channel, a: args });
            if (ws.readyState === 1) ws.send(msg); else queue.push(msg);
        },
        on(channel, fn) { (listeners[channel] = listeners[channel] || []).push({ fn, once: false }); return ipcRenderer; },
        once(channel, fn) { (listeners[channel] = listeners[channel] || []).push({ fn, once: true }); return ipcRenderer; },
        removeListener(channel, fn) { if (listeners[channel]) listeners[channel] = listeners[channel].filter(l => l.fn !== fn); return ipcRenderer; },
        removeAllListeners(channel) { if (channel) delete listeners[channel]; return ipcRenderer; }
    };
    ipcRenderer.off = ipcRenderer.removeListener;

    // Forward JS errors / console output to the Rust process (visible in the launcher console).
    const fmt = a => { try { return typeof a === "string" ? a : (a && a.stack) || JSON.stringify(a); } catch (e) { return String(a); } };
    window.addEventListener("error", e => ipcRenderer.send("log", "error", `JS error: ${e.message} @ ${e.filename}:${e.lineno}:${e.colno}`));
    window.addEventListener("unhandledrejection", e => ipcRenderer.send("log", "error", "Unhandled promise rejection: " + fmt(e.reason)));
    ["warn", "error"].forEach(k => { const o = console[k].bind(console); console[k] = (...a) => { o(...a); ipcRenderer.send("log", k, a.map(fmt).join(" ")); }; });

    // ---------- pty websocket: ws://127.0.0.1:<tty id> -> /pty/<id> ----------
    const NativeWS = window.WebSocket;
    function RootWS(url, protocols) {
        const m = /^ws:\/\/(?:127\.0\.0\.1|localhost):(\d+)\/?$/.exec(url);
        if (m && Number(m[1]) !== Number(location.port)) url = `ws://127.0.0.1:${location.port}/pty/${m[1]}`;
        return protocols === undefined ? new NativeWS(url) : new NativeWS(url, protocols);
    }
    RootWS.prototype = NativeWS.prototype;
    ["CONNECTING", "OPEN", "CLOSING", "CLOSED"].forEach(k => RootWS[k] = NativeWS[k]);
    // (the IPC socket above was created with the native constructor on purpose)
    window.WebSocket = RootWS;

    // ---------- path ----------
    const pathMod = V.path;

    // ---------- fs ----------
    const normP = p => (typeof p === "string" ? p : String(p));
    function fsErr(r) {
        const e = new Error(r.message || "fs error");
        e.code = r.code || "EIO";
        return e;
    }
    function fsCall(op, p) {
        const r = getJSON(`/api/fs?op=${op}&path=${encodeURIComponent(normP(p))}`);
        if (!r.ok) throw fsErr(r);
        return r;
    }
    async function fsCallAsync(op, p) {
        const r = await (await fetch(`/api/fs?op=${op}&path=${encodeURIComponent(normP(p))}`)).json();
        if (!r.ok) throw fsErr(r);
        return r;
    }
    function mkStats(r) {
        return {
            isFile: () => !!r.isFile, isDirectory: () => !!r.isDirectory, isSymbolicLink: () => !!r.isSymbolicLink,
            isBlockDevice: () => false, isCharacterDevice: () => false, isFIFO: () => false, isSocket: () => false,
            size: r.size, mtimeMs: r.mtimeMs, mtime: new Date(r.mtimeMs), mode: 0
        };
    }
    const cbify = (promiseFn) => (...args) => {
        const cb = typeof args[args.length - 1] === "function" ? args.pop() : null;
        const p = promiseFn(...args);
        if (cb) p.then(v => cb(null, v), e => cb(e));
        return p;
    };
    const strOrBuf = (text, opts) => {
        const enc = typeof opts === "string" ? opts : (opts && opts.encoding);
        return enc ? text : Object.assign(new String(text), { toString: () => text });
    };
    const fs = {
        existsSync: p => { try { return !!fsCall("exists", p).exists; } catch (e) { return false; } },
        readdirSync: p => fsCall("readdir", p).entries,
        lstatSync: p => mkStats(fsCall("lstat", p)),
        statSync: p => mkStats(fsCall("stat", p)),
        readlinkSync: p => fsCall("readlink", p).target,
        mkdirSync: p => { fsCall("mkdir", p); },
        unlinkSync: p => { fsCall("unlink", p); },
        readFileSync: (p, opts) => strOrBuf(fsCall("read", p).data, opts),
        writeFileSync: (p, data) => {
            const r = JSON.parse(xhr("POST", `/api/fs?op=write&path=${encodeURIComponent(normP(p))}`, String(data)));
            if (!r.ok) throw fsErr(r);
        },
        readdir: cbify(async p => (await fsCallAsync("readdir", p)).entries),
        lstat: cbify(async p => mkStats(await fsCallAsync("lstat", p))),
        stat: cbify(async p => mkStats(await fsCallAsync("stat", p))),
        readlink: cbify(async p => (await fsCallAsync("readlink", p)).target),
        readFile: cbify(async (p, opts) => strOrBuf((await fsCallAsync("read", p)).data, opts)),
        writeFile: cbify(async (p, data) => {
            const r = await (await fetch(`/api/fs?op=write&path=${encodeURIComponent(normP(p))}`, { method: "POST", body: String(data) })).json();
            if (!r.ok) throw fsErr(r);
        }),
        // Polling replacement for fs.watch (directory content changes)
        watch: (dir, ...rest) => {
            const cb = rest.find(a => typeof a === "function") || (() => {});
            let last = null, stopped = false;
            const tick = async () => {
                if (stopped) return;
                try {
                    const sig = (await fsCallAsync("readdir", dir)).entries.slice().sort().join("\u0000");
                    if (last !== null && sig !== last) cb("rename", null);
                    last = sig;
                } catch (e) { /* ignore */ }
                if (!stopped) setTimeout(tick, 1500);
            };
            tick();
            return { close() { stopped = true; }, on() { return this; } };
        }
    };

    // One-request directory listing (names + stats) used by the file browser.
    window.__fsList = async dir => {
        const r = await (await fetch(`/api/fslist?path=${encodeURIComponent(normP(dir))}`)).json();
        if (!r.ok) throw fsErr(r);
        return r.entries;
    };
    // Plain text of a gist (fetched by the Rust side, so no CORS issues).
    window.__gistText = async url => {
        const r = await fetch(`/api/gist?url=${encodeURIComponent(url)}`);
        if (!r.ok) throw new Error("gist unavailable");
        return await r.text();
    };

    // ---------- os / username ----------
    const t0 = performance.now();
    const os = {
        platform: () => info.platform, type: () => info.type, arch: () => info.arch, release: () => info.release,
        hostname: () => info.hostname, homedir: () => info.home, tmpdir: () => "/tmp",
        uptime: () => info.uptime + (performance.now() - t0) / 1000
    };
    const username = () => Promise.resolve(info.username);
    username.sync = () => info.username;

    // ---------- clipboard / shell ----------
    const clipboard = {
        readText: () => { try { return xhr("GET", "/api/clipboard"); } catch (e) { return ""; } },
        writeText: t => { fetch("/api/clipboard", { method: "POST", body: String(t) }).catch(() => {}); }
    };
    const shell = {
        openPath: p => post("/api/shell/open", { path: p }),
        openExternal: p => post("/api/shell/open", { path: p })
    };

    // ---------- window + global shortcuts ----------
    const winState = { fullscreen: !!info.fullscreen, maximized: false };
    const leaveFsCbs = [];
    const send = (cmd, extra) => post("/api/window", Object.assign({ cmd }, extra || {}));
    const win = {
        on(ev, cb) {
            if (ev === "resize") window.addEventListener("resize", cb);
            else if (ev === "leave-full-screen") leaveFsCbs.push(cb);
            return win;
        },
        isFullScreen: () => winState.fullscreen,
        isMaximized: () => winState.maximized,
        setFullScreen(v) {
            winState.fullscreen = !!v;
            send("setFullScreen", { value: !!v });
            if (!v) setTimeout(() => leaveFsCbs.forEach(cb => cb()), 50);
        },
        maximize() { winState.maximized = true; send("maximize"); },
        unmaximize() { winState.maximized = false; send("unmaximize"); },
        minimize() { send("minimize"); },
        focus() { send("focus"); },
        close() { send("quit"); },
        setSize(w, h) { send("setSize", { w, h }); },
        getSize: () => [window.innerWidth, window.innerHeight],
        webContents: { toggleDevTools() { console.warn("RootUI: DevTools are not available in this build."); } }
    };

    const shortcuts = [];
    const KEYNAMES = { space: " ", esc: "escape", return: "enter", plus: "+", del: "delete", up: "arrowup", down: "arrowdown", left: "arrowleft", right: "arrowright" };
    function parseAccel(a) {
        const o = { ctrl: false, shift: false, alt: false, meta: false, key: "" };
        for (const part of String(a).split("+")) {
            const l = part.trim().toLowerCase();
            if (l === "ctrl" || l === "control") o.ctrl = true;
            else if (l === "commandorcontrol" || l === "cmdorctrl") { if (info.platform === "darwin") o.meta = true; else o.ctrl = true; }
            else if (l === "shift") o.shift = true;
            else if (l === "alt" || l === "option") o.alt = true;
            else if (l === "cmd" || l === "command" || l === "super" || l === "meta") o.meta = true;
            else o.key = KEYNAMES[l] || l;
        }
        return o;
    }
    function matches(a, e) {
        if (e.ctrlKey !== a.ctrl || e.shiftKey !== a.shift || e.altKey !== a.alt || e.metaKey !== a.meta) return false;
        const k = (e.key || "").toLowerCase();
        if (k === a.key) return true;
        if (a.key.length === 1) {
            if (/[a-z]/.test(a.key)) return e.code === "Key" + a.key.toUpperCase();
            if (/[0-9]/.test(a.key)) return e.code === "Digit" + a.key || e.code === "Numpad" + a.key;
        }
        return false;
    }
    window.addEventListener("keydown", e => {
        for (const s of shortcuts) {
            if (matches(s.a, e)) { e.preventDefault(); e.stopPropagation(); s.cb(); return; }
        }
    }, true);
    const globalShortcut = {
        register(accel, cb) { shortcuts.push({ a: parseAccel(accel), cb }); return true; },
        unregisterAll() { shortcuts.length = 0; },
        unregister(accel) { const p = parseAccel(accel); const i = shortcuts.findIndex(s => JSON.stringify(s.a) === JSON.stringify(p)); if (i >= 0) shortcuts.splice(i, 1); },
        isRegistered: accel => { const p = JSON.stringify(parseAccel(accel)); return shortcuts.some(s => JSON.stringify(s.a) === p); }
    };

    // ---------- remote / electron ----------
    const remote = {
        app: {
            getVersion: () => info.version, getName: () => "RootUI",
            getPath: n => (n === "userData" ? info.userData : info.home),
            focus() { send("focus"); }, quit() { send("quit"); }, relaunch() { send("relaunch"); }
        },
        process: { argv: window.process.argv },
        screen: { getAllDisplays: () => Array.from({ length: info.displays || 1 }, () => ({})) },
        getCurrentWindow: () => win,
        globalShortcut, clipboard, shell
    };
    const electron = { ipcRenderer, remote, shell, clipboard, webFrame: { setVisualZoomLevelLimits() {} } };

    // ---------- https / net (external IP + ping, via the Rust server) ----------
    class Emitter {
        constructor() { this._h = {}; }
        on(ev, fn) { (this._h[ev] = this._h[ev] || []).push(fn); return this; }
        emit(ev, ...a) { (this._h[ev] || []).forEach(f => f(...a)); }
    }
    const https = {
        Agent: function () {},
        get(opts, cb) {
            const req = new Emitter();
            req.finished = false;
            let url = "";
            if (opts && opts.host === "myexternalip.com") url = "/api/externalip";
            fetch(url || "/api/__none", url ? {} : { method: "GET" }).then(async r => {
                const res = new Emitter();
                res.statusCode = r.status;
                const text = await r.text();
                req.finished = true;
                if (cb) cb(res);
                if (r.ok) res.emit("data", text);
                res.emit("end");
            }).catch(e => { req.finished = true; req.emit("error", e); });
            return req;
        }
    };
    class SocketImpl extends Emitter {
        connect(o, cb) {
            fetch(`/api/ping?host=${encodeURIComponent(o.host)}&port=${o.port}`).then(r => r.json()).then(j => {
                if (j.ok) { if (cb) cb(); } else this.emit("error", new Error("Socket timeout"));
            }).catch(e => this.emit("error", e));
            return this;
        }
        destroy() {}
        setTimeout() {}
    }
    // The UI calls it as `new require("net").Socket()` - i.e. WITHOUT `new` on Socket (works in Node).
    function Socket() { return new SocketImpl(); }
    const net = { Socket };
    const geolite2 = {
        downloadDbs: () => fetch("/api/geoip/init").then(() => {}),
        open: () => Promise.resolve({
            get: ip => {
                try {
                    const r = JSON.parse(xhr("GET", "/api/geoip?ip=" + encodeURIComponent(ip)));
                    return r || null;
                } catch (e) { return null; }
            }
        })
    };

    // ---------- require() ----------
    // xterm's WebGL renderer: fall back to the default renderer when WebGL is unavailable/broken
    // (also forced with ROOTUI_NOWEBGL=1).
    (function () {
        const Real = V["xterm-addon-webgl"] && V["xterm-addon-webgl"].WebglAddon;
        const forced = location.search.indexOf("nowebgl") >= 0;
        let usable = !forced;
        if (usable) { try { usable = !!document.createElement("canvas").getContext("webgl2"); } catch (e) { usable = false; } }
        V["xterm-addon-webgl"] = {
            WebglAddon: class {
                constructor() { this._a = usable && Real ? new Real() : null; }
                activate(t) { try { if (this._a) this._a.activate(t); } catch (e) { console.warn("WebGL renderer unavailable, using fallback: " + e); this._a = null; } }
                dispose() { try { if (this._a) this._a.dispose(); } catch (e) { /* ignore */ } }
            }
        };
    })();
    const builtin = Object.assign({}, V, {
        path: pathMod, fs, os, username, electron, "@electron/remote": remote, https, net,
        "geolite2-redist": geolite2, maxmind: { open: () => ({}) },
        child_process: { exec() { throw new Error("child_process is not available in RootUI"); } }
    });
    const fileCache = {};
    function virtual(p) {
        p = p.replace(/\\/g, "/");
        if (p.startsWith("./")) p = "/__app/" + p.slice(2);
        return p;
    }
    window.require = function require(name) {
        if (Object.prototype.hasOwnProperty.call(builtin, name)) return builtin[name];
        const p = virtual(name);
        if (p.startsWith("/") || /^[a-zA-Z]:\//.test(p)) {
            if (fileCache[p]) return fileCache[p].exports;
            const text = fsCall("read", p).data;
            const module = { exports: {} };
            fileCache[p] = module;
            if (/\.json$/i.test(p)) module.exports = JSON.parse(text);
            else new Function("module", "exports", "require", "__dirname", "__filename", text + "\n//# sourceURL=" + p)(module, module.exports, window.require, p.replace(/\/[^/]*$/, ""), p);
            return module.exports;
        }
        throw new Error("Cannot find module '" + name + "' (RootUI)");
    };

    // Path -> URL for local files loaded by <img>/<video>/<audio>/pdf.js
    window.__fileURL = p => "/__file/" + encodeURI(String(p).replace(/\\/g, "/").replace(/^\//, "")).replace(/#/g, "%23").replace(/\?/g, "%3F");

})();
