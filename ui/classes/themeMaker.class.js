// RootUI theme maker: design a theme inside the app, see it live, draw a pixel icon, export / import it.
// Custom themes are saved next to the built-in ones (<config>/themes/<name>.json) with  "custom": true.

const TM_HEX = /^#[0-9a-f]{6}$/i;
const TM_SIZE = 12; // pixel icon is 12 x 12
const TM_DEFAULT = {
    name: "", accent: "#aacfd1", panel: "#05080d", black: "#000000", grey: "#262828",
    fg: "#aacfd1", termBg: "#05080d", cursor: "#aacfd1", cursorStyle: "block", globe: "#aacfd1",
    fontMain: "United Sans Medium", fontLight: "United Sans Light", fontMono: "Fira Mono"
};
const TM_COLORS = [
    ["accent", "Accent (lines, text)"], ["panel", "Panel background"], ["black", "Screen background"], ["grey", "Keys & bars"],
    ["fg", "Terminal text"], ["termBg", "Terminal background"], ["cursor", "Terminal cursor"], ["globe", "Globe"]
];
const TM_PALETTE = [["#", "Accent"], ["d", "Dark"], ["w", "Light"], ["k", "Shadow"], [".", "Erase"]];

class ThemeMaker {
    constructor() {
        this.st = null;
        this.kbUsers = 0;
        this.painting = false;
        this.brush = "#";
        this.fontsLoaded = {};
    }

