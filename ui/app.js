"use strict";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const appWindow = window.__TAURI__.window.getCurrentWindow();

const COLORS = ["grass", "amber", "sky", "redstone", "amethyst"];
const ICON_NAMES = { block: "Blocco", grass: "Erba", gem: "Gemma", pickaxe: "Piccone", sword: "Spada", tree: "Albero", potion: "Pozione", heart: "Cuore", house: "Casa", flame: "Fiamma" };
const LOADERS = { vanilla: "Vanilla", fabric: "Fabric", optifine: "OptiFine" };
const KINDS = { mod: "Mod", resourcepack: "Texture pack", shader: "Shader" };
const KIND_LOWER = { mod: "mod", resourcepack: "texture pack", shader: "shader" };
const STAGES = {
  "Signing in": "Accesso all'account", "Reading version": "Lettura versione", "Libraries": "Librerie",
  "Assets": "Risorse di gioco", "Fabric": "Fabric", "OptiFine": "Installazione OptiFine", "Starting": "Avvio del gioco",
};

const S = {
  profiles: [], accounts: [], settings: {}, running: new Set(), busy: new Set(), dataDir: "",
  tasks: {}, logs: {}, view: { kind: "loading" },
  versions: null, fabric: null, optifine: null, sysMem: 8192,
  content: {}, mode: "installed", browse: null, installing: new Set(),
  pano: {}, skin: undefined, capes: null, custom: null, customV: Date.now(), capeEdit: null, login: null,
};

const $ = (sel, root = document) => root.querySelector(sel);
const esc = escHtml;
const icon = (name, cls = "") => `<svg class="${cls}" aria-hidden="true"><use href="#i-${name}"/></svg>`;
/** A "?" that shows `text` on hover or keyboard focus. */
const tip = (text, cls = "") => `<span class="tip ${cls}" tabindex="0" role="note" aria-label="${esc(text)}" data-tip="${esc(text)}">${icon("help")}</span>`;
/** Image URL for a file inside the data folder, served by the app's `agate://` protocol. */
const fileUrl = (p) => {
  const rel = p.startsWith(S.dataDir) ? p.slice(S.dataDir.length).replace(/^[\\/]+/, "") : p;
  return `http://agate.localhost/${rel.split(/[\\/]/).map(encodeURIComponent).join("/")}`;
};
const fmtMem = (mb) => (mb >= 1024 ? `${+(mb / 1024).toFixed(1)} GB` : `${mb} MB`);
const fmtNum = (n) => (n >= 1e6 ? `${(n / 1e6).toFixed(1)} M` : n >= 1e3 ? `${Math.round(n / 1e3)} k` : `${n}`);
const fmtBytes = (b) => (b >= 1 << 20 ? `${(b / (1 << 20)).toFixed(1)} MB` : `${Math.max(1, Math.round(b / 1024))} KB`);
const ofLabel = (file) => file ? file.replace(/^preview_/, "").replace(/^OptiFine_[^_]+_/, "").replace(/\.jar$/, "").replace(/_/g, " ") : tp("ultima versione");
const stagger = (i) => `style="--i:${Math.min(i, 12)}"`;

function ago(ts) {
  if (!ts) return null;
  const s = Date.now() / 1000 - ts;
  if (s < 60) return tp("ora");
  if (s < 3600) return tp("{n} min fa", { n: Math.floor(s / 60) });
  if (s < 86400) return tp("{n} h fa", { n: Math.floor(s / 3600) });
  const d = Math.floor(s / 86400);
  return d === 1 ? tp("ieri") : d < 30 ? tp("{n} giorni fa", { n: d }) : new Date(ts * 1000).toLocaleDateString(LANG === "en" ? "en-GB" : "it-IT");
}

function toast(msg, kind = "ok") {
  const el = document.createElement("div");
  el.className = `toast ${kind}`;
  el.innerHTML = `${icon(kind === "ok" ? "check" : "warn")}<div>${esc(msg)}</div>`;
  $("#toasts").append(el);
  setTimeout(() => { el.classList.add("out"); setTimeout(() => el.remove(), 260); }, kind === "ok" ? 3500 : 9000);
}

async function call(cmd, args) {
  try { return await invoke(cmd, args); }
  catch (e) { toast(String(e), "bad"); throw e; }
}

function confirmDialog(title, body, okLabel) {
  const d = $("#confirm");
  $("#confirm-title").textContent = title;
  $("#confirm-body").textContent = body;
  $("#confirm-ok").textContent = okLabel;
  $("#confirm-cancel").textContent = tp("Annulla");
  d.returnValue = "";
  d.showModal();
  return new Promise((ok) => d.addEventListener("close", () => ok(d.returnValue === "ok"), { once: true }));
}

const profile = (id) => S.profiles.find((p) => p.id === id);
const activeAccount = () => S.accounts.find((a) => a.active);
const current = () => (S.view.kind === "profile" ? profile(S.view.id) : null);

async function refresh() {
  const snap = await invoke("snapshot");
  S.profiles = snap.profiles.sort((a, b) => (b.last_played || b.created) - (a.last_played || a.created));
  S.accounts = snap.accounts;
  S.settings = snap.settings;
  S.running = new Set(snap.running);
  S.busy = new Set(snap.busy);
  S.dataDir = snap.data_dir;
  S.version = snap.version;
  setLang(S.settings.language);
  applyMotion();
}

/* Animations: "on" always, "off" never, "auto" follows Windows' animation effects setting. */
const systemReduce = matchMedia("(prefers-reduced-motion: reduce)");
function applyMotion() {
  const m = S.settings.motion || "auto";
  document.documentElement.classList.toggle("reduce-motion", m === "off" || (m === "auto" && systemReduce.matches));
}
systemReduce.addEventListener("change", applyMotion);

async function loadSkin() {
  S.skin = activeAccount() ? await invoke("account_skin").catch(() => null) : null;
  S.custom = await invoke("custom_cape").catch(() => null);
  S.customV = Date.now();
  S.capes = null;
}

/** The cape you see on yourself: your custom one when it is on, else the Microsoft one. */
const visibleCape = () => S.settings.custom_cape && S.custom ? `${fileUrl(S.custom.path)}?v=${S.customV}`
  : S.skin?.cape ? fileUrl(S.skin.cape) : null;
const figureFor = (s, opts = {}) => Skin.figure(fileUrl(S.skin.path), { slim: S.skin.slim, s, cape: visibleCape(), ...opts });

/* ---------------- window chrome ---------------- */

function paintChrome() {
  const labels = { min: "Riduci a icona", max: "Ingrandisci", close: "Chiudi" };
  document.querySelectorAll("[data-win]").forEach((b) => b.setAttribute("aria-label", tp(labels[b.dataset.win])));
  $("#rail-title").textContent = tp("Profili");
  $("#new-profile").title = `${tp("Nuovo profilo")} (Ctrl+N)`;
  $("#new-profile").setAttribute("aria-label", tp("Nuovo profilo"));
  $("#settings-btn").title = `${tp("Impostazioni")} (Ctrl+,)`;
  $("#settings-btn").setAttribute("aria-label", tp("Impostazioni"));
}

document.querySelectorAll("[data-win]").forEach((b) => b.addEventListener("click", () => {
  const w = b.dataset.win;
  if (w === "min") appWindow.minimize();
  else if (w === "max") appWindow.toggleMaximize();
  else appWindow.close();
}));
$("#main").addEventListener("scroll", () => {
  const hero = $(".hero");
  const limit = hero ? hero.offsetHeight - 40 : 4;
  $(".titlebar").classList.toggle("solid", $("#main").scrollTop > limit);
}, { passive: true });
/* Floating scrollbar for the page: drawn over the content, so showing it never resizes the layout. */
const thumb = $("#scroll-thumb");
let thumbTimer, thumbFrame = 0;
function paintThumb() {
  thumbFrame = 0;
  const m = $("#main");
  const { scrollTop, scrollHeight, clientHeight } = m;
  const bar = 36, pad = 4, track = clientHeight - bar - pad * 2;
  if (scrollHeight <= clientHeight + 1) { thumb.style.display = "none"; return; }
  thumb.style.display = "";
  const h = Math.max(40, (track * clientHeight) / scrollHeight);
  thumb.style.height = `${h}px`;
  thumb.style.transform = `translateY(${bar + pad + ((track - h) * scrollTop) / (scrollHeight - clientHeight)}px)`;
  thumb.dataset.ratio = String((scrollHeight - clientHeight) / (track - h));
}
const queueThumb = () => { thumbFrame ||= requestAnimationFrame(paintThumb); };
$("#main").addEventListener("scroll", () => {
  queueThumb();
  thumb.classList.add("show");
  clearTimeout(thumbTimer);
  thumbTimer = setTimeout(() => thumb.classList.remove("show"), 900);
}, { passive: true });
$("#main").addEventListener("load", queueThumb, true);
new ResizeObserver(queueThumb).observe($("#main"));
new MutationObserver(queueThumb).observe($("#main"), { childList: true, subtree: true });
thumb.addEventListener("pointerdown", (e) => {
  const start = { y: e.clientY, top: $("#main").scrollTop, ratio: +thumb.dataset.ratio || 1 };
  thumb.setPointerCapture(e.pointerId);
  thumb.classList.add("drag");
  const move = (ev) => { $("#main").scrollTop = start.top + (ev.clientY - start.y) * start.ratio; };
  const up = () => { thumb.classList.remove("drag"); thumb.removeEventListener("pointermove", move); };
  thumb.addEventListener("pointermove", move);
  thumb.addEventListener("pointerup", up, { once: true });
});

document.addEventListener("visibilitychange", () => document.body.classList.toggle("paused", document.hidden));

/* ---------------- progress ---------------- */

function taskInfo(id) {
  const t = S.tasks[id];
  if (!t) return null;
  let p = null;
  if (t.total_bytes > 0) p = Math.min(1, t.bytes / t.total_bytes);
  else if (t.total > 0) p = t.done / t.total;
  const java = /^Extracting Java (\d+)/.exec(t.label);
  const label = java ? tp("Estrazione Java {v}", { v: java[1] }) : tp(STAGES[t.label] || t.label);
  let sub = "";
  if (t.total > 0) sub = t.total_bytes > 0 ? tp("{a} di {b}", { a: fmtBytes(t.bytes), b: fmtBytes(t.total_bytes) }) : tp("{a} di {b} file", { a: t.done, b: t.total });
  else if (t.bytes > 0) sub = fmtBytes(t.bytes);
  return { p, label, sub };
}

function profileState(p) {
  if (S.running.has(p.id)) return { text: tp("In gioco"), live: true };
  if (S.busy.has(p.id)) {
    const t = taskInfo(p.id);
    return { text: t && t.p != null ? `${Math.round(t.p * 100)}%` : "…", live: true };
  }
  return { text: ago(p.last_played) || tp("Nuovo"), live: false };
}

