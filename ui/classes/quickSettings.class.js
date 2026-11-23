// Pixel-art theme icons (12x12). # = accent, d = dark accent, w = light accent, k = shadow, . = empty
const PIXEL_SPRITES = {
    dragon: "#..............#|##............##|.##...####...##.|..##.######.##..|..############..|.##wwk####kww##.|.##############.|..############..|...####dd####...|...##kd##dk##...|...##########...|....########....|....#w#ww#w#....|.....#.ww.#.....|......#..#......|................",
    cyborg: "............|...######...|..########..|.##########.|.#kk#wwwww#.|.#kk#wwwww#.|.##########.|.#d######d#.|..#dkkkkd#..|..########..|...#d##d#...|############",
    disc: "...######...|.###....###.|.#........#.|##..####..##|#..#....#..#|#..#.ww.#..#|#..#.ww.#..#|#..#....#..#|##..####..##|.#........#.|.###....###.|...######...",
    anchor: ".....##.....|....#..#....|.....##.....|...######...|.....##.....|.....##.....|.#...##...#.|.#...##...#.|.##..##..##.|..##.##.##..|...######...|.....##.....",
    chalk: "############|#..........#|#.ww...w...#|#...w.w.w..#|#....w..w..#|#.ww...w.w.#|#..........#|#.wwwwww...#|#..........#|############|..dd..ww.dd.|.dddddddddd.",
    rocket: ".....##.....|....####....|....####....|...######...|...#wwww#...|...#wwww#...|...######...|..########..|.###.##.###.|.##..##..##.|.....ww.....|....w..w....",
    hole: "............|....dddd....|..dddwwddd..|.ddwkkkkwdd.|.dwkkkkkkwd.|wwwwwwwwwwww|dwwwwwwwwwwd|.dwkkkkkkwd.|.ddwkkkkwdd.|..dddwwddd..|....dddd....|............",
    eye: "............|............|....####....|..##wwww##..|.#wwwwwwww#.|#www####www#|#www#kk#www#|.#wwwwwwww#.|..##wwww##..|....####....|............|............",
    flame: "............|.....#......|....##..#...|...###.##...|..#######.#.|..#######.#.|.#####w####.|.####www###.|.###wwwww##.|.###wwwww##.|..##wwww##..|...######...",
    sun: "............|.....##.....|.#...##...#.|..#.####.#..|...######...|##.######.##|##.######.##|...######...|..#.####.#..|.#...##...#.|.....##.....|............",
    blossom: "............|....####....|....####....|.##########.|.####ww####.|.###wdww###.|.###wwdw###.|..###ww###..|..########..|..########..|...##..##...|............",
    crt: "............|.##########.|.#........#.|.#.w......#.|.#..w.....#.|.#.w..ww..#.|.#........#.|.##########.|.....##.....|...######...|............|............",
    bulb: "............|...######...|..########..|.##########.|.##w.ww.w##.|.##########.|..########..|...######...|...dddddd...|....dddd....|...dddddd...|....d..d....",
    braces: "............|...##..##...|..#......#..|..#......#..|.#........#.|#....ww....#|#....ww....#|.#........#.|..#......#..|..#......#..|...##..##...|............",
    crown: "............|............|#....##....#|##...##...##|###.####.###|############|##w##ww##w##|############|dddddddddddd|............|............|............",
    bat: "............|............|#...#..#...#|##.######.##|############|#.#w####w#.#|..#d####d#..|...#w##w#...|....#..#....|............|............|............",
    snow: ".....##.....|.#...##...#.|..#..##..#..|...#.##.#...|....####....|############|############|....####....|...#.##.#...|..#..##..#..|.#...##...#.|.....##.....",
    mountain: "............|............|.....ww.....|....wwww....|...wwwwww...|..w#wwww#w..|.##########.|############|############|dddddddddddd|............|............",
    gem: "............|............|..########..|.#wd####dw#.|############|.#d######d#.|..#d####d#..|...#d##d#...|....#dd#....|.....##.....|............|............",
    ghost: "............|..wwwwwwww..|.wwwwwwwwww.|wwwwwwwwwwww|wkkwwwwwwkkw|wkkwwwwwwkkw|wwwwwwwwwwww|wwwwwwwwwwww|wwwwwwwwwwww|ww.wwwwww.ww|w..w.ww.w..w|............",
    prompt: "............|............|############|#..........#|#.w........#|#..w.......#|#.w..wwww..#|#..........#|############|............|............|............",
    moon: "............|...##.......|..##....w...|.###........|.###........|.###........|.###......w.|.####.......|.#####......|..#####w##..|...######...|............",
    city: "............|............|............|...###......|...dwd......|...ddd##....|###dwddw....|dwdddddd####|ddddwddwdwdw|dwdddddddddd|ddddwddwdwdw|dwdddddddddd",
    synth: "............|...######...|..########..|..########..|.##########.|.##########.|............|..########..|............|dddddddddddd|..d..d..d..d|.d..d..d..d.",
    rain: "....d.....d.|....d.....d.|.d..#.....d.|.d..#.....d.|.d..w..d..#.|.d.....d..#.|.#.....d..w.|.#.....d....|.w.....#....|.......#....|.......w....|............",
};
const PIXEL_THEME_MAP = {
    cyborg: "cyborg", "cyborg-focus": "cyborg", tron: "disc", navy: "anchor", chalkboard: "chalk", apollo: "rocket",
    interstellar: "hole", blade: "eye", red: "flame", matrix: "rain", "solarized-dark": "sun", sakura: "blossom",
    gruvbox: "crt", amber: "bulb", monokai: "braces", gold: "crown", "tokyo-night": "moon", dracula: "bat",
    ice: "snow", nord: "mountain", emerald: "gem", cyberpunk: "city", synthwave: "synth", ghost: "ghost"
};