    // ---------------------------------------------------------------- helpers
    esc(s) { return window._escapeHtml(String(s)); }
    slugify(n) { return String(n || "").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "").slice(0, 32).replace(/-+$/g, ""); }
    hexToRgb(h) { return [1, 3, 5].map(i => parseInt(h.substr(i, 2), 16)); }
    rgbToHex(r, g, b) { return "#" + [r, g, b].map(v => Math.max(0, Math.min(255, Math.round(Number(v) || 0))).toString(16).padStart(2, "0")).join(""); }
    validHex(h, fallback) { return typeof h === "string" && TM_HEX.test(h) ? h.toLowerCase() : fallback; }

    builtinNames() {
        try {
            return fs.readdirSync(path.join(__dirname, "assets", "themes")).filter(f => f.endsWith(".json")).map(f => f.slice(0, -5));
        } catch (e) { return []; }
    }

    listFonts() {
        const out = [];
        try {
            fs.readdirSync(fontsDir).forEach(f => {
                if (/\.woff2$/i.test(f)) out.push(f.slice(0, -6).split("_").map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(" "));
            });
        } catch (e) { /* no fonts folder */ }
        if (!out.length) out.push("United Sans Medium", "United Sans Light", "Fira Mono");
        return out.sort();
    }

    ensureFont(name) {
        if (!name || this.fontsLoaded[name]) return;
        this.fontsLoaded[name] = true;
        try {
            const url = path.join(fontsDir, name.toLowerCase().replace(/ /g, "_") + ".woff2").replace(/\\/g, "/");
            const f = new FontFace(name, `url("${url}")`);
            document.fonts.add(f);
            f.load().catch(() => {});
        } catch (e) { /* preview just uses the fallback font */ }
    }

    normalizeRows(rows) {
        const out = [];
        for (let y = 0; y < TM_SIZE; y++) {
            let row = String((rows && rows[y]) || "");
            row = row.slice(0, TM_SIZE).split("").map(c => ("#dwk".includes(c) ? c : ".")).join("");
            out.push(row.padEnd(TM_SIZE, "."));
        }
        return out;
    }

    presetRows(key) { return this.normalizeRows((PIXEL_SPRITES[key] || PIXEL_SPRITES.prompt).split("|")); }

    spriteRowsFor(name) {
        const base = name.replace(/-(notype|typeleft|fulltype|colorfilter|disrupted|ligatures|focus)$/, "");
        const key = PIXEL_THEME_MAP[name] || PIXEL_THEME_MAP[base] || "prompt";
        return this.presetRows(key);
    }

    readTheme(slug) { return JSON.parse(fs.readFileSync(path.join(themesDir, slug + ".json")).toString()); }

    // theme file -> editor state
    stateFromTheme(t, slug, keepName) {
        const c = t.colors || {}, tm = t.terminal || {}, g = t.globe || {}, cv = t.cssvars || {};
        const accent = this.rgbToHex(c.r, c.g, c.b);
        const fonts = this.listFonts();
        const pick = (v, d) => (typeof v === "string" && fonts.includes(v) ? v : (fonts.includes(d) ? d : fonts[0]));
        const rows = t.pixelIcon && Array.isArray(t.pixelIcon.rows) ? this.normalizeRows(t.pixelIcon.rows) : this.spriteRowsFor(slug || "");
        return {
            name: keepName !== undefined ? keepName : (slug || ""),
            editing: keepName !== undefined ? null : (t.custom ? slug : null),
            accent,
            panel: this.validHex(c.light_black, TM_DEFAULT.panel),
            black: this.validHex(c.black, TM_DEFAULT.black),
            grey: this.validHex(c.grey, TM_DEFAULT.grey),
            fg: this.validHex(tm.foreground, accent),
            termBg: this.validHex(tm.background, this.validHex(c.light_black, TM_DEFAULT.termBg)),
            cursor: this.validHex(tm.cursor, accent),
            cursorStyle: ["block", "underline", "bar"].includes(tm.cursorStyle) ? tm.cursorStyle : "block",
            globe: this.validHex(g.marker, accent),
            fontMain: pick(cv.font_main, TM_DEFAULT.fontMain),
            fontLight: pick(cv.font_main_light, TM_DEFAULT.fontLight),
            fontMono: pick(tm.fontFamily, TM_DEFAULT.fontMono),
            rows
        };
    }

    // editor state -> theme file
    themeFromState(st) {
        const [r, g, b] = this.hexToRgb(st.accent);
        return {
            format: "rootui-theme", version: 1, custom: true, name: st.name,
            colors: { r, g, b, black: st.black, light_black: st.panel, grey: st.grey },
            cssvars: { font_main: st.fontMain, font_main_light: st.fontLight },
            terminal: {
                fontFamily: st.fontMono, cursorStyle: st.cursorStyle,
                foreground: st.fg, background: st.termBg, cursor: st.cursor, cursorAccent: st.cursor,
                selection: `rgba(${r},${g},${b},0.3)`
            },
            globe: { base: st.black, marker: st.globe, pin: st.globe, satellite: st.globe },
            pixelIcon: { size: TM_SIZE, rows: this.normalizeRows(st.rows) }
        };
    }

    // Anything that comes from a file / the clipboard goes through here: only known keys, strict value checks.
    sanitize(obj) {
        if (!obj || typeof obj !== "object" || !obj.colors) throw new Error("this does not look like a RootUI theme");
        const n = v => Math.max(0, Math.min(255, Math.round(Number(v))));
        const c = obj.colors;
        if (![c.r, c.g, c.b].every(v => Number.isFinite(Number(v)))) throw new Error("the theme has no valid accent colour (colors.r / g / b)");
        const base = this.stateFromTheme(Object.assign({}, obj, { colors: Object.assign({}, c, { r: n(c.r), g: n(c.g), b: n(c.b) }) }), "", "");
        base.name = String(obj.name || "").replace(/[^\w .-]/g, "").trim().slice(0, 24);
        return this.themeFromState(base);
    }

    kb(on) { // the on-screen keyboard must not type into the terminal while we use text fields
        if (on) {
            if (this.kbUsers++ === 0 && window.keyboard) window.keyboard.detach();
        } else if (--this.kbUsers <= 0) {
            this.kbUsers = 0;
            if (window.keyboard) window.keyboard.attach();
            try { window.term[window.currentTerm].term.focus(); } catch (e) { /* no terminal */ }
        }
    }

    msg(text, bad, id) {
        const el = document.getElementById(id || "tm_msg");
        if (el) { el.className = "tm_msg" + (bad ? " bad" : " good"); el.innerHTML = text; }
        if (bad) window.audioManager.error.play();
        return null;
    }

    // ---------------------------------------------------------------- the maker window
    open(edit) {
        if (document.getElementById("tm_root")) return;
        let st = Object.assign({}, TM_DEFAULT, { rows: this.presetRows("prompt"), editing: null });
        if (edit) {
            try { st = this.stateFromTheme(this.readTheme(edit), edit); } catch (e) { console.warn("Cannot open theme " + edit + ": " + e); }
        }
        this.st = st;
        this.brush = "#";
        this.fonts = this.listFonts();
        this.fonts.forEach(f => this.ensureFont(f));

        this.kb(true);
        this.onUp = () => { this.painting = false; };
        window.addEventListener("mouseup", this.onUp);

        new Modal({
            type: "custom",
            title: `Theme <i>maker</i>`,
            html: this.html(),
            buttons: [
                { label: "Save", action: "window.themeMaker.save(false)" },
                { label: "Save & use", action: "window.themeMaker.save(true)" },
                { label: "Export", action: "window.themeMaker.exportCurrent()" },
                { label: "Import", action: "window.themeMaker.openImport()" }
            ]
        }, () => this.onClose());
        this.bind();
        this.refresh();
    }

    onClose() {
        window.removeEventListener("mouseup", this.onUp);
        this.live(false);
        this.kb(false);
    }

    html() {
        const st = this.st;
        const colorRows = TM_COLORS.map(([k, label]) => `<div class="tm_row"><span>${label}</span>
            <input type="color" data-k="${k}" value="${st[k]}">
            <input type="text" class="tm_hex" data-k="${k}" maxlength="7" spellcheck="false" value="${st[k]}"></div>`).join("");
        const opts = (list, cur) => list.map(f => `<option${f === cur ? " selected" : ""}>${this.esc(f)}</option>`).join("");
        const themes = window.quickSettings ? window.quickSettings.listThemes().map(t => t.name) : [];
        const presets = Object.keys(PIXEL_SPRITES).filter(k => k !== "dragon");
        const palette = TM_PALETTE.map(([ch, label]) => `<button type="button" class="tm_sw${ch === this.brush ? " on" : ""}" data-p="${ch}" title="${label}"><i></i><em>${label}</em></button>`).join("");
        let cells = "";
        for (let y = 0; y < TM_SIZE; y++) for (let x = 0; x < TM_SIZE; x++) cells += `<i data-x="${x}" data-y="${y}"></i>`;

        return `<div id="tm_root" class="tm">
          <div class="tm_cols">
            <div class="tm_form">
              <div class="tm_row tm_top"><span>Theme name</span>
                <input type="text" id="tm_name" data-k="name" maxlength="24" spellcheck="false" placeholder="my-theme" value="${this.esc(st.name)}"></div>
              <div class="tm_row tm_top"><span>Start from</span>
                <select id="tm_base" data-k="base"><option value="">(keep my colours)</option>${opts(themes, "")}</select></div>

              <h4 class="rootui_h">COLOURS</h4>
              ${colorRows}

              <h4 class="rootui_h">FONTS &amp; CURSOR</h4>
              <div class="tm_row"><span>Main font</span><select data-k="fontMain">${opts(this.fonts, st.fontMain)}</select></div>
              <div class="tm_row"><span>Light font</span><select data-k="fontLight">${opts(this.fonts, st.fontLight)}</select></div>
              <div class="tm_row"><span>Terminal font</span><select data-k="fontMono">${opts(this.fonts, st.fontMono)}</select></div>
              <div class="tm_row"><span>Cursor</span><select data-k="cursorStyle">${opts(["block", "underline", "bar"], st.cursorStyle)}</select></div>
            </div>

            <div class="tm_side">
              <h4 class="rootui_h">PREVIEW</h4>
              ${this.previewHtml()}
              <label class="tm_live"><input type="checkbox" id="tm_live"> Also preview on the real app (behind this window)</label>

              <h4 class="rootui_h">PIXEL ICON <small>12 x 12 - click or drag to paint</small></h4>
              <div class="tm_icon">
                <div class="tm_grid" id="tm_grid">${cells}</div>
                <div class="tm_tools">
                  <div class="tm_palette">${palette}</div>
                  <select id="tm_preset"><option value="">Load a preset icon...</option>${opts(presets, "")}</select>
                  <div class="tm_btns">
                    <button type="button" data-a="clear">Clear</button>
                    <button type="button" data-a="mirror">Mirror</button>
                    <button type="button" data-a="flip">Flip</button>
                  </div>
                  <div class="tm_iconprev"><i id="tm_iconprev"></i><span>in the theme list</span></div>
                </div>
              </div>
            </div>
          </div>
          <div id="tm_msg" class="tm_msg"></div>
        </div>`;
    }

    previewHtml() {
        const spark = "0,12 8,9 16,13 24,6 32,10 40,14 48,8 56,11 64,5 72,9 80,12 88,7 100,10";
        let keys = "";
        ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "A", "S", "D", "F", "G", "H", "J", "K", "L", ";"].forEach(k => keys += `<b>${k}</b>`);
        return `<div class="tm_pv" id="tm_pv">
          <div class="pv_cols">
            <div class="pv_l">
              <div class="pv_cap"><span>PANEL</span><span>SYSTEM</span></div>
              <div class="pv_clock">15:45:37</div>
              <div class="pv_lbl">CPU USAGE</div>
              <svg class="pv_spark" viewBox="0 0 100 20" preserveAspectRatio="none"><polyline points="${spark}"/></svg>
              <div class="pv_lbl">MEMORY</div>
              <div class="pv_dots"></div>
            </div>
            <div class="pv_mid">
              <div class="pv_tabs"><b class="on">MAIN - shell</b><b>EMPTY</b><b>EMPTY</b></div>
              <div class="pv_term">
                <div>Welcome to RootUI ${this.esc((window.__info && window.__info.version) || "1.0V")}</div>
                <div>~ $ ls</div><div>docs&nbsp;&nbsp;music&nbsp;&nbsp;src</div>
                <div>~ $ <i class="pv_cur"></i></div>
              </div>
            </div>
            <div class="pv_r">
              <div class="pv_cap"><span>PANEL</span><span>NETWORK</span></div>
              <div class="pv_lbl">NETWORK STATUS</div>
              <div class="pv_state">ONLINE <small>21ms</small></div>
              <div class="pv_globe"></div>
              <svg class="pv_spark" viewBox="0 0 100 20" preserveAspectRatio="none"><polyline points="0,10 12,10 20,3 28,17 36,10 60,10 72,8 84,12 100,10"/></svg>
            </div>
          </div>
          <div class="pv_bottom">
            <div class="pv_fs"><i></i><i></i><i></i><i></i></div>
            <div class="pv_kb">${keys}</div>
          </div>
        </div>`;
    }

    bind() {
        const root = document.getElementById("tm_root");
        const onField = e => {
            const t = e.target;
            if (t.id === "tm_base") return this.loadBase(t.value);
            if (t.id === "tm_preset") { if (t.value) { this.st.rows = this.presetRows(t.value); t.value = ""; this.refresh(); } return; }
            if (t.id === "tm_live") return this.refresh();
            if (t.dataset && t.dataset.k) this.setField(t.dataset.k, t.value, t);
        };
        root.addEventListener("input", onField);
        root.addEventListener("change", onField);

        root.addEventListener("mousedown", e => {
            const cell = e.target.closest && e.target.closest("#tm_grid > i");
            if (cell) { this.painting = true; this.paintCell(cell); e.preventDefault(); return; }
            const sw = e.target.closest && e.target.closest(".tm_sw");
            if (sw) { this.brush = sw.dataset.p; root.querySelectorAll(".tm_sw").forEach(b => b.classList.toggle("on", b === sw)); }
        });
        root.addEventListener("mouseover", e => {
            if (!this.painting) return;
            const cell = e.target.closest && e.target.closest("#tm_grid > i");
            if (cell) this.paintCell(cell);
        });
        root.addEventListener("click", e => {
            const b = e.target.closest && e.target.closest("button[data-a]");
            if (!b) return;
            const rows = this.st.rows.map(r => r.split(""));
            if (b.dataset.a === "clear") this.st.rows = this.normalizeRows([]);
            if (b.dataset.a === "mirror") this.st.rows = rows.map(r => { for (let x = 0; x < TM_SIZE / 2; x++) r[TM_SIZE - 1 - x] = r[x]; return r.join(""); });
            if (b.dataset.a === "flip") this.st.rows = rows.reverse().map(r => r.join(""));
            this.refresh();
        });
    }

    setField(k, v, src) {
        const st = this.st;
        if (TM_COLORS.some(c => c[0] === k)) {
            if (src.type === "text") {
                if (v && v[0] !== "#") v = "#" + v;
                src.classList.toggle("bad", !TM_HEX.test(v));
                if (!TM_HEX.test(v)) return;
            }
            st[k] = v.toLowerCase();
            document.querySelectorAll(`#tm_root [data-k="${k}"]`).forEach(el => { if (el !== src) el.value = st[k]; });
        } else {
            st[k] = v;
        }
        this.refresh();
    }

    loadBase(name) {
        if (!name) return;
        try {
            const keep = this.st.name, editing = this.st.editing;
            this.st = this.stateFromTheme(this.readTheme(name), name, keep);
            this.st.editing = editing;
            this.syncInputs();
            this.refresh();
        } catch (e) { this.msg("Could not read that theme.", true); }
    }

    syncInputs() {
        const st = this.st;
        document.querySelectorAll("#tm_root [data-k]").forEach(el => {
            const k = el.dataset.k;
            if (k !== "base" && k !== "name" && st[k] !== undefined) { el.value = st[k]; el.classList.remove("bad"); }
        });
    }

    paintCell(cell) {
        const x = Number(cell.dataset.x), y = Number(cell.dataset.y);
        const row = this.st.rows[y];
        if (row[x] === this.brush) return;
        this.st.rows[y] = row.slice(0, x) + this.brush + row.slice(x + 1);
        this.refresh();
    }

    // redraw everything that depends on the editor state
    refresh() {
        const st = this.st;
        if (!st || !document.getElementById("tm_root")) return;
        const [r, g, b] = this.hexToRgb(st.accent);
        const pal = pixelPalette(r, g, b, st.panel);

        // pixel grid + palette swatches
        const cells = document.querySelectorAll("#tm_grid > i");
        cells.forEach(el => {
            const ch = st.rows[el.dataset.y][el.dataset.x];
            el.style.background = pal[ch] || "";
            el.classList.toggle("empty", !pal[ch]);
        });
        document.querySelectorAll(".tm_sw").forEach(sw => { sw.firstChild.style.background = pal[sw.dataset.p] || "transparent"; });
        document.getElementById("tm_grid").style.background = st.panel;
        document.getElementById("tm_iconprev").style.background = st.panel;
        document.getElementById("tm_iconprev").innerHTML = pixelSvgFromRows(st.rows, r, g, b, st.panel);

        // mini preview of the app
        this.fonts.forEach(f => this.ensureFont(f));
        const pv = document.getElementById("tm_pv");
        const vars = {
            "--pa": `${r},${g},${b}`, "--pp": st.panel, "--pb": st.black, "--pg": st.grey, "--pf": st.fg, "--ptb": st.termBg,
            "--pc": st.cursor, "--pgl": st.globe, "--pfm": `"${st.fontMain}"`, "--pfl": `"${st.fontLight}"`, "--pfo": `"${st.fontMono}"`
        };
        Object.keys(vars).forEach(k => pv.style.setProperty(k, vars[k]));
        pv.dataset.cursor = st.cursorStyle;

        this.live(document.getElementById("tm_live") && document.getElementById("tm_live").checked);
    }

    // optional: paint the colours on the real app too (inline variables win over the theme's <style>)
    live(on) {
        const css = document.documentElement.style;
        const names = ["--color_r", "--color_g", "--color_b", "--color_black", "--color_light_black", "--color_grey", "--font_main", "--font_main_light", "--font_mono"];
        if (!on || !this.st) { names.forEach(n => css.removeProperty(n)); return; }
        const st = this.st, [r, g, b] = this.hexToRgb(st.accent);
        const vals = [r, g, b, st.black, st.panel, st.grey, `"${st.fontMain}"`, `"${st.fontLight}"`, `"${st.fontMono}"`];
        names.forEach((n, i) => css.setProperty(n, vals[i]));
    }

    // ---------------------------------------------------------------- save / export / delete
    persist() {
        const st = this.st;
        st.name = String(st.name || "").replace(/[^\w .-]/g, "").trim();
        const slug = this.slugify(st.name);
        if (!slug) return this.msg("Give your theme a name first.", true);
        if (this.builtinNames().includes(slug)) return this.msg(`&ldquo;${this.esc(slug)}&rdquo; is the name of a built-in theme - pick another name.`, true);
        const file = path.join(themesDir, slug + ".json");
        if (fs.existsSync(file) && slug !== st.editing) {
            return this.msg(`A theme called &ldquo;${this.esc(slug)}&rdquo; already exists. Pick another name, or edit that one with the pencil button.`, true);
        }
        try {
            fs.writeFileSync(file, JSON.stringify(this.themeFromState(Object.assign({}, st, { name: st.name })), "", 4));
        } catch (e) { return this.msg("Could not save the theme: " + this.esc(e.message || e), true); }
        if (st.editing && st.editing !== slug) { // renamed: remove the old file
            try { fs.unlinkSync(path.join(themesDir, st.editing + ".json")); } catch (e) { /* leave it */ }
        }
        st.editing = slug;
        if (window.quickSettings) window.quickSettings.refreshThemes();
        return slug;
    }

    save(useNow) {
        const slug = this.persist();
        if (!slug) return;
        window.audioManager.granted.play();
        this.msg(`Saved as &ldquo;${this.esc(slug)}&rdquo;. It is in your theme list now.`, false);
        if (useNow) { this.live(false); window.setTheme(slug); }
    }

    exportCurrent() {
        const slug = this.persist();
        if (slug) this.exportTheme(slug);
    }

    exportTheme(slug) {
        let text;
        try { text = fs.readFileSync(path.join(themesDir, slug + ".json")).toString(); } catch (e) {
            new Modal({ type: "error", title: "Export failed", message: "Cannot read that theme." });
            return;
        }
        const home = (window.__info && window.__info.home) || "";
        let dir = home;
        try { if (fs.existsSync(path.join(home, "Downloads"))) dir = path.join(home, "Downloads"); } catch (e) { /* use home */ }
        const file = path.join(dir, slug + ".rootui-theme.json");
        let saved = true;
        try { fs.writeFileSync(file, text); } catch (e) { saved = false; }
        try { electron.clipboard.writeText(text); } catch (e) { /* no clipboard */ }
        new Modal({
            type: "info", title: "Theme exported",
            message: saved
                ? `Saved to<br><b>${this.esc(file)}</b><br><br>The theme text is also on your clipboard, ready to paste to a friend.<br>They can load it with <b>Import</b> in RootUI settings.`
                : `The file could not be written, but the theme text is on your clipboard.<br>Paste it into a text file to share it.`
        });
    }

    // two-step delete: first click arms the button, second click deletes
    remove(btn) {
        const cell = btn.closest(".rootui_theme");
        const slug = cell.dataset.theme;
        if (!btn.dataset.armed) {
            btn.dataset.armed = "1";
            btn.textContent = "sure?";
            setTimeout(() => { if (btn.isConnected) { delete btn.dataset.armed; btn.innerHTML = "&#10005;"; } }, 3000);
            return;
        }
        if (slug === window.settings.theme) {
            btn.textContent = "in use";
            return;
        }
        try { fs.unlinkSync(path.join(themesDir, slug + ".json")); } catch (e) { console.warn("Could not delete theme: " + e); }
        if (window.quickSettings) window.quickSettings.refreshThemes();
    }

    // ---------------------------------------------------------------- import
    openImport() {
        if (document.getElementById("tm_import")) return;
        this.kb(true);
        new Modal({
            type: "custom",
            title: `Import <i>theme</i>`,
            html: `<div id="tm_import" class="tm tm_imp">
                <p>Choose a theme file (.json) or paste the theme text below.</p>
                <label class="tm_file"><input type="file" id="tm_file" accept=".json,application/json"></label>
                <textarea id="tm_paste" spellcheck="false" placeholder='{ "colors": { "r": 170, "g": 207, "b": 209, ... } }'></textarea>
                <div id="tm_imsg" class="tm_msg"></div>
            </div>`,
            buttons: [
                { label: "Paste from clipboard", action: "window.themeMaker.pasteClipboard()" },
                { label: "Import", action: "window.themeMaker.doImport()" }
            ]
        }, () => this.kb(false));
        document.getElementById("tm_file").addEventListener("change", ev => {
            const f = ev.target.files && ev.target.files[0];
            if (!f) return;
            if (f.size > 200000) return this.msg("That file is too big to be a theme.", true, "tm_imsg");
            const rd = new FileReader();
            rd.onload = () => { document.getElementById("tm_paste").value = String(rd.result || ""); this.msg("Loaded " + this.esc(f.name) + ". Press Import.", false, "tm_imsg"); };
            rd.readAsText(f);
        });
    }

    pasteClipboard() {
        let t = "";
        try { t = electron.clipboard.readText(); } catch (e) { /* ignore */ }
        document.getElementById("tm_paste").value = t;
        if (!t) this.msg("The clipboard is empty.", true, "tm_imsg");
    }

    doImport() {
        const box = document.getElementById("tm_paste");
        if (!box || !box.value.trim()) return this.msg("Choose a file or paste a theme first.", true, "tm_imsg");
        let theme;
        try { theme = this.sanitize(JSON.parse(box.value)); } catch (e) { return this.msg("Cannot import: " + this.esc(e.message || e), true, "tm_imsg"); }

        let slug = this.slugify(theme.name) || "imported-theme";
        const taken = s => this.builtinNames().includes(s) || fs.existsSync(path.join(themesDir, s + ".json"));
        if (taken(slug)) {
            const root = slug;
            let i = 2;
            while (taken(root + "-" + i)) i++;
            slug = root + "-" + i;
        }
        theme.name = slug;
        try { fs.writeFileSync(path.join(themesDir, slug + ".json"), JSON.stringify(theme, "", 4)); } catch (e) {
            return this.msg("Could not save: " + this.esc(e.message || e), true, "tm_imsg");
        }
        window.audioManager.granted.play();
        if (window.quickSettings) window.quickSettings.refreshThemes();
        this.msg(`Imported as &ldquo;${slug}&rdquo;. <a onclick="window.setTheme('${slug}')">Use it now</a>`, false, "tm_imsg");
        box.value = "";
    }
}

window.themeMaker = new ThemeMaker();

module.exports = { ThemeMaker };