/** Repaints only progress-bearing nodes, so a download never rebuilds the page. */
function paintProgress(id) {
  const p = profile(id);
  if (!p) return;
  const tab = document.querySelector(`.tab[data-id="${id}"]`);
  if (tab) {
    const st = profileState(p);
    const s = $(".tab-state", tab);
    s.textContent = st.text;
    s.classList.toggle("live", st.live);
    const t = taskInfo(id);
    let bar = $(".tab-bar", tab);
    if (S.busy.has(id) && !S.running.has(id) && t) {
      if (!bar) { tab.insertAdjacentHTML("beforeend", `<span class="tab-bar"><i></i></span>`); bar = $(".tab-bar", tab); }
      bar.firstChild.style.width = `${(t.p ?? 0) * 100}%`;
    } else if (bar) bar.remove();
  }
  if (current()?.id === id) {
    const btn = $(".play");
    if (btn) btn.outerHTML = playHtml(p);
  }
}

/* ---------------- rail ---------------- */

let railIntro = true;
function renderRail() {
  const list = $("#profile-list");
  if (railIntro) {
    railIntro = false;
    list.classList.add("intro");
    setTimeout(() => list.classList.remove("intro"), 900);
  }
  list.innerHTML = S.profiles.length ? S.profiles.map((p, i) => {
    const st = profileState(p);
    const cur = current()?.id === p.id;
    return `<button class="tab" data-id="${p.id}" data-color="${p.color}" ${cur ? 'aria-current="page"' : ""} ${stagger(i)}>
      ${Art.icon(p.icon || "block", p.color)}
      <span style="min-width:0"><div class="tab-name">${esc(p.name)}</div><div class="tab-meta">${esc(p.mc)} · ${LOADERS[p.loader]}</div></span>
      <span class="tab-state ${st.live ? "live" : ""}">${esc(st.text)}</span>
    </button>`;
  }).join("") : `<p class="rail-empty">${t("Nessun profilo ancora.")}</p>`;
  S.profiles.forEach((p) => S.busy.has(p.id) && paintProgress(p.id));
  const a = activeAccount();
  $("#account-btn").innerHTML = a
    ? `<span class="avatar" id="rail-avatar">${esc(a.name[0].toUpperCase())}</span><span style="min-width:0"><div class="account-name">${esc(a.name)}</div><div class="account-kind">${t(a.kind === "msa" ? "Microsoft" : "Offline")}</div></span>`
    : `<span class="avatar">${icon("user")}</span><span><div class="account-name">${t("Accedi")}</div><div class="account-kind">${t("Nessun account")}</div></span>`;
  if (a && S.skin) Skin.head(fileUrl(S.skin.path), 32).then((c) => $("#rail-avatar")?.replaceChildren(c)).catch(() => {});
  $("#settings-btn").setAttribute("aria-pressed", S.view.kind === "settings");
}

$("#profile-list").addEventListener("click", (e) => {
  const t = e.target.closest(".tab");
  if (t && current()?.id !== t.dataset.id) go({ kind: "profile", id: t.dataset.id, tab: defaultTab(profile(t.dataset.id)) });
});
$("#new-profile").addEventListener("click", () => go({ kind: "new" }));
$("#settings-btn").addEventListener("click", () => go({ kind: "settings" }));
$("#account-btn").addEventListener("click", () => go({ kind: "settings" }));

const defaultTab = (p) => (p.loader === "fabric" ? "mod" : p.loader === "optifine" ? "shader" : "resourcepack");

function go(view) {
  // Remember the last profile, so Esc from Settings or New profile returns to it.
  if (S.view.kind === "profile") S.lastProfile = { ...S.view };
  S.view = view;
  if (view.kind !== "new") S.importer = null;
  S.browse = null;
  S.mode = "installed";
  renderRail();
  fadeSwap($("#main"), () => {
    render();
    $("#main").scrollTop = 0;
    $(".titlebar").classList.remove("solid");
  });
}

/** Fades the old content out, then runs `swap` (which draws the new one, entering with `rise`). */
function fadeSwap(el, swap) {
  if (!el || !el.firstChild || document.documentElement.classList.contains("reduce-motion")) return swap();
  el.classList.add("leaving");
  setTimeout(() => { el.classList.remove("leaving"); swap(); }, 130);
}

function render() {
  paintChrome();
  renderRail();
  const m = $("#main");
  const v = S.view;
  m.onclick = null;
  if (v.kind === "settings") { m.innerHTML = settingsPage(); bindSettings(); return; }
  if (v.kind === "new" || (v.kind !== "loading" && !S.profiles.length)) { m.innerHTML = newPage(); bindProfileForm(null); return; }
  const p = current();
  if (p) { document.documentElement.dataset.color = p.color; m.innerHTML = profilePage(p); mountHero(p); bindProfile(p); renderTab(p); moveInk(true); return; }
  if (S.profiles.length) go({ kind: "profile", id: S.profiles[0].id, tab: defaultTab(S.profiles[0]) });
}

/* ---------------- profile: hero ---------------- */

function playHtml(p) {
  if (S.running.has(p.id)) return `<button class="play running" data-act="stop">${icon("stop")}<span>${t("Ferma")}<span class="sub">${t("Il gioco è aperto")}</span></span></button>`;
  if (S.busy.has(p.id)) {
    const k = taskInfo(p.id) || { p: null, label: tp("Preparazione"), sub: "" };
    return `<button class="play busy" style="--p:${k.p ?? 0}" aria-busy="true" aria-disabled="true">${icon("down")}<span>${esc(k.label)}<span class="sub">${esc(k.sub || tp("In corso…"))}</span></span></button>`;
  }
  return `<button class="play" data-act="play" title="${esc(tp("Gioca"))} (${keyName("Ctrl+Invio")})">${icon("play")}${t("Gioca")}</button>`;
}

function profilePage(p) {
  const a = activeAccount();
  const loader = p.loader === "vanilla" ? "Vanilla" : `${LOADERS[p.loader]} <b>${esc(p.loader === "optifine" ? ofLabel(p.loader_version) : p.loader_version || tp("ultima versione"))}</b>`;
  const tabs = ["mod", "resourcepack", "shader", "worlds", "shots", "logs", "edit"];
  const names = { mod: "Mod", resourcepack: "Texture pack", shader: "Shader", worlds: "Mondi e server", shots: "Screenshot", logs: "Log", edit: "Profilo" };
  return `
  <header class="hero" data-color="${p.color}">
    <div class="hero-text">
      <h1>${esc(p.name)}</h1>
      <div class="facts"><span>Minecraft <b>${esc(p.mc)}</b></span><span>${loader}</span><span>${t("Memoria")} <b>${fmtMem(p.memory_mb)}</b></span><span>${p.last_played ? t("Ultima partita {_when}", { _when: `<b>${esc(ago(p.last_played))}</b>` }) : t("Mai avviato")}</span></div>
      <div class="hero-actions">
        ${playHtml(p)}
        <button class="icon-btn" data-act="folder" title="${esc(tp("Apri la cartella del profilo"))}" aria-label="${esc(tp("Apri la cartella del profilo"))}">${icon("folder")}</button>
        <span class="as">${a ? t("come {name}", { name: a.name }) : t("Serve un account per giocare")}</span>
      </div>
    </div>
    <div class="hero-figure" id="hero-figure"></div>
  </header>
  <div class="tabs" role="tablist" data-color="${p.color}">
    ${tabs.map((k) => `<button role="tab" data-tab="${k}" aria-selected="${S.view.tab === k}">${t(names[k])}</button>`).join("")}
    <span class="ink" aria-hidden="true"></span>
  </div>
  <div class="page" id="tab-body" data-color="${p.color}"></div>`;
}

/** Slides the tab underline to the selected tab; `instant` places it without sliding (new page). */
function moveInk(instant = false) {
  const sel = $('.tabs [aria-selected="true"]'), ink = $(".tabs .ink");
  if (!sel || !ink) return;
  if (instant) ink.style.transition = "none";
  ink.style.width = `${sel.offsetWidth - 24}px`;
  ink.style.transform = `translateX(${sel.offsetLeft + 12}px)`;
  if (instant) { ink.getBoundingClientRect(); ink.style.transition = ""; }
}
window.addEventListener("resize", moveInk);

async function mountHero(p) {
  const hero = $(".hero");
  if (!(p.mc in S.pano)) S.pano[p.mc] = await invoke("panorama", { mc: p.mc }).catch(() => null);
  const pano = S.pano[p.mc];
  if (pano && hero.isConnected) Pano.mount(hero, pano.faces.map(fileUrl));
  new IntersectionObserver(([e]) => hero.classList.toggle("paused", !e.isIntersecting)).observe(hero);
  mountFigure(p);
}

/** The player standing in the hero; runs again once the skin has loaded. */
async function mountFigure(p) {
  const a = activeAccount(), hero = $(".hero");
  if (!a || !S.skin || !hero) return;
  const s = Math.max(5, Math.min(8, Math.floor(hero.clientHeight / 62)));
  const fig = await figureFor(s).catch(() => null);
  const host = $("#hero-figure");
  if (fig && host && current()?.id === p.id) host.replaceChildren(Object.assign(document.createElement("span"), { className: "nametag", textContent: a.name }), fig);
}

function bindProfile(p) {
  $("#main").onclick = async (e) => {
    const b = e.target.closest("[data-act],[data-tab],[data-mode],[data-install],[data-remove],[data-toggle],[data-page],[data-world],[data-server],[data-open]");
    if (!b || b.getAttribute("aria-disabled") === "true") return;
    const pp = profile(p.id);
    if (b.dataset.tab) {
      S.view.tab = b.dataset.tab; S.mode = "installed"; S.browse = null;
      document.querySelectorAll(".tabs [data-tab]").forEach((x) => x.setAttribute("aria-selected", x === b));
      moveInk();
      return fadeSwap($("#tab-body"), () => renderTab(profile(p.id)));
    }
    if (b.dataset.mode) { S.mode = b.dataset.mode; renderTab(pp); if (S.mode === "browse") runSearch(pp); return; }
    if (b.dataset.install) return installContent(pp, b.dataset.install);
    if (b.dataset.remove) { await call("content_remove", { id: p.id, kind: S.view.tab, file: b.dataset.remove }); return loadContent(pp); }
    if (b.dataset.toggle) { await call("content_toggle", { id: p.id, kind: S.view.tab, file: b.dataset.toggle }); return loadContent(pp); }
    if (b.dataset.page) { S.browse.offset = Math.max(0, S.browse.offset + Number(b.dataset.page)); return runSearch(pp); }
    if (b.dataset.world) return play(pp, { world: b.dataset.world });
    if (b.dataset.server) return play(pp, { server: b.dataset.server });
    if (b.dataset.open) return call("open_path", { path: b.dataset.open });
    const act = b.dataset.act;
    if (act === "play") return play(pp);
    if (act === "stop") return call("stop", { id: p.id });
    if (act === "folder") return call("open_folder", { id: p.id });
    if (act === "delete") return deleteProfile(pp);
    if (act === "copy-log") { navigator.clipboard.writeText((S.logs[p.id] || []).join("\n")); return toast(tp("Log copiato")); }
    if (act === "to-edit") { $(`.tabs [data-tab="edit"]`).click(); }
  };
}