// Palette used by every pixel icon: # accent, d dark accent, w light accent, k shadow, . empty
const PIXEL_CHARS = "#dwk.";
function pixelPalette(r, g, b, bg) {
    const hex = c => "#" + c.map(v => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, "0")).join("");
    const mix = (a, c, t) => a.map((v, i) => v * (1 - t) + c[i] * t);
    const m = String(bg).match(/^#?([0-9a-f]{6})$/i);
    const bgc = m ? [0, 2, 4].map(i => parseInt(m[1].substr(i, 2), 16)) : [0, 0, 0];
    const acc = [Number(r) || 0, Number(g) || 0, Number(b) || 0];
    return { "#": hex(acc), d: hex(mix(acc, bgc, 0.55)), w: hex(mix(acc, [255, 255, 255], 0.75)), k: hex(mix(bgc, [0, 0, 0], 0.5)) };
}
function pixelSvgFromRows(rows, r, g, b, bg) {
    const col = pixelPalette(r, g, b, bg);
    const size = Math.max(1, rows.length);
    let rects = "";
    rows.forEach((row, y) => {
        let x = 0;
        while (x < row.length) {
            const c = row[x];
            if (!col[c]) { x++; continue; }
            let w = 1;
            while (row[x + w] === c) w++;
            rects += `<rect x="${x}" y="${y}" width="${w}" height="1" fill="${col[c]}"/>`;
            x += w;
        }
    });
    return `<svg viewBox="0 0 ${Math.max(size, ...rows.map(q => q.length))} ${size}" shape-rendering="crispEdges">${rects}</svg>`;
}

// customRows: 12 strings drawn by the user in the theme maker (overrides the built-in sprite)
function pixelThemeIcon(name, r, g, b, bg, customRows) {
    if (Array.isArray(customRows) && customRows.length) return pixelSvgFromRows(customRows, r, g, b, bg);
    const base = name.replace(/-(notype|typeleft|fulltype|colorfilter|disrupted|ligatures|focus)$/, "");
    const key = PIXEL_THEME_MAP[name] || PIXEL_THEME_MAP[base] || "prompt";
    return pixelSvgFromRows(PIXEL_SPRITES[key].split("|"), r, g, b, bg);
}

// Dragon pixel art: coloured with CSS classes that read the theme variables, so it changes with the theme.
function dragonSvg() {
    const rows = PIXEL_SPRITES.dragon.split("|");
    const cls = { "#": "dg_a", d: "dg_d", w: "dg_w", k: "dg_k" };
    let rects = "";
    rows.forEach((row, y) => {
        let x = 0;
        while (x < row.length) {
            const c = row[x];
            if (!cls[c]) { x++; continue; }
            let w = 1;
            while (row[x + w] === c) w++;
            rects += `<rect class="${cls[c]}" x="${x}" y="${y}" width="${w}" height="1"/>`;
            x += w;
        }
    });
    return `<svg viewBox="0 0 16 16" shape-rendering="crispEdges">${rects}</svg>`;
}