function renderTab(p) {
  const body = $("#tab-body");
  if (!body) return;
  const k = S.view.tab;
  if (KINDS[k]) { body.innerHTML = contentTab(p, k); bindSearchBox(p); if (!S.content[p.id]?.[k]) loadContent(p); }
  else if (k === "worlds") { body.innerHTML = `<div class="worlds">${'<div class="skeleton" style="height:90px"></div>'.repeat(3)}</div>`; loadWorlds(p); }
  else if (k === "shots") { body.innerHTML = `<div class="shots">${'<div class="skeleton" style="aspect-ratio:16/9"></div>'.repeat(4)}</div>`; loadShots(p); }
  else if (k === "logs") { body.innerHTML = logsTab(p); loadLogs(p); }
  else { body.innerHTML = editTab(p); bindProfileForm(p); }
}

/** Starts the profile; `quick` jumps straight into { world } or { server } (1.20+). */
async function play(p, quick = {}) {
  if (!activeAccount()) { toast(tp("Aggiungi un account prima di giocare"), "bad"); return go({ kind: "settings" }); }
  S.busy.add(p.id);
  S.tasks[p.id] = { label: "Preparazione", done: 0, total: 0, bytes: 0, total_bytes: 0 };
  paintProgress(p.id);
  try {
    await invoke("play", { id: p.id, world: quick.world ?? null, server: quick.server ?? null });
  } catch (e) {
    S.busy.delete(p.id);
    delete S.tasks[p.id];
    paintProgress(p.id);
    toast(String(e), "bad");
  }
}

async function deleteProfile(p) {
  const ok = await confirmDialog(tp("Eliminare “{name}”?", { name: p.name }), tp("Verranno cancellati mondi, mod e configurazione di questo profilo. Le impostazioni condivise e gli altri profili restano."), tp("Elimina profilo"));
  if (!ok) return;
  await call("delete_profile", { id: p.id });
  delete S.content[p.id];
  await refresh();
  toast(tp("Profilo eliminato"));
  go(S.profiles.length ? { kind: "profile", id: S.profiles[0].id, tab: defaultTab(S.profiles[0]) } : { kind: "new" });
}

/* ---------------- content (mods, resource packs, shaders) ---------------- */

function unavailable(p, kind) {
  const box = (title, body) => `<div class="empty"><h4>${t(title)}</h4><p>${t(body)}</p><button class="btn" data-act="to-edit">${t("Cambia caricatore")}</button></div>`;
  if (kind === "mod" && p.loader === "optifine") return box("OptiFine non usa mod", "Le mod di Modrinth funzionano con i profili <b>Fabric</b>. Per prestazioni e shader con Fabric installa <b>Sodium</b> e <b>Iris</b>: Iris arriva da solo quando aggiungi uno shader.");
  if (kind === "mod" && p.loader === "vanilla") return box("Questo profilo è vanilla", "Passa il profilo a Fabric per installare mod da Modrinth con un clic: le dipendenze arrivano da sole.");
  if (kind === "shader" && p.loader === "vanilla") return box("Gli shader richiedono Fabric o OptiFine", "Con Fabric il launcher installa Iris in automatico insieme allo shader che scegli. Con OptiFine gli shader funzionano subito.");
  return null;
}

function contentTab(p, kind) {
  const blocked = unavailable(p, kind);
  if (blocked) return blocked;
  const items = S.content[p.id]?.[kind];
  const browse = S.mode === "browse";
  return `<div class="toolbar">
      <div class="segmented" role="group">
        <button data-mode="installed" aria-pressed="${!browse}">${t("Installati")}${items ? ` <span class="count">${items.length}</span>` : ""}</button>
        <button data-mode="browse" aria-pressed="${browse}">${icon("search")}${t("Sfoglia Modrinth")}</button>
      </div>
      <span class="spacer"></span>
      ${browse ? `<div class="search">${icon("search")}<input id="q" class="input" placeholder="${esc(tp("Cerca {what} per {mc}", { what: tp(KIND_LOWER[kind]), mc: p.mc }))}" value="${esc(S.browse?.q || "")}" autocomplete="off" spellcheck="false"></div>` : ""}
    </div>
    ${S.running.has(p.id) && kind !== "mod" ? `<div class="note" style="margin-bottom:14px">${icon("warn")}<span>${t("Il gioco è aperto: cambia texture pack e shader dal gioco, o chiudilo prima.")}</span></div>` : ""}
    <div id="content-body">${browse ? browseHtml(p, kind) : installedHtml(p, kind, items)}</div>`;
}

function installedHtml(p, kind, items) {
  if (!items) return `<div class="list">${'<div class="skeleton skel-row"></div>'.repeat(3)}</div>`;
  if (!items.length) {
    const what = {
      mod: ["Nessuna mod installata", t("Sfoglia Modrinth per trovare mod per Minecraft {mc}. Fabric API e le altre dipendenze arrivano da sole.", { mc: p.mc })],
      resourcepack: ["Nessun texture pack", t("Cambia l'aspetto di blocchi, oggetti e interfaccia. Quello che installi viene attivato subito.")],
      shader: ["Nessuno shader", t(p.loader === "fabric" ? "Luci, ombre e acqua realistiche. Iris viene installato in automatico con il primo shader." : "Luci, ombre e acqua realistiche, tramite OptiFine.")],
    }[kind];
    return `<div class="empty"><h4>${t(what[0])}</h4><p>${what[1]}</p><button class="btn primary" data-mode="browse">${icon("search")}${t("Sfoglia Modrinth")}</button></div>`;
  }
  const action = (m) => kind === "mod"
    ? `<label class="switch" title="${esc(tp(m.enabled ? "Disattiva" : "Attiva"))}"><input type="checkbox" data-toggle="${esc(m.file)}" ${m.enabled ? "checked" : ""} aria-label="${esc(tp("Attiva {name}", { name: m.title }))}"></label>`
    : `<button class="btn sm" data-toggle="${esc(m.file)}">${m.enabled ? `${icon("check", "accent")}${t("Attivo")}` : t("Usa")}</button>`;
  return `<div class="list">${items.map((m, i) => `
    <div class="item ${kind === "mod" && !m.enabled ? "off" : ""}" ${stagger(i)}>
      ${m.icon_url ? `<img class="thumb" src="${esc(m.icon_url)}" alt="" loading="lazy">` : `<span class="thumb">${icon(kind === "mod" ? "box" : kind === "shader" ? "sun" : "image")}</span>`}
      <div style="min-width:0"><div class="item-title">${esc(m.title)}</div><div class="meta">${esc(m.version || tp("Aggiunto a mano"))} · ${esc(m.file)}</div></div>
      <div class="actions">${action(m)}<button class="icon-btn" data-remove="${esc(m.file)}" title="${esc(tp("Rimuovi"))}" aria-label="${esc(tp("Rimuovi {name}", { name: m.title }))}">${icon("trash")}</button></div>
    </div>`).join("")}</div>`;
}

function browseHtml(p, kind) {
  const s = S.browse;
  if (!s || (!s.hits && !s.error)) return kind === "mod" ? `<div class="list">${'<div class="skeleton skel-row"></div>'.repeat(5)}</div>` : `<div class="cards">${'<div class="skeleton" style="height:250px"></div>'.repeat(6)}</div>`;
  if (s.error) return `<div class="note bad">${icon("warn")}<span>${t("Modrinth non risponde: {err}. Controlla la connessione e riprova.", { err: s.error })}</span></div>`;
  if (!s.hits.length) return `<div class="empty"><h4>${s.q ? t("Nessun risultato per “{q}”", { q: s.q }) : t("Nessun risultato")}</h4><p>${t("Niente di compatibile con Minecraft {mc}. Prova un nome più corto o in inglese.", { mc: p.mc })}</p></div>`;
  const installed = new Set((p.mods || []).map((m) => m.project_id));
  const btn = (h) => installed.has(h.project_id) ? `<span class="done">${icon("check")}${t("Installato")}</span>`
    : S.installing.has(h.project_id) ? `<button class="btn sm" disabled>${icon("down", "spin")}${t("Installo…")}</button>`
    : `<button class="btn sm" data-install="${esc(h.project_id)}">${icon("down", "accent")}${t("Installa")}</button>`;
  const iconImg = (h) => h.icon_url ? `<img class="thumb" src="${esc(h.icon_url)}" alt="" loading="lazy">` : `<span class="thumb">${icon("box")}</span>`;
  const page = Math.floor(s.offset / 20) + 1, pages = Math.ceil(s.total / 20);
  const body = kind === "mod"
    ? `<div class="list">${s.hits.map((h, i) => `<div class="item" ${stagger(i)}>${iconImg(h)}
        <div style="min-width:0"><div class="item-title">${esc(h.title)}<span class="by">${t("di {author}", { author: h.author })}</span></div><div class="item-desc">${esc(h.description)}</div><div class="meta">${t("{n} download", { n: fmtNum(h.downloads) })}</div></div>
        <div class="actions">${btn(h)}</div></div>`).join("")}</div>`
    : `<div class="cards">${s.hits.map((h, i) => {
        const img = h.featured_gallery || h.gallery[0];
        return `<article class="card" ${stagger(i)}>
          <div class="card-img">${img ? `<img src="${esc(img)}" alt="" loading="lazy">` : `<span class="fallback">${icon(kind === "shader" ? "sun" : "image")}</span>`}${iconImg(h)}</div>
          <div class="card-body"><div class="card-title">${esc(h.title)}</div><div class="item-desc">${esc(h.description)}</div></div>
          <div class="card-foot"><span class="meta">${t("{n} download", { n: fmtNum(h.downloads) })}</span><span class="spacer"></span>${btn(h)}</div>
        </article>`;
      }).join("")}</div>`;
  return `<div class="${s.loading ? "loading" : ""}">${body}</div>
    ${pages > 1 ? `<div class="row" style="margin-top:16px"><span class="meta">${t("Pagina {a} di {b}", { a: page, b: pages })}</span><span class="spacer"></span>
      <button class="btn sm" data-page="-20" ${s.offset === 0 ? "disabled" : ""}>${t("Precedenti")}</button>
      <button class="btn sm" data-page="20" ${page >= pages ? "disabled" : ""}>${t("Successivi")}</button></div>` : ""}`;
}

function paintContent(p) {
  const kind = S.view.tab;
  if (current()?.id !== p.id || !KINDS[kind]) return;
  const pp = profile(p.id);
  const box = $("#content-body");
  if (!box) return renderTab(pp);
  box.innerHTML = S.mode === "browse" ? browseHtml(pp, kind) : installedHtml(pp, kind, S.content[p.id]?.[kind]);
  if (!document.documentElement.classList.contains("reduce-motion")) box.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 200, easing: "ease-out" });
  const items = S.content[p.id]?.[kind];
  const seg = $('.segmented [data-mode="installed"]');
  if (seg && items) seg.innerHTML = `${t("Installati")} <span class="count">${items.length}</span>`;
}

function bindSearchBox(p) {
  const input = $("#q");
  if (!input) return;
  let timer;
  const run = () => { S.browse.q = input.value; S.browse.offset = 0; runSearch(profile(p.id)); };
  input.addEventListener("input", () => { clearTimeout(timer); timer = setTimeout(run, 320); });
  input.addEventListener("keydown", (e) => { if (e.key === "Enter") { clearTimeout(timer); run(); } });
  input.focus();
}

let searchSeq = 0;
async function runSearch(p) {
  const kind = S.view.tab;
  if (!S.browse || S.browse.kind !== kind) S.browse = { kind, q: "", hits: null, total: 0, offset: 0 };
  const b = S.browse;
  const seq = ++searchSeq;
  b.loading = true; b.error = null;
  if (!$("#q")) renderTab(p); else paintContent(p);
  try {
    const r = await invoke("content_search", { id: p.id, kind, query: b.q, offset: b.offset });
    if (seq !== searchSeq) return;
    b.hits = r.hits; b.total = r.total_hits;
  } catch (e) { if (seq === searchSeq) b.error = String(e); }
  b.loading = false;
  if (seq === searchSeq) paintContent(p);
}

async function loadContent(p) {
  const kind = S.view.tab;
  if (unavailable(p, kind)) return;
  try { (S.content[p.id] ||= {})[kind] = await invoke("content_list", { id: p.id, kind }); }
  catch (e) { (S.content[p.id] ||= {})[kind] = []; toast(String(e), "bad"); }
  paintContent(p);
}

async function installContent(p, project) {
  const kind = S.view.tab;
  S.installing.add(project);
  paintContent(p);
  try {
    const added = await invoke("content_install", { id: p.id, kind, project });
    const one = added[0] || tp("Contenuto");
    toast(added.length > 1 ? tp("Installati: {list}", { list: added.join(", ") }) : tp(kind === "mod" ? "{name} installata" : "{name} installato e attivato", { name: one }));
  } catch (e) { toast(String(e), "bad"); }
  S.installing.delete(project);
  await refresh();
  delete S.content[p.id];
  const pp = profile(p.id);
  for (const k of Object.keys(KINDS)) if (!unavailable(pp, k)) (S.content[p.id] ||= {})[k] = await invoke("content_list", { id: p.id, kind: k }).catch(() => []);
  paintContent(pp);
}

/* ---------------- worlds, screenshots, logs ---------------- */

async function loadWorlds(p) {
  const r = await invoke("worlds", { id: p.id }).catch(() => ({ worlds: [], servers: [], quick_play: false }));
  const body = $("#tab-body");
  if (!body || S.view.tab !== "worlds" || current()?.id !== p.id) return;
  const busy = S.busy.has(p.id) ? 'aria-disabled="true"' : "";
  const join = (attr, value) => r.quick_play ? `<button class="btn sm" ${attr}="${esc(value)}" ${busy}>${icon("play", "accent")}${t("Entra")}</button>` : "";
  const base = `${S.dataDir}\\instances\\${p.id}\\saves\\`;
  const worlds = r.worlds.length
    ? `<div class="worlds">${r.worlds.map((w, i) => `<div class="world" ${stagger(i)}>
        ${w.icon ? `<img src="${esc(fileUrl(w.icon))}" alt="">` : `<span class="noicon">${icon("globe")}</span>`}
        <div style="min-width:0"><div class="item-title">${esc(w.folder)}</div><div class="meta">${t("Giocato {_when}", { _when: esc(ago(w.modified) || "—") })}</div>
          <div class="actions">${join("data-world", w.folder)}
          <button class="icon-btn" data-open="${esc(base + w.folder)}" title="${esc(tp("Apri la cartella di {name}", { name: w.folder }))}" aria-label="${esc(tp("Apri la cartella di {name}", { name: w.folder }))}">${icon("folder")}</button></div>
        </div></div>`).join("")}</div>`
    : `<div class="empty"><h4>${t("Ancora nessun mondo")}</h4><p>${t("I mondi creati in questo profilo compaiono qui.")}</p></div>`;
  const servers = r.servers.length
    ? `<div class="worlds">${r.servers.map((sv, i) => `<div class="world" ${stagger(i)}>
        ${sv.icon ? `<img src="${esc(sv.icon)}" alt="">` : `<span class="noicon">${icon("server")}</span>`}
        <div style="min-width:0"><div class="item-title">${esc(sv.name)}</div><div class="meta">${esc(sv.ip)}</div>
          <div class="actions">${join("data-server", sv.ip)}</div>
        </div></div>`).join("")}</div>`
    : `<div class="empty"><h4>${t("Nessun server salvato")}</h4><p>${t("Aggiungi un server dal menu Multigiocatore del gioco: comparirà qui.")}</p></div>`;
  body.innerHTML = `${r.quick_play ? "" : `<p class="lead">${t("Per entrare direttamente in un mondo o in un server serve Minecraft 1.20 o successivo.")}</p>`}
    <section class="section"><h3 class="section-title">${t("Mondi")}</h3>${worlds}</section>
    <section class="section"><h3 class="section-title">${t("Server")}</h3>${servers}</section>`;
}

async function loadShots(p) {
  const shots = await invoke("screenshots", { id: p.id }).catch(() => []);
  const body = $("#tab-body");
  if (!body || S.view.tab !== "shots" || current()?.id !== p.id) return;
  body.innerHTML = shots.length
    ? `<div class="shots">${shots.map((s, i) => `<button class="shot" data-open="${esc(s.path)}" title="${esc(s.name)}" ${stagger(i)}><img src="${esc(fileUrl(s.path))}" alt="${esc(s.name)}" loading="lazy" decoding="async"></button>`).join("")}</div>`
    : `<div class="empty"><h4>${t("Nessuno screenshot")}</h4><p>${t("Premi F2 in gioco per scattarne uno: lo trovi qui.")}</p></div>`;
}

function logsTab(p) {
  return `<div class="toolbar"><span class="meta">${t("Output dell'ultima sessione di gioco")}</span><span class="spacer"></span><button class="btn sm" data-act="copy-log">${icon("copy")}${t("Copia")}</button></div>
  <div class="log" id="log">${(S.logs[p.id] || []).length ? "" : `<span style="color:var(--ink-3)">${t("Avvia il gioco per vedere qui il suo output.")}</span>`}</div>`;
}
const logLine = (l) => { const c = /\bERROR\b|Exception/.test(l) ? "e" : /\bWARN\b/.test(l) ? "w" : ""; return c ? `<span class="${c}">${esc(l)}</span>\n` : `${esc(l)}\n`; };
async function loadLogs(p) {
  S.logs[p.id] = await invoke("logs", { id: p.id });
  const el = $("#log");
  if (el && S.logs[p.id].length) { el.innerHTML = S.logs[p.id].map(logLine).join(""); el.scrollTop = el.scrollHeight; }
}

/* ---------------- profile form ---------------- */

function editTab(p) {
  return `${profileForm(p)}
  <section class="section danger-zone">
    <div class="row"><h3>${t("Elimina profilo")}</h3>${tip(tp("Cancella mondi, mod e configurazione di questo profilo. Non si può annullare."))}</div>
    <button class="btn danger" data-act="delete" style="margin-top:12px">${icon("trash")}${t("Elimina profilo")}</button>
  </section>`;
}

function newPage() {
  const first = !S.profiles.length;
  return `<div class="page-head head-row"><h1>${t(first ? "Il tuo primo profilo" : "Nuovo profilo")}</h1>
    <button class="btn" id="import-open" aria-expanded="${!!S.importer}">${icon("down")}${t("Importa da un altro launcher")}</button></div>
  <div class="page">${S.importer ? importPanel() : profileForm(null)}</div>`;
}

/* ----- import from Modrinth App, CurseForge, Prism Launcher (optional) ----- */

const SOURCES = { modrinth: "Modrinth App", curseforge: "CurseForge", prism: "Prism Launcher" };

function importPanel() {
  const im = S.importer;
  if (!im.found) return `<section class="section"><div class="list">${'<div class="skeleton skel-row"></div>'.repeat(4)}</div></section>`;
  if (!im.found.length) {
    return `<div class="empty"><h4>${t("Nessun profilo da importare")}</h4><p>${t("Non ho trovato istanze di Modrinth App, CurseForge o Prism Launcher su questo computer.")}</p><button class="btn" id="import-close">${t("Torna al nuovo profilo")}</button></div>`;
  }
  const n = im.selected.size;
  const groups = Object.keys(SOURCES).map((src) => {
    const rows = im.found.map((f, i) => ({ f, i })).filter(({ f }) => f.source === src);
    if (!rows.length) return "";
    return `<section class="section"><h3 class="section-title">${SOURCES[src]}</h3><div class="list">${rows.map(({ f, i }) => {
      const st = im.status[i];
      const facts = [f.mc, f.loader === "vanilla" ? "Vanilla" : f.loader === "fabric" ? "Fabric" : f.loader, tp(f.worlds === 1 ? "1 mondo" : "{n} mondi", { n: f.worlds }), tp("{n} mod", { n: f.mods })].filter(Boolean).join(" · ");
      const right = st === "busy" ? `<span class="done">${icon("down", "spin")}${t("Copio…")}</span>`
        : st === "done" ? `<span class="done">${icon("check", "accent")}${t("Importato")}</span>`
        : st ? `<span class="meta" title="${esc(st)}">${t("Errore")}</span>`
        : f.supported ? "" : `<span class="meta">${t("Non supportato")}</span>`;
      return `<label class="item import-row ${f.supported ? "" : "off"}" ${stagger(i)}>
        <input type="checkbox" class="check" data-pick="${i}" ${im.selected.has(i) ? "checked" : ""} ${f.supported && !im.running ? "" : "disabled"} aria-label="${esc(f.name)}">
        <div style="min-width:0"><div class="item-title">${esc(f.name)}</div><div class="meta">${esc(facts)}</div></div>
        <div class="actions">${right}${f.supported ? "" : tip(tp("Agate avvia solo profili Vanilla e Fabric."))}</div>
      </label>`;
    }).join("")}</div></section>`;
  }).join("");
  return `<p class="lead">${t("Scegli i profili da copiare in Agate: mondi, mod, texture pack, shader e impostazioni. Gli originali non vengono toccati.")}</p>
    ${groups}
    <div class="row import-foot">
      <button class="btn primary" id="import-go" ${n && !im.running ? "" : "disabled"}>${im.running ? icon("down", "spin") : icon("down")}${t("Importa {n}", { n: n || "" })}</button>
      <button class="btn ghost" id="import-close" ${im.running ? "disabled" : ""}>${t("Annulla")}</button>
    </div>`;
}

function paintImporter() {
  const page = $("#main .page");
  if (page && S.importer) page.innerHTML = importPanel();
  $("#import-open")?.setAttribute("aria-expanded", !!S.importer);
}

async function openImporter() {
  S.importer = { found: null, selected: new Set(), status: {}, running: false };
  fadeSwap($("#main .page"), paintImporter);
  const found = await invoke("import_scan").catch(() => []);
  if (!S.importer) return;
  S.importer.found = found;
  paintImporter();
}