class QuickSettings {
    constructor() {
        const fs = require("fs");
        const path = require("path");

        this.discordUrl = null;
        this.socials = { links: [], discordGist: "" };
        try {
            this.socials = JSON.parse(fs.readFileSync(path.join(__dirname, "assets", "misc", "socials.json")).toString());
        } catch (e) {
            console.warn("socials.json not readable: " + e);
        }

        this.gear = `<path d="M 11.9994,15.498C 10.0664,15.498 8.49939,13.931 8.49939,11.998C 8.49939,10.0651 10.0664,8.49805 11.9994,8.49805C 13.9324,8.49805 15.4994,10.0651 15.4994,11.998C 15.4994,13.931 13.9324,15.498 11.9994,15.498 Z M 19.4284,12.9741C 19.4704,12.6531 19.4984,12.329 19.4984,11.998C 19.4984,11.6671 19.4704,11.343 19.4284,11.022L 21.5414,9.36804C 21.7294,9.21606 21.7844,8.94604 21.6594,8.73004L 19.6594,5.26605C 19.5354,5.05005 19.2734,4.96204 19.0474,5.04907L 16.5584,6.05206C 16.0424,5.65607 15.4774,5.32104 14.8684,5.06903L 14.4934,2.41907C 14.4554,2.18103 14.2484,1.99805 13.9994,1.99805L 9.99939,1.99805C 9.74939,1.99805 9.5434,2.18103 9.5054,2.41907L 9.1304,5.06805C 8.52039,5.32104 7.95538,5.65607 7.43939,6.05206L 4.95139,5.04907C 4.7254,4.96204 4.46338,5.05005 4.33939,5.26605L 2.33939,8.73004C 2.21439,8.94604 2.26938,9.21606 2.4574,9.36804L 4.5694,11.022C 4.5274,11.342 4.49939,11.6671 4.49939,11.998C 4.49939,12.329 4.5274,12.6541 4.5694,12.9741L 2.4574,14.6271C 2.26938,14.78 2.21439,15.05 2.33939,15.2661L 4.33939,18.73C 4.46338,18.946 4.7254,19.0341 4.95139,18.947L 7.4404,17.944C 7.95639,18.34 8.52139,18.675 9.1304,18.9271L 9.5054,21.577C 9.5434,21.8151 9.74939,21.998 9.99939,21.998L 13.9994,21.998C 14.2484,21.998 14.4554,21.8151 14.4934,21.577L 14.8684,18.9271C 15.4764,18.6741 16.0414,18.34 16.5574,17.9431L 19.0474,18.947C 19.2734,19.0341 19.5354,18.946 19.6594,18.73L 21.6594,15.2661C 21.7844,15.05 21.7294,14.78 21.5414,14.6271L 19.4284,12.9741 Z"/>`;
        this.addButton();
    }

    addButton() {
        if (document.getElementById("rootui_gear")) return;
        const btn = document.createElement("div");
        btn.id = "rootui_gear";
        btn.title = "RootUI settings - themes, theme maker & links";
        btn.innerHTML = dragonSvg();
        btn.addEventListener("mousedown", e => e.stopPropagation());
        btn.addEventListener("click", () => this.open());
        document.body.appendChild(btn);

        // Sits right above terminal tab #1, in the free strip over the shell frame (never touching its line).
        const place = () => {
            const shell = document.getElementById("main_shell");
            const r = shell ? shell.getBoundingClientRect() : null;
            const vh = window.innerHeight / 100;
            if (!r || !r.width) { // no shell on screen: park it in the top-left corner
                btn.style.left = Math.round(1.2 * vh) + "px";
                btn.style.visibility = "visible";
                return;
            }
            const size = btn.offsetHeight || 2.6 * vh;
            const gap = 0.8 * vh;
            btn.style.left = Math.round(r.left + 1.2 * vh) + "px";
            btn.style.top = Math.max(0.3 * vh, Math.round(r.top - size - gap)) + "px";
            btn.style.visibility = "visible";
        };
        this.placeButton = place;
        place();
        window.addEventListener("resize", place);
        const shell = document.getElementById("main_shell");
        if (shell && window.ResizeObserver) new ResizeObserver(place).observe(shell);
        [300, 800, 1600].forEach(ms => setTimeout(place, ms)); // after the shell's own size transition
    }

    // [{name, r, g, b, bg, custom, pixelIcon}] for every theme file in the themes folder
    listThemes() {
        const fs = require("fs");
        const path = require("path");
        const out = [];
        let files = [];
        try { files = fs.readdirSync(themesDir); } catch (e) { return out; }
        files.forEach(f => {
            if (!f.endsWith(".json")) return;
            try {
                const t = JSON.parse(fs.readFileSync(path.join(themesDir, f)).toString());
                out.push({
                    name: f.slice(0, -5),
                    r: t.colors.r, g: t.colors.g, b: t.colors.b,
                    bg: t.colors.light_black || "#000000",
                    custom: !!t.custom,
                    pixelIcon: t.pixelIcon && Array.isArray(t.pixelIcon.rows) ? t.pixelIcon.rows : null
                });
            } catch (e) { /* skip broken theme files */ }
        });
        out.sort((a, b) => a.name.localeCompare(b.name));
        return out;
    }