async function runImport() {
  const im = S.importer;
  im.running = true;
  const picks = [...im.selected];
  let last = null, ok = 0;
  for (const [k, i] of picks.entries()) {
    im.status[i] = "busy";
    paintImporter();
    try {
      const f = im.found[i];
      const p = await invoke("import_instance", { path: f.path, color: COLORS[(S.profiles.length + k) % COLORS.length], icon: Art.names[(S.profiles.length + k) % Art.names.length] });
      im.status[i] = "done";
      last = p; ok++;
    } catch (e) { im.status[i] = String(e); toast(String(e), "bad"); }
    paintImporter();
  }
  im.running = false;
  await refresh();
  if (ok) toast(tp(ok === 1 ? "1 profilo importato" : "{n} profili importati", { n: ok }));
  S.importer = null;
  if (last) go({ kind: "profile", id: last.id, tab: defaultTab(last) });
  else render();
}

document.addEventListener("click", (e) => {
  if (e.target.closest("#import-open")) return S.importer ? (S.importer = null, fadeSwap($("#main .page"), () => render())) : openImporter();
  if (e.target.closest("#import-close")) { S.importer = null; return fadeSwap($("#main .page"), () => render()); }
  if (e.target.closest("#import-go")) return runImport();
});
document.addEventListener("change", (e) => {
  const pick = e.target.closest("[data-pick]");
  if (!pick || !S.importer) return;
  const i = +pick.dataset.pick;
  if (pick.checked) S.importer.selected.add(i); else S.importer.selected.delete(i);
  const btn = $("#import-go");
  if (btn) { btn.disabled = !S.importer.selected.size; btn.innerHTML = `${icon("down")}${t("Importa {n}", { n: S.importer.selected.size || "" })}`; }
});

function profileForm(p) {
  const v = p || { name: "", mc: "", loader: "fabric", icon: Art.names[S.profiles.length % Art.names.length], color: COLORS[S.profiles.length % COLORS.length], memory_mb: S.settings.default_memory_mb || 4096, jvm_args: "", java_path: "", sync_settings: true };
  const maxMem = Math.max(2048, Math.floor((S.sysMem * 0.75) / 512) * 512);
  return `<form id="pform" class="section" data-color="${v.color}" autocomplete="off">
    <div class="field"><span>${t("Caricatore")}</span>
      <div class="segmented" role="group">${Object.entries(LOADERS).map(([k, l]) => `<button type="button" data-loader="${k}" aria-pressed="${v.loader === k}">${l}</button>`).join("")}</div>
      <small id="loader-hint"></small>
    </div>
    <div class="grid2" style="margin-top:20px">
      <label class="field"><span>${t("Versione di Minecraft")}</span><select class="input" name="mc" id="mc-select"><option value="">${t("Caricamento…")}</option></select></label>
      <label class="field" id="of-field" hidden><span>${t("Build di OptiFine")}</span><select class="input" id="of-select"></select></label>
      <label class="field"><span>${t("Nome")}</span><input class="input" name="name" maxlength="48" value="${esc(v.name)}" placeholder="${esc(tp("Es. Sopravvivenza con gli amici"))}"></label>
    </div>
    <div class="field" style="margin-top:20px"><span>${t("Icona")}</span>
      <div class="icon-grid" role="group" aria-label="${esc(tp("Icona"))}">${Art.names.map((n) => `<button type="button" class="icon-pick" data-icon="${n}" aria-pressed="${(v.icon || "block") === n}" title="${esc(tp(ICON_NAMES[n]))}" aria-label="${esc(tp(ICON_NAMES[n]))}">${Art.icon(n)}</button>`).join("")}</div>
    </div>
    <div class="grid2" style="margin-top:20px">
      <div class="field"><span>${t("Colore")}</span><div class="swatches" role="group" aria-label="${esc(tp("Colore"))}">${COLORS.map((c) => `<button type="button" class="swatch" data-color="${c}" data-swatch="${c}" aria-pressed="${v.color === c}" aria-label="${c}">${Art.icon(v.icon || "block")}</button>`).join("")}</div></div>
      <div class="field"><span class="label-row">${t("Memoria")}${tip(tp("4 GB bastano per vanilla e poche mod; 6 GB con gli shader. Il computer ha {mem}.", { mem: fmtMem(S.sysMem) }))}</span>
        <div class="range"><input type="range" name="memory" min="1024" max="${maxMem}" step="512" value="${Math.min(v.memory_mb, maxMem)}" aria-label="${esc(tp("Memoria"))}"><output id="mem-out">${fmtMem(Math.min(v.memory_mb, maxMem))}</output></div></div>
    </div>
    <div class="field" style="margin-top:22px"><div class="row"><label class="switch"><input type="checkbox" name="sync" ${v.sync_settings ? "checked" : ""}><b>${t("Condividi comandi e impostazioni")}</b></label>${tip(tp("Tasti, opzioni video e audio e lista server restano uguali in tutti i profili."))}</div></div>
    <details class="disclosure" style="margin-top:18px"><summary>${icon("chevron")}${t("Avanzate")}</summary>
      <div class="grid2" style="margin-top:14px">
        <label class="field"><span class="label-row">${t("Java personalizzato")}${tip(tp("Percorso di javaw.exe. Vuoto: il launcher scarica la versione giusta."))}</span><input class="input" name="java" value="${esc(v.java_path || "")}" placeholder="${esc(tp("Automatico"))}" spellcheck="false"></label>
        <label class="field"><span>${t("Argomenti JVM")}</span><input class="input" name="jvm" value="${esc(v.jvm_args || "")}" placeholder="-XX:+UseG1GC" spellcheck="false"></label>
      </div>
    </details>
    <div class="row" style="margin-top:26px">
      <button class="btn primary" type="submit" id="pform-submit">${p ? `${icon("check")}${t("Salva modifiche")}` : `${icon("plus")}${t("Crea profilo")}`}</button>
      ${!p && S.profiles.length ? `<button class="btn ghost" type="button" data-cancel>${t("Annulla")}</button>` : ""}
      <span class="hint" id="pform-msg" role="alert"></span>
    </div>
  </form>`;
}

let catalogLoad = null;
/** Version lists from Mojang, Fabric and OptiFine, loaded once (retried after a failure). */
function ensureCatalog() {
  catalogLoad ||= Promise.all([
    S.versions ? null : invoke("versions").then((v) => (S.versions = v)),
    S.fabric ? null : invoke("fabric_versions").then((v) => (S.fabric = new Set(v))).catch(() => (S.fabric = new Set())),
    S.optifine ? null : invoke("optifine_builds").then((v) => (S.optifine = v)).catch((e) => { S.optifineError = String(e); S.optifine = {}; }),
  ]).catch((e) => { catalogLoad = null; throw e; });
  return catalogLoad;
}

async function bindProfileForm(p) {
  const form = $("#pform");
  if (!form) return;
  let loader = p ? p.loader : "fabric";
  let color = form.dataset.color;
  let iconName = form.querySelector('.icon-pick[aria-pressed="true"]')?.dataset.icon || "block";
  form.querySelectorAll("[data-icon]").forEach((b) => b.addEventListener("click", () => {
    iconName = b.dataset.icon;
    form.querySelectorAll("[data-icon]").forEach((x) => x.setAttribute("aria-pressed", x === b));
    form.querySelectorAll(".swatch").forEach((sw) => (sw.innerHTML = Art.icon(iconName)));
  }));
  const mcSel = $("#mc-select"), ofSel = $("#of-select"), ofField = $("#of-field"), hint = $("#loader-hint");
  const nameInput = form.elements.name;
  let autoName = !p;
  nameInput.addEventListener("input", () => (autoName = nameInput.value === ""));
  form.querySelectorAll("[data-swatch]").forEach((b) => b.addEventListener("click", () => {
    color = b.dataset.swatch;
    form.dataset.color = color;
    form.querySelectorAll("[data-swatch]").forEach((x) => x.setAttribute("aria-pressed", x === b));
  }));
  form.elements.memory.addEventListener("input", (e) => ($("#mem-out").textContent = fmtMem(+e.target.value)));
  form.querySelector("[data-cancel]")?.addEventListener("click", () => go({ kind: "profile", id: S.profiles[0].id, tab: defaultTab(S.profiles[0]) }));

  try { await ensureCatalog(); }
  catch (e) {
    if (!form.isConnected) return;
    mcSel.innerHTML = `<option value="">${t("Elenco non disponibile")}</option>`;
    hint.innerHTML = `${t("Non riesco a scaricare l'elenco delle versioni ({err}).", { err: e })} <button type="button" class="btn sm" id="retry-cat">${t("Riprova")}</button>`;
    $("#retry-cat").onclick = () => bindProfileForm(p);
    return;
  }
  if (!form.isConnected) return;

  /** Versions this loader supports, newest first. */
  const versionsFor = (l) => S.versions.filter((v) => {
    if (p && v.id === p.mc) return true;
    if (v.kind !== "release" && !S.settings.show_snapshots) return false;
    return l === "fabric" ? S.fabric.has(v.id) : l === "optifine" ? !!S.optifine[v.id] : true;
  });

  function syncVersions() {
    form.querySelectorAll("[data-loader]").forEach((b) => b.setAttribute("aria-pressed", b.dataset.loader === loader));
    const list = versionsFor(loader);
    // First fill: the profile's own version (the select still holds the loading placeholder).
    const prev = mcSel.dataset.filled ? mcSel.value : p ? p.mc : "";
    mcSel.dataset.filled = "1";
    mcSel.innerHTML = list.map((v) => {
      const pre = loader === "optifine" && S.optifine[v.id]?.every((b) => b.preview);
      return `<option value="${esc(v.id)}">${esc(v.id)}${v.kind === "snapshot" ? " (snapshot)" : ""}${pre ? ` · ${t("solo anteprima")}` : ""}</option>`;
    }).join("") || `<option value="">${t("Nessuna versione disponibile")}</option>`;
    mcSel.value = list.some((v) => v.id === prev) ? prev : list[0]?.id || "";
    const latest = S.versions.find((v) => v.kind === "release")?.id;
    hint.innerHTML = loader === "fabric" ? t("Mod, shader e texture pack da Modrinth.")
      : loader === "optifine" ? (S.optifineError ? t("optifine.net non risponde ({err}).", { err: S.optifineError })
        : latest && !S.optifine[latest] ? t("Prestazioni e shader. Disponibile fino alla {v}: per la {latest} non è ancora uscito.", { v: list[0]?.id || "—", latest })
        : t("Prestazioni e shader. Disponibile fino alla {v}.", { v: list[0]?.id || "—" }))
      : t("Minecraft così com'è, con i texture pack.");
    syncBuild();
  }
  function syncBuild() {
    const builds = loader === "optifine" ? S.optifine[mcSel.value] || [] : [];
    ofField.hidden = loader !== "optifine";
    ofSel.innerHTML = builds.map((b) => `<option value="${esc(b.file)}">OptiFine ${esc(b.label)}${b.preview ? ` ${t("(anteprima)")}` : ""}</option>`).join("");
    if (p && p.loader === "optifine" && p.mc === mcSel.value && p.loader_version) ofSel.value = p.loader_version;
    if (autoName && mcSel.value) nameInput.value = `${loader === "vanilla" ? "Minecraft" : LOADERS[loader]} ${mcSel.value}`;
  }
  form.querySelectorAll("[data-loader]").forEach((b) => b.addEventListener("click", () => { loader = b.dataset.loader; syncVersions(); }));
  mcSel.addEventListener("change", syncBuild);
  syncVersions();

  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    const name = nameInput.value.trim();
    if (!name) { nameInput.setAttribute("aria-invalid", "true"); $("#pform-msg").textContent = tp("Dai un nome al profilo."); return nameInput.focus(); }
    if (!mcSel.value) { $("#pform-msg").textContent = tp("Scegli una versione."); return; }
    const mc = mcSel.value;
    const keep = p && p.loader === loader && p.mc === mc;
    const data = {
      id: p ? p.id : "", name, mc, loader, color, icon: iconName,
      loader_version: loader === "optifine" ? ofSel.value || null : keep ? p.loader_version : null,
      memory_mb: +form.elements.memory.value,
      java_path: form.elements.java.value.trim() || null,
      jvm_args: form.elements.jvm.value.trim(),
      sync_settings: form.elements.sync.checked,
      created: p ? p.created : 0, last_played: p ? p.last_played : null, mods: [],
    };
    $("#pform-submit").disabled = true;
    try {
      const saved = await call("save_profile", { profile: data });
      await refresh();
      delete S.content[saved.id];
      if (p && (p.mc !== mc || p.loader !== loader) && (p.mods || []).length) toast(tp("Versione cambiata: controlla che mod e pack siano compatibili"));
      else toast(tp(p ? "Profilo salvato" : "Profilo creato"));
      go({ kind: "profile", id: saved.id, tab: p ? "edit" : defaultTab(saved) });
    } catch { $("#pform-submit").disabled = false; }
  });
}

/* ---------------- settings ---------------- */

/** One preference: label (+ optional "?" tip) on the left, control on the right. */
const pref = (label, control, help) => `<div class="pref"><span class="pref-label">${label}${help ? tip(help) : ""}</span><div class="pref-control">${control}</div></div>`;

function settingsPage() {
  const s = S.settings;
  const a = activeAccount();
  const toggle = (id, on, label) => `<label class="switch"><input type="checkbox" id="${id}" ${on ? "checked" : ""} aria-label="${esc(label)}"></label>`;
  return `<div class="page-head"><h1>${t("Impostazioni")}</h1></div>
  <div class="page settings-grid">
    <aside class="player-card" data-color="grass">
      <div class="stand" id="big-figure">${a ? "" : `<span class="hint">${t("Accedi per vedere il tuo giocatore")}</span>`}</div>
      ${a ? `<div class="player-name"><h3>${esc(a.name)}</h3><span class="meta">${t(a.kind === "msa" ? "Account Microsoft" : "Account offline")}</span></div>` : ""}
      ${a && a.kind === "msa" ? `
        <div class="row wrap center">
          <div class="segmented" role="group" aria-label="${esc(tp("Modello"))}"><button type="button" data-model="classic" aria-pressed="${!S.skin?.slim}">${t("Classico")}</button><button type="button" data-model="slim" aria-pressed="${!!S.skin?.slim}">${t("Slim")}</button></div>
          <label class="btn">${icon("upload")}${t("Cambia skin")}<input type="file" accept="image/png" id="skin-file" hidden></label>
          ${tip(tp("PNG 64×64. Trascina il giocatore per girarlo."))}
        </div>
        <div class="capes-block">
          <div class="row"><h4>${t("Mantello Microsoft")}</h4>${tip(tp("I mantelli del tuo account Microsoft. Quello scelto si vede in gioco su tutti i server."))}</div>
          <div class="capes" id="capes">${'<span class="skeleton cape-skel"></span>'.repeat(3)}</div>
        </div>` : ""}
      ${a ? `<div class="capes-block" id="custom-cape"></div>` : ""}
    </aside>
    <div class="settings-col">
      <section class="section">
        <div class="row"><h3>${t("Account")}</h3>${tip(tp("Accedi con l'account Microsoft che possiede Minecraft per giocare online. Un nome offline basta per il giocatore singolo."))}</div>
        ${S.accounts.length ? `<div class="list" style="margin:12px 0 14px">${S.accounts.map((x) => `
          <div class="item" style="grid-template-columns:32px 1fr auto"><span class="avatar" data-head="${x.uuid}">${esc(x.name[0].toUpperCase())}</span>
            <div style="min-width:0"><div class="item-title">${esc(x.name)}</div><div class="meta">${t(x.kind === "msa" ? "Microsoft" : "Offline · solo giocatore singolo")}</div></div>
            <div class="actions">${x.active ? `<span class="done">${icon("check")}${t("In uso")}</span>` : `<button class="btn sm" data-use="${x.uuid}">${t("Usa")}</button>`}
            <button class="icon-btn" data-forget="${x.uuid}" title="${esc(tp("Rimuovi account"))}" aria-label="${esc(tp("Rimuovi {name}", { name: x.name }))}">${icon("trash")}</button></div>
          </div>`).join("")}</div>` : `<div style="height:12px"></div>`}
        <div id="login-area">${loginArea()}</div>
      </section>
      <section class="section">
        <h3>${t("Preferenze")}</h3>
        <div class="prefs">
          ${pref(t("Lingua"), `<div class="segmented" role="group" aria-label="${esc(tp("Lingua"))}"><button type="button" data-lang="it" aria-pressed="${LANG === "it"}">Italiano</button><button type="button" data-lang="en" aria-pressed="${LANG === "en"}">English</button></div>`)}
          ${pref(t("Animazioni"), `<div class="segmented" role="group" aria-label="${esc(tp("Animazioni"))}">${[["auto", "Auto"], ["on", "Sempre"], ["off", "Ridotte"]].map(([k, l]) => `<button type="button" data-motion="${k}" aria-pressed="${(s.motion || "auto") === k}">${t(l)}</button>`).join("")}</div>`, tp(systemReduce.matches ? "Auto segue Windows, dove gli effetti di animazione ora sono disattivati." : "Auto segue Windows, dove gli effetti di animazione ora sono attivi."))}
          ${pref(t("Memoria per i nuovi profili"), `<select class="input compact" id="def-mem" aria-label="${esc(tp("Memoria per i nuovi profili"))}">${[2048, 3072, 4096, 6144, 8192].map((m) => `<option value="${m}" ${s.default_memory_mb === m ? "selected" : ""}>${fmtMem(m)}</option>`).join("")}</select>`)}
          ${pref(t("Nascondi il launcher mentre giochi"), toggle("hide", s.close_on_launch, tp("Nascondi il launcher mentre giochi")), tp("Riappare quando chiudi il gioco."))}
          ${pref(t("Mostra snapshot"), toggle("snapshots", s.show_snapshots, tp("Mostra snapshot")), tp("Versioni di prova settimanali di Mojang."))}
          <div id="update-row">${updateRow()}</div>
        </div>
      </section>
      <details class="disclosure section"><summary>${icon("chevron")}${t("Scorciatoie da tastiera")}</summary>
        <div class="prefs">${SHORTCUTS.map(([k, l]) => pref(t(l), `<span class="keys">${k.split("  ").map((x) => `<kbd>${esc(keyName(x))}</kbd>`).join("")}</span>`)).join("")}</div>
      </details>
      <details class="disclosure section"><summary>${icon("chevron")}${t("Avanzate")}</summary>
        <div class="prefs">
          ${pref(t("ID client Azure"), `<div class="row"><input class="input" id="azure-id" value="${esc(s.azure_client_id || "")}" placeholder="00000000-0000-0000-0000-000000000000" spellcheck="false"><button class="btn" id="save-azure">${t("Salva")}</button></div>`, tp("Se hai un'app Azure approvata da Mojang, inserisci il suo ID client. Vuoto: si usa l'ID pubblico del launcher di Minecraft."))}
          ${pref(t("Cartella dati"), `<button class="btn" id="open-data">${icon("folder")}${t("Apri")}</button>`, tp("Versioni, Java, profili e impostazioni condivise (cartella shared) sono in {path}.", { path: S.dataDir }))}
        </div>
      </details>
    </div>
  </div>`;
}

function loginArea() {
  const l = S.login;
  if (l && l.code) {
    return `<div class="code-card" data-color="grass">
      <ol class="steps"><li>${t("Apri <b>{uri}</b> nel browser.", { uri: l.code.verification_uri.replace(/^https:\/\//, "") })}</li><li>${t("Inserisci questo codice e accedi con il tuo account Microsoft.")}</li></ol>
      <div class="code">${esc(l.code.user_code)}</div>
      <div class="row wrap"><button class="btn primary" id="open-link">${icon("out")}${t("Apri la pagina")}</button><button class="btn" id="copy-code">${icon("copy")}${t("Copia codice")}</button><span class="spacer"></span><button class="btn ghost" id="cancel-login">${t("Annulla")}</button></div>
      <div class="meta">${t("In attesa dell'accesso…")}</div>
    </div>`;
  }
  return `<div class="row wrap" data-color="grass">
    <button class="btn primary" id="ms-login" ${l && l.starting ? "disabled" : ""}>${l && l.starting ? icon("down", "spin") : icon("user")}${t("Accedi con Microsoft")}</button>
    <form class="row" id="offline-form"><input class="input" name="name" placeholder="${esc(tp("Nome offline"))}" aria-label="${esc(tp("Nome offline"))}" maxlength="16" style="width:170px" spellcheck="false"><button class="btn" type="submit">${t("Aggiungi")}</button></form>
  </div>`;
}

async function mountSettingsArt() {
  const a = activeAccount();
  if (!a || !S.skin) return;
  const fig = await figureFor(9).catch(() => null);
  if (fig && S.view.kind === "settings") $("#big-figure")?.replaceChildren(fig);
  const head = await Skin.head(fileUrl(S.skin.path), 32).catch(() => null);
  const slot = document.querySelector(`[data-head="${a.uuid}"]`);
  if (head && slot) slot.replaceChildren(head);
  if (a.kind === "msa") paintCapes();
  paintCustomCape();
}

/* ----- custom cape (shown only to you, by the Agate Cape mod on Fabric 26.x) ----- */