    themeGridHtml() {
        const current = window.settings.theme;
        let themes = "";
        this.listThemes().forEach(t => {
            const n = window._escapeHtml(t.name);
            const tools = t.custom
                ? `<b class="rt_tool" title="Edit theme" onclick="event.stopPropagation();window.themeMaker.open(this.closest('.rootui_theme').dataset.theme)">&#9998;</b>
                   <b class="rt_tool" title="Export theme" onclick="event.stopPropagation();window.themeMaker.exportTheme(this.closest('.rootui_theme').dataset.theme)">&#8613;</b>
                   <b class="rt_tool" title="Delete theme" onclick="event.stopPropagation();window.themeMaker.remove(this)">&#10005;</b>`
                : "";
            themes += `<div class="rootui_theme${t.name === current ? " current" : ""}${t.custom ? " custom" : ""}" onclick="window.setTheme(this.dataset.theme)" data-theme="${n}" title="${n}">
                <i style="background:${window._escapeHtml(String(t.bg))}">${pixelThemeIcon(t.name, t.r, t.g, t.b, t.bg, t.pixelIcon)}</i><span>${n}</span>${tools}</div>`;
        });
        return themes;
    }

    // re-draw the theme list if the settings panel is open (after saving / importing / deleting a theme)
    refreshThemes() {
        const grid = document.querySelector("#rootui_panel .rootui_themes");
        if (grid) grid.innerHTML = this.themeGridHtml();
    }

    open() {
        if (document.getElementById("rootui_panel")) return;
        let links = `<a class="rootui_link discord disabled" id="rootui_discord" data-url="">
                <span>Discord server<br><small id="rootui_discord_hint">looking up the invite link...</small></span><span>&#9654;</span></a>`;
        (this.socials.links || []).forEach(l => {
            links += `<a class="rootui_link" data-url="${window._escapeHtml(String(l.url))}" onclick="window.rootuiOpen(this)">
                ${window._escapeHtml(String(l.label))}<small>${window._escapeHtml(String(l.handle || ""))}</small></a>`;
        });

        const modal = new Modal({
            type: "custom",
            title: `RootUI <i>settings</i>`,
            html: `<div id="rootui_panel">
                <h4 class="rootui_h">THEME
                    <span class="rootui_h_actions">
                        <a onclick="window.themeMaker.open()">+ Create theme</a>
                        <a onclick="window.themeMaker.openImport()">Import</a>
                    </span></h4>
                <div class="rootui_themes">${this.themeGridHtml()}</div>
                <h4 class="rootui_h">FOLLOW HAFIZ IBRAHIM</h4>
                <div class="rootui_links">${links}</div>
            </div>`,
            buttons: [
                { label: "All settings", action: "document.querySelector('#rootui_panel').closest('.modal_popup').querySelector('button:last-child').click();window.openSettings();" }
            ]
        });

        this.loadDiscord();
        return modal;
    }

    // The invite link lives in a gist so it can be changed without shipping a new build.
    async loadDiscord() {
        const el = () => document.getElementById("rootui_discord");
        const hint = () => document.getElementById("rootui_discord_hint");
        const apply = url => {
            if (!el()) return;
            el().dataset.url = url;
            el().classList.remove("disabled");
            el().setAttribute("onclick", "window.rootuiOpen(this)");
            hint().innerText = url.replace(/^https?:\/\//, "");
        };
        if (this.discordUrl) apply(this.discordUrl);
        try {
            const text = await window.__gistText(this.socials.discordGist);
            let m = text.match(/https?:\/\/(?:www\.)?(?:discord\.gg|discord(?:app)?\.com\/invite)\/[A-Za-z0-9_-]+/i)
                 || text.match(/https:\/\/[^\s"'<>]+/i);
            let url = m ? m[0] : null;
            if (!url && /^[A-Za-z0-9_-]{4,32}$/.test(text.trim())) url = "https://discord.gg/" + text.trim();
            if (!url) throw new Error("no link in gist");
            this.discordUrl = url;
            apply(url);
        } catch (e) {
            if (!this.discordUrl && hint()) hint().innerText = "unavailable (offline?)";
        }
    }
}

window.rootuiOpen = el => {
    const url = el && el.dataset ? el.dataset.url : "";
    if (url && /^https?:\/\//i.test(url)) electron.shell.openExternal(url);
};

// Switch theme: remember it in settings.json, then hot-reload with the new theme.
window.setTheme = name => {
    if (!name) return;
    try {
        const raw = JSON.parse(fs.readFileSync(settingsFile).toString());
        raw.theme = name;
        fs.writeFileSync(settingsFile, JSON.stringify(raw, "", 4));
        window.settings.theme = name;
    } catch (e) {
        console.warn("Could not save theme to settings.json: " + e);
    }
    window.audioManager.theme.play();
    window.themeChanger(name);
};

module.exports = { QuickSettings };