function customCapeHtml() {
  const e = S.capeEdit;
  const head = `<div class="row"><h4>${t("Mantello personalizzato")}</h4>${tip(tp("Un mantello tuo, che vedi solo tu in gioco. Funziona nei profili Fabric 26.x: il launcher aggiunge da solo la mod Agate Cape."))}</div>`;
  const picker = (label, cls = "btn sm") => `<label class="${cls}">${icon("upload")}${t(label)}<input type="file" accept="image/png,image/jpeg" id="cape-file" hidden></label>`;
  if (e) {
    return `${head}
      <div class="custom-cape">
        <span class="cape-art big" id="custom-art"></span>
        <div class="custom-actions">
          ${e.template ? `<span class="meta">${t("Già un modello di mantello: usato così com'è.")}</span>` : `
          <div class="segmented" role="group" aria-label="${esc(tp("Immagine"))}"><button type="button" data-fit="cover" aria-pressed="${e.fit === "cover"}">${t("Riempi")}</button><button type="button" data-fit="contain" aria-pressed="${e.fit === "contain"}">${t("Adatta")}</button></div>
          <label class="color-pick"><input type="color" id="cape-color" value="${esc(e.color)}"><span>${t("Bordi")}</span></label>`}
          ${picker("Cambia immagine")}
        </div>
      </div>
      <div class="row"><button class="btn primary sm" id="cape-save">${icon("check")}${t("Salva")}</button><button class="btn ghost sm" id="cape-cancel">${t("Annulla")}</button></div>`;
  }
  if (!S.custom) return `${head}${picker("Crea da un'immagine", "btn")}`;
  return `${head}
    <div class="custom-cape">
      <span class="cape-art big" id="custom-art"></span>
      <div class="custom-actions">
        <label class="switch"><input type="checkbox" id="custom-on" ${S.settings.custom_cape ? "checked" : ""}><b>${t("Usa in gioco")}</b></label>
        <div class="row"><button class="btn sm" id="cape-edit">${t("Modifica")}</button>
        <button class="icon-btn" id="cape-remove" title="${esc(tp("Rimuovi"))}" aria-label="${esc(tp("Rimuovi il mantello personalizzato"))}">${icon("trash")}</button></div>
      </div>
    </div>`;
}

/** The finished texture for the current edit. */
const editedCape = () => CapeMaker.make(S.capeEdit.img, { fit: S.capeEdit.fit, color: S.capeEdit.color });

async function paintCustomCape() {
  const box = $("#custom-cape");
  if (!box) return;
  box.innerHTML = customCapeHtml();
  const art = $("#custom-art");
  const src = S.capeEdit ? editedCape().toDataURL("image/png") : S.custom ? `${fileUrl(S.custom.path)}?v=${S.customV}` : null;
  if (art && src) Skin.capeFront(src, 5).then((c) => art.replaceChildren(c)).catch(() => {});
  if (S.capeEdit) previewFigure(src);
  bindCustomCape();
}

/** While editing, the big player turns around to show the cape being made. */
let previewTimer;
function previewFigure(capeSrc) {
  clearTimeout(previewTimer);
  previewTimer = setTimeout(async () => {
    if (!S.skin) return;
    const fig = await Skin.figure(fileUrl(S.skin.path), { slim: S.skin.slim, s: 9, cape: capeSrc, yaw: 165 }).catch(() => null);
    if (fig && S.view.kind === "settings") $("#big-figure")?.replaceChildren(fig);
  }, 120);
}

/** Reads a picked file into an image, plus a PNG copy (max 2048 px) to keep as the source. */
async function readPicture(file) {
  const url = await new Promise((ok, fail) => { const r = new FileReader(); r.onload = () => ok(r.result); r.onerror = fail; r.readAsDataURL(file); });
  const img = await loadImage(url);
  const scale = Math.min(1, 2048 / Math.max(img.naturalWidth, img.naturalHeight));
  const c = document.createElement("canvas");
  c.width = Math.round(img.naturalWidth * scale);
  c.height = Math.round(img.naturalHeight * scale);
  c.getContext("2d").drawImage(img, 0, 0, c.width, c.height);
  return { img: scale < 1 ? await loadImage(c.toDataURL("image/png")) : img, source: await CapeMaker.toPng(c) };
}

function bindCustomCape() {
  $("#cape-file")?.addEventListener("change", async (ev) => {
    const file = ev.target.files[0];
    ev.target.value = "";
    if (!file) return;
    try {
      const { img, source } = await readPicture(file);
      const prev = S.capeEdit;
      S.capeEdit = { img, source, fit: prev?.fit || "cover", color: CapeMaker.average(img), template: CapeMaker.isTemplate(img) };
      paintCustomCape();
    } catch { toast(tp("Non riesco a leggere questa immagine"), "bad"); }
  });
  document.querySelectorAll("[data-fit]").forEach((b) => b.addEventListener("click", () => { S.capeEdit.fit = b.dataset.fit; paintCustomCape(); }));
  $("#cape-color")?.addEventListener("input", (ev) => {
    S.capeEdit.color = ev.target.value;
    const src = editedCape().toDataURL("image/png");
    Skin.capeFront(src, 5).then((c) => $("#custom-art")?.replaceChildren(c));
    previewFigure(src);
  });
  $("#cape-cancel")?.addEventListener("click", () => { S.capeEdit = null; render(); });
  $("#cape-save")?.addEventListener("click", async (ev) => {
    ev.target.disabled = true;
    try {
      const png = await CapeMaker.toPng(editedCape());
      const first = !S.custom;
      await invoke("save_custom_cape", { png, source: S.capeEdit.source, options: JSON.stringify({ fit: S.capeEdit.fit, color: S.capeEdit.color }) });
      if (first) await saveSettings({ custom_cape: true });
      S.capeEdit = null;
      S.custom = await invoke("custom_cape");
      S.customV = Date.now();
      toast(tp("Mantello salvato: lo vedrai al prossimo avvio di un profilo Fabric 26.x"));
      render();
    } catch (err) { toast(String(err), "bad"); ev.target.disabled = false; }
  });
  $("#cape-edit")?.addEventListener("click", async () => {
    try {
      const o = S.custom.options ? JSON.parse(S.custom.options) : {};
      const url = `${fileUrl(S.custom.source || S.custom.path)}?v=${S.customV}`;
      const img = await loadImage(url);
      const c = document.createElement("canvas");
      c.width = img.naturalWidth; c.height = img.naturalHeight;
      c.getContext("2d").drawImage(img, 0, 0);
      S.capeEdit = { img, source: await CapeMaker.toPng(c), fit: o.fit || "cover", color: o.color || CapeMaker.average(img), template: CapeMaker.isTemplate(img) };
      paintCustomCape();
    } catch { toast(tp("Non riesco a leggere questa immagine"), "bad"); }
  });
  $("#cape-remove")?.addEventListener("click", async () => {
    if (!(await confirmDialog(tp("Eliminare il mantello personalizzato?"), tp("Sparirà dal launcher e dal gioco. I mantelli Microsoft non cambiano."), tp("Elimina")))) return;
    await call("delete_custom_cape");
    await saveSettings({ custom_cape: false });
    S.custom = null;
    render();
  });
  $("#custom-on")?.addEventListener("change", async (ev) => {
    await saveSettings({ custom_cape: ev.target.checked });
    S.customV = Date.now();
    mountSettingsArt();
  });
}

async function paintCapes() {
  if (!S.capes) S.capes = await invoke("capes").catch(() => []);
  const box = $("#capes");
  if (!box) return;
  if (!S.capes.length) { box.outerHTML = `<p class="meta">${t("Questo account non ha mantelli.")}</p>`; return; }
  const none = !S.capes.some((c) => c.active);
  box.innerHTML = `<button class="cape" data-cape="" aria-pressed="${none}" title="${esc(tp("Nessuno"))}"><span class="cape-none">${icon("x")}</span><span>${t("Nessuno")}</span></button>` +
    S.capes.map((c) => `<button class="cape" data-cape="${esc(c.id)}" aria-pressed="${c.active}" title="${esc(c.alias)}"><span class="cape-art" data-art="${esc(c.path || "")}"></span><span>${esc(c.alias)}</span></button>`).join("");
  box.querySelectorAll("[data-art]").forEach(async (el) => {
    if (!el.dataset.art) return;
    const c = await Skin.capeFront(fileUrl(el.dataset.art), 4).catch(() => null);
    if (c) el.replaceChildren(c);
  });
}

function bindSettings() {
  mountSettingsArt();
  const m = $("#main");
  let slim = !!S.skin?.slim;
  m.onclick = async (e) => {
    const b = e.target.closest("button");
    if (!b) return;
    if (b.dataset.motion) {
      await saveSettings({ motion: b.dataset.motion });
      applyMotion();
      document.querySelectorAll("[data-motion]").forEach((x) => x.setAttribute("aria-pressed", x === b));
      return;
    }
    if (b.dataset.lang) { await saveSettings({ language: b.dataset.lang }); setLang(b.dataset.lang); return render(); }
    if (b.dataset.model) { slim = b.dataset.model === "slim"; document.querySelectorAll("[data-model]").forEach((x) => x.setAttribute("aria-pressed", x === b)); return; }
    if ("cape" in b.dataset) {
      if (b.getAttribute("aria-pressed") === "true") return;
      document.querySelectorAll(".cape").forEach((x) => x.classList.toggle("pending", x === b));
      try {
        await invoke("set_cape", { id: b.dataset.cape || null });
        toast(tp(b.dataset.cape ? "Mantello aggiornato" : "Mantello nascosto"));
        await loadSkin();
        return render();
      } catch (err) { toast(String(err), "bad"); b.classList.remove("pending"); return; }
    }
    if (b.dataset.use) { await call("set_account", { uuid: b.dataset.use }); await refresh(); await loadSkin(); return render(); }
    if (b.dataset.forget) {
      const a = S.accounts.find((x) => x.uuid === b.dataset.forget);
      if (!(await confirmDialog(tp("Rimuovere {name}?", { name: a.name }), tp("L'account verrà disconnesso da questo launcher. Potrai accedere di nuovo quando vuoi."), tp("Rimuovi")))) return;
      await call("remove_account", { uuid: a.uuid }); await refresh(); await loadSkin(); return render();
    }
    if (b.id === "ms-login") return startLogin();
    if (b.id === "open-link") return call("open_url", { url: S.login.code.verification_uri });
    if (b.id === "copy-code") { navigator.clipboard.writeText(S.login.code.user_code); return toast(tp("Codice copiato")); }
    if (b.id === "cancel-login") { S.login = null; return paintLogin(); }
    if (b.id === "open-data") return call("open_folder", { id: null });
    if (b.id === "save-azure") { await saveSettings({ azure_client_id: $("#azure-id").value.trim() || null }); return toast(tp("ID client salvato")); }
  };
  $("#skin-file")?.addEventListener("change", async (e) => {
    const file = e.target.files[0];
    e.target.value = "";
    if (!file) return;
    const png = Array.from(new Uint8Array(await file.arrayBuffer()));
    try {
      await invoke("upload_skin", { slim, png });
      await loadSkin();
      render();
      toast(tp("Skin aggiornata: la vedrai anche in gioco"));
    } catch (err) { toast(String(err), "bad"); }
  });
  $("#def-mem").addEventListener("change", (e) => saveSettings({ default_memory_mb: +e.target.value }));
  $("#snapshots").addEventListener("change", (e) => saveSettings({ show_snapshots: e.target.checked }));
  $("#hide").addEventListener("change", (e) => saveSettings({ close_on_launch: e.target.checked }));
  bindOffline();
}

function bindOffline() {
  $("#offline-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const name = e.target.elements.name.value.trim();
    if (!/^[A-Za-z0-9_]{3,16}$/.test(name)) return toast(tp("Il nome offline usa 3–16 lettere, numeri o _"), "bad");
    await call("add_offline", { name });
    await refresh(); await loadSkin(); render(); toast(tp("Account {name} aggiunto", { name }));
  });
}

function paintLogin() {
  const el = $("#login-area");
  if (el) { el.innerHTML = loginArea(); bindOffline(); }
}

async function saveSettings(patch) {
  const next = { ...S.settings, ...patch };
  await call("save_settings", { settings: next });
  S.settings = next;
}

async function startLogin() {
  S.login = { starting: true };
  paintLogin();
  let code;
  try { code = await invoke("login_start"); }
  catch (e) { S.login = null; paintLogin(); return toast(String(e), "bad"); }
  const session = { code };
  S.login = session;
  paintLogin();
  call("open_url", { url: code.verification_uri }).catch(() => {});
  const deadline = Date.now() + code.expires_in * 1000;
  while (S.login === session && Date.now() < deadline) {
    await new Promise((r) => setTimeout(r, Math.max(2, code.interval) * 1000));
    if (S.login !== session) return;
    try {
      const acc = await invoke("login_poll", { deviceCode: code.device_code });
      if (acc) {
        S.login = null;
        await refresh(); await loadSkin(); render();
        return toast(tp("Accesso effettuato come {name}", { name: acc.name }));
      }
    } catch (e) { S.login = null; paintLogin(); return toast(String(e), "bad"); }
  }
  if (S.login === session) { S.login = null; paintLogin(); toast(tp("Il codice è scaduto. Riprova."), "bad"); }
}

/* ---------------- events from Rust ---------------- */

listen("task", ({ payload: k }) => { S.tasks[k.profile] = k; S.busy.add(k.profile); paintProgress(k.profile); });
listen("game-start", async ({ payload: id }) => {
  delete S.tasks[id];
  S.running.add(id);
  S.logs[id] = [];
  await refresh();
  renderRail();
  paintProgress(id);
  // First launch installed the version: its title-screen art is now available.
  const p = profile(id);
  if (p && !S.pano[p.mc]) { delete S.pano[p.mc]; if (current()?.id === id) mountHero(p); }
  if (current()?.id === id && S.view.tab === "logs") $("#log").innerHTML = "";
});
listen("game-exit", async ({ payload: e }) => {
  S.running.delete(e.profile);
  S.busy.delete(e.profile);
  await refresh();
  renderRail();
  paintProgress(e.profile);
  if (e.error) toast(e.error, "bad");
  if (!e.stopped && e.code && e.code !== 0) toast(e.reason ? tp("Il gioco si è chiuso: {reason}", { reason: tp(e.reason.key, e.reason.vars) }) : tp("Il gioco si è chiuso con un errore (codice {code}). Guarda il Log.", { code: e.code }), "bad");
  const p = profile(e.profile);
  if (p && current()?.id === p.id && ["worlds", "shots"].includes(S.view.tab)) renderTab(p);
});
const LOG_LINES = 3000; // same window as the launcher keeps
listen("log", ({ payload: { profile: id, lines } }) => {
  const log = (S.logs[id] ||= []);
  const fresh = !log.length;
  log.push(...lines);
  const over = log.length - LOG_LINES;
  if (over > 0) log.splice(0, over);
  if (current()?.id !== id || S.view.tab !== "logs") return;
  const el = $("#log");
  if (!el) return;
  const stick = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
  // One DOM write per batch; past the window, redraw the window instead of growing forever.
  if (fresh || over > 0) el.innerHTML = log.map(logLine).join("");
  else el.insertAdjacentHTML("beforeend", lines.map(logLine).join(""));
  if (stick) el.scrollTop = el.scrollHeight;
});

/* ---------------- keyboard ---------------- */

const SHORTCUTS = [
  ["Esc", "Indietro / chiudi"], ["Ctrl+Invio", "Gioca o ferma"], ["Ctrl+1…9", "Vai al profilo"],
  ["Ctrl+Tab", "Scheda successiva"], ["Ctrl+N", "Nuovo profilo"], ["Ctrl+,", "Impostazioni"], ["Ctrl+F  /", "Cerca su Modrinth"],
];

const keyName = (k) => (LANG === "en" ? k.replace("Invio", "Enter") : k);

/** Esc goes back one level: the innermost open thing first, then the page. */
function goBack() {
  const tipEl = document.activeElement?.closest?.(".tip");
  if (tipEl) return tipEl.blur();
  if (S.capeEdit) { S.capeEdit = null; return paintCustomCape(); }
  if (S.login?.code) { S.login = null; return paintLogin(); }
  if (S.importer && !S.importer.running) { S.importer = null; return fadeSwap($("#main .page"), () => render()); }
  if (S.mode === "browse" && current()) { S.mode = "installed"; S.browse = null; return renderTab(current()); }
  if (S.view.kind !== "profile" && S.profiles.length) {
    const back = S.lastProfile && profile(S.lastProfile.id) ? S.lastProfile : { kind: "profile", id: S.profiles[0].id, tab: defaultTab(S.profiles[0]) };
    return go(back);
  }
}

document.addEventListener("keydown", (e) => {
  if (e.key === "F5" || (e.ctrlKey && e.key.toLowerCase() === "r")) return e.preventDefault();
  if ($("#confirm").open) return; // the dialog handles its own keys
  const typing = e.target.closest?.("input, textarea, select, [contenteditable]");
  const ctrl = e.ctrlKey || e.metaKey;
  const p = current();
  if (e.key === "Escape") {
    e.preventDefault();
    if (typing) e.target.blur();
    return goBack();
  }
  if (ctrl && e.key === "Enter" && p) { e.preventDefault(); return $(".play")?.click(); }
  if (ctrl && /^[1-9]$/.test(e.key)) {
    const target = S.profiles[+e.key - 1];
    if (target) { e.preventDefault(); if (current()?.id !== target.id) go({ kind: "profile", id: target.id, tab: defaultTab(target) }); }
    return;
  }
  if (ctrl && e.key === "Tab" && p) {
    e.preventDefault();
    const tabs = [...document.querySelectorAll(".tabs [data-tab]")];
    const i = tabs.findIndex((x) => x.getAttribute("aria-selected") === "true");
    tabs[(i + (e.shiftKey ? -1 : 1) + tabs.length) % tabs.length]?.click();
    return;
  }
  if (ctrl && e.key.toLowerCase() === "n") { e.preventDefault(); return go({ kind: "new" }); }
  if (ctrl && e.key === ",") { e.preventDefault(); return go({ kind: "settings" }); }
  if (p && KINDS[S.view.tab] && !unavailable(p, S.view.tab) && ((ctrl && e.key.toLowerCase() === "f") || (!typing && e.key === "/"))) {
    e.preventDefault();
    if (S.mode !== "browse") { S.mode = "browse"; renderTab(p); runSearch(p); }
    $("#q")?.focus();
  }
});
document.addEventListener("contextmenu", (e) => { if (!e.target.closest("input,textarea,.log,.code")) e.preventDefault(); });

$("#brand-mark").innerHTML = Art.logo("brand-logo");

(async () => {
  // Draw straight away from local data; the skin (network: token refresh, texture) and
  // system info arrive afterwards and are painted in place.
  await refresh();
  S.view = S.profiles.length ? { kind: "profile", id: S.profiles[0].id, tab: defaultTab(S.profiles[0]) } : { kind: "new" };
  render();
  invoke("system_memory_mb").then((m) => (S.sysMem = m)).catch(() => {});
  ensureCatalog().catch(() => {});
  await loadSkin();
  renderRail();
  if (current()) mountFigure(current());
  else if (S.view.kind === "settings") mountSettingsArt();
  setTimeout(() => checkUpdate(true), 2500);
})();

/* ---------------- app updates ---------------- */

// S.update: { state: "checking" | "none" | "available" | "installing" | "error", version?, notes?, pct?, error? }

function updateRow() {
  const u = S.update || {};
  const label = t("Agate {v}", { v: S.version || "" });
  let control;
  if (u.state === "available") {
    control = `<button class="btn primary" data-color="grass" id="update-go">${icon("down")}${t("Aggiorna a {v}", { v: u.version })}</button>`;
  } else if (u.state === "installing") {
    control = `<span class="done">${icon("down", "spin")}${u.pct == null ? t("Installo…") : t("Scarico… {n}%", { n: u.pct })}</span>`;
  } else {
    const status = u.state === "none" ? `<span class="done">${icon("check")}${t("Aggiornato")}</span>`
      : u.state === "error" ? `<span class="meta" title="${esc(u.error || "")}">${t("Controllo non riuscito")}</span>` : "";
    control = `${status}<button class="btn" id="update-check" ${u.state === "checking" ? "disabled" : ""}>${u.state === "checking" ? icon("down", "spin") : ""}${t(u.state === "checking" ? "Controllo…" : "Controlla aggiornamenti")}</button>`;
  }
  const help = u.state === "available" && u.notes ? u.notes : tp("Agate controlla da solo all'avvio. L'aggiornamento si installa e l'app si riapre: profili e mondi restano dove sono.");
  return pref(label, `<div class="row">${control}</div>`, help);
}

function paintUpdate() {
  const row = $("#update-row");
  if (row) row.innerHTML = updateRow();
  const chip = $("#update-chip");
  const u = S.update || {};
  chip.hidden = !(u.state === "available" || u.state === "installing");
  chip.disabled = u.state === "installing";
  chip.innerHTML = u.state === "installing"
    ? `${icon("down", "spin")}${u.pct == null ? t("Installo…") : `${u.pct}%`}`
    : `<span class="update-dot"></span>${t("Aggiorna")}`;
  chip.title = u.state === "available" ? tp("Installa Agate {v}: l'app si riapre da sola.", { v: u.version }) : "";
}

async function checkUpdate(quiet) {
  if (S.update?.state === "installing") return;
  S.update = { state: "checking" };
  paintUpdate();
  try {
    const u = await invoke("update_check");
    S.update = u ? { state: "available", version: u.version, notes: u.notes } : { state: "none" };
  } catch (e) {
    // At startup a failed check (offline, no release yet) stays silent.
    S.update = quiet ? null : { state: "error", error: String(e) };
  }
  paintUpdate();
}

async function installUpdate() {
  const u = S.update;
  if (!u || u.state !== "available") return;
  // Not destructive (data stays where it is), so no dialog: the click is the decision.
  if (S.running.size || S.busy.size) return toast(tp("Chiudi il gioco prima di aggiornare."), "bad");
  S.update = { ...u, state: "installing", pct: 0 };
  paintUpdate();
  try { await invoke("update_install"); }
  catch (e) { S.update = { ...u, state: "available" }; paintUpdate(); toast(String(e), "bad"); }
}

listen("update-progress", ({ payload: { done, total } }) => {
  if (S.update?.state !== "installing") return;
  S.update.pct = total ? Math.min(100, Math.floor((done / total) * 100)) : null;
  if (total && done >= total) S.update.pct = null; // downloaded: the installer takes over
  paintUpdate();
});

document.addEventListener("click", (e) => {
  if (e.target.closest("#update-check")) return checkUpdate(false);
  if (e.target.closest("#update-go, #update-chip")) return installUpdate();
});
