"use strict";
/* Game art for the UI: a 3D player built from the skin texture, the skin's face
   as an avatar, and the title-screen panorama as a CSS cube map. */

/** Loads an image; app-served files come with CORS so canvases stay readable. */
const loadImage = (src) => new Promise((ok, fail) => {
  const i = new Image();
  if (/^https?:/.test(src)) i.crossOrigin = "anonymous";
  i.onload = () => ok(i);
  i.onerror = fail;
  i.src = src;
});

const Skin = (() => {
  // [w, h, d, u, v] in skin pixels. Overlay UVs follow the 64×64 layout.
  const PARTS = {
    head: { box: [8, 8, 8], uv: [0, 0], over: [32, 0], at: [0, -16, 0] },
    body: { box: [8, 12, 4], uv: [16, 16], over: [16, 32], at: [0, -8, 0] },
    rarm: { box: [4, 12, 4], uv: [40, 16], over: [40, 32], at: [-6, -8, 0], limb: "arm-r" },
    larm: { box: [4, 12, 4], uv: [32, 48], over: [48, 48], at: [6, -8, 0], limb: "arm-l", legacy: [40, 16] },
    rleg: { box: [4, 12, 4], uv: [0, 16], over: [0, 32], at: [-2, 4, 0], limb: "leg-r" },
    lleg: { box: [4, 12, 4], uv: [16, 48], over: [0, 48], at: [2, 4, 0], limb: "leg-l", legacy: [0, 16] },
  };

  /** One textured face. Texture coordinates are in a 64-unit-wide space (texH units tall). */
  function face(src, s, texH, x, y, w, h, transform) {
    const el = document.createElement("i");
    el.style.cssText = `width:${w * s}px;height:${h * s}px;margin:${(-h * s) / 2}px 0 0 ${(-w * s) / 2}px;` +
      `background-image:url("${src}");background-size:${64 * s}px ${texH * s}px;background-position:${-x * s}px ${-y * s}px;transform:${transform}`;
    return el;
  }

  function box(src, s, texH, [w, h, d], [u, v], inflate) {
    const b = document.createElement("div");
    b.className = "box";
    if (inflate) b.style.transform = `scale3d(${(w + inflate) / w},${(h + inflate) / h},${(d + inflate) / d})`;
    b.append(
      face(src, s, texH, u + d, v + d, w, h, `translateZ(${(d / 2) * s}px)`),
      face(src, s, texH, u + 2 * d + w, v + d, w, h, `rotateY(180deg) translateZ(${(d / 2) * s}px)`),
      face(src, s, texH, u, v + d, d, h, `rotateY(-90deg) translateZ(${(w / 2) * s}px)`),
      face(src, s, texH, u + d + w, v + d, d, h, `rotateY(90deg) translateZ(${(w / 2) * s}px)`),
      face(src, s, texH, u + d, v, w, d, `rotateX(90deg) translateZ(${(h / 2) * s}px)`),
      face(src, s, texH, u + d + w, v, w, d, `rotateX(-90deg) translateZ(${(h / 2) * s}px)`),
    );
    return b;
  }

  /**
   * The skin as the game uses it: Minecraft makes the base layer (head, body, arms, legs)
   * fully opaque, so pixels a skin leaves transparent there (e.g. a slim-armed skin worn
   * with the classic model) show their colour instead of a hole. Only the overlay layer
   * keeps transparency. Returns a data URL plus the image's size.
   */
  const prepared = new Map();
  async function gameSkin(src) {
    if (prepared.has(src)) return prepared.get(src);
    const img = await loadImage(src);
    const w = img.naturalWidth, h = img.naturalHeight, k = w / 64;
    const c = document.createElement("canvas");
    c.width = w;
    c.height = h;
    const g = c.getContext("2d");
    g.drawImage(img, 0, 0);
    const base = h === w ? [[0, 0, 32, 16], [0, 16, 64, 32], [16, 48, 48, 64]] : [[0, 0, 32, 16], [0, 16, 64, 32]];
    const data = g.getImageData(0, 0, w, h);
    for (const [x0, y0, x1, y1] of base) {
      for (let y = y0 * k; y < y1 * k; y++) for (let x = x0 * k; x < x1 * k; x++) data.data[(y * w + x) * 4 + 3] = 255;
    }
    g.putImageData(data, 0, 0);
    const out = { url: c.toDataURL("image/png"), width: w, height: h };
    prepared.set(src, out);
    return out;
  }

  /** A posable 3D player. `s` = screen pixels per skin pixel. Drag to turn it. */
  async function figure(skinSrc, { slim = false, s = 7, interactive = true, cape = null, yaw: startYaw = -24 } = {}) {
    const skin = await gameSkin(skinSrc);
    const src = skin.url;
    const img = { naturalWidth: skin.width, naturalHeight: skin.height };
    const texH = img.naturalHeight === 32 ? 32 : 64;
    const legacy = texH === 32;
    const root = document.createElement("div");
    root.className = "skin3d";
    root.style.cssText = `--s:${s}px;width:${16 * s}px;height:${32 * s}px`;
    // .spin turns the player gently when idle; .model takes the drag rotation.
    const spin = document.createElement("div");
    spin.className = "spin";
    const model = document.createElement("div");
    model.className = "model";
    spin.append(model);
    root.append(spin);
    for (const [name, p] of Object.entries(PARTS)) {
      const [w0, h, d] = p.box;
      const w = slim && name.endsWith("arm") ? 3 : w0;
      let [x, y, z] = p.at;
      if (slim && name === "rarm") x += 0.5;
      if (slim && name === "larm") x -= 0.5;
      const uv = legacy && p.legacy ? p.legacy : p.uv;
      // Pivot at the top of the part (shoulder, hip, neck), the box hangs below it.
      const pivot = document.createElement("div");
      pivot.className = `part ${p.limb || name}`;
      pivot.style.transform = `translate3d(${x * s}px,${y * s}px,${z * s}px)`;
      const hang = document.createElement("div");
      hang.className = "hang";
      hang.style.setProperty("--hy", `${(h / 2) * s}px`);
      hang.append(box(src, s, texH, [w, h, d], uv, 0));
      if (!legacy || name === "head") hang.append(box(src, s, texH, [w, h, d], p.over, name === "head" ? 1 : 0.5));
      pivot.append(hang);
      model.append(pivot);
    }
    if (cape) {
      const capeImg = await loadImage(cape).catch(() => null);
      if (capeImg) {
        // 10×16×1 box hanging from the shoulders behind the back; its outer art faces backwards.
        const capeH = (64 * capeImg.naturalHeight) / capeImg.naturalWidth;
        const pivot = document.createElement("div");
        pivot.className = "part cape";
        pivot.style.transform = `translate3d(0,${-8 * s}px,${-2.5 * s}px)`;
        const hang = document.createElement("div");
        hang.className = "hang";
        hang.style.setProperty("--hy", `${8 * s}px`);
        const b = box(cape, s, capeH, [10, 16, 1], [0, 0], 0);
        b.style.transform = "rotateY(180deg)";
        hang.append(b);
        pivot.append(hang);
        model.append(pivot);
      }
    }
    if (interactive) {
      let drag = null, yaw = startYaw;
      const apply = () => model.style.setProperty("--yaw", `${yaw}deg`);
      apply();
      root.addEventListener("pointerdown", (e) => { drag = { x: e.clientX, yaw }; root.setPointerCapture(e.pointerId); root.classList.add("dragging"); });
      root.addEventListener("pointermove", (e) => { if (drag) { yaw = drag.yaw + (e.clientX - drag.x) * 0.6; apply(); } });
      const end = () => { drag = null; root.classList.remove("dragging"); };
      root.addEventListener("pointerup", end);
      root.addEventListener("pointercancel", end);
    }
    return root;
  }

  /** The skin's face with its hat layer, as a crisp canvas. */
  async function head(src, size = 32) {
    const img = await loadImage((await gameSkin(src)).url);
    const c = document.createElement("canvas");
    c.width = c.height = size;
    c.className = "head";
    const g = c.getContext("2d");
    g.imageSmoothingEnabled = false;
    g.drawImage(img, 8, 8, 8, 8, 0, 0, size, size);
    g.drawImage(img, 40, 8, 8, 8, 0, 0, size, size);
    return c;
  }

  /** The outer face of a cape (what you see from behind), as a crisp canvas. */
  async function capeFront(src, scale = 4) {
    const img = await loadImage(src);
    const u = img.naturalWidth / 64;
    const c = document.createElement("canvas");
    c.width = 10 * scale;
    c.height = 16 * scale;
    const g = c.getContext("2d");
    g.imageSmoothingEnabled = false;
    g.drawImage(img, 1 * u, 1 * u, 10 * u, 16 * u, 0, 0, c.width, c.height);
    return c;
  }

  return { figure, head, capeFront };
})();

/* Turns any picture into a cape texture (64×32 layout, drawn at 16× for detail). */
const CapeMaker = (() => {
  const K = 16;
  /** A picture already laid out as a cape texture (2:1, width a multiple of 64). */
  const isTemplate = (img) => img.naturalWidth === img.naturalHeight * 2 && img.naturalWidth % 64 === 0;

  /** Average colour of a picture, as #rrggbb. */
  function average(img) {
    const c = document.createElement("canvas");
    c.width = c.height = 8;
    const g = c.getContext("2d");
    g.drawImage(img, 0, 0, 8, 8);
    const d = g.getImageData(0, 0, 8, 8).data;
    let r = 0, gg = 0, b = 0, n = 0;
    for (let i = 0; i < d.length; i += 4) if (d[i + 3] > 32) { r += d[i]; gg += d[i + 1]; b += d[i + 2]; n++; }
    const h = (v) => Math.round(v / Math.max(1, n)).toString(16).padStart(2, "0");
    return `#${h(r)}${h(gg)}${h(b)}`;
  }

  function place(g, img, x, y, w, h, fit) {
    const s = fit === "contain" ? Math.min(w / img.naturalWidth, h / img.naturalHeight) : Math.max(w / img.naturalWidth, h / img.naturalHeight);
    const dw = img.naturalWidth * s, dh = img.naturalHeight * s;
    g.save();
    g.beginPath();
    g.rect(x, y, w, h);
    g.clip();
    g.drawImage(img, x + (w - dw) / 2, y + (h - dh) / 2, dw, dh);
    g.restore();
  }

  /** Canvas with the finished texture. */
  function make(img, { fit = "cover", color } = {}) {
    if (isTemplate(img)) {
      const c = document.createElement("canvas");
      c.width = img.naturalWidth;
      c.height = img.naturalHeight;
      c.getContext("2d").drawImage(img, 0, 0);
      return c;
    }
    const c = document.createElement("canvas");
    c.width = 64 * K;
    c.height = 32 * K;
    const g = c.getContext("2d");
    g.imageSmoothingQuality = "high";
    g.fillStyle = color || average(img);
    g.fillRect(0, 0, 22 * K, 17 * K); // cape box: edges, top and bottom
    place(g, img, 1 * K, 1 * K, 10 * K, 16 * K, fit); // outer face, seen from behind
    place(g, img, 12 * K, 1 * K, 10 * K, 16 * K, fit); // inner face, against the back
    g.fillStyle = "rgb(0 0 0 / .35)";
    g.fillRect(12 * K, 1 * K, 10 * K, 16 * K); // darker, like cloth in shadow
    g.fillStyle = color || average(img);
    g.fillRect(22 * K, 0, 24 * K, 22 * K); // elytra
    place(g, img, 24 * K, 2 * K, 10 * K, 20 * K, fit);
    return c;
  }

  const toPng = (canvas) => new Promise((ok) => canvas.toBlob((b) => b.arrayBuffer().then((a) => ok(Array.from(new Uint8Array(a)))), "image/png"));
  return { make, toPng, average, isTemplate };
})();

const Pano = (() => {
  /** Builds the six-face cube map; the camera sits in the middle and slowly turns. */
  function mount(host, faces) {
    host.querySelector(".pano")?.remove();
    const pano = document.createElement("div");
    pano.className = "pano";
    pano.setAttribute("aria-hidden", "true");
    const cam = document.createElement("div");
    cam.className = "pano-cam";
    // Minecraft order: 0 front, 1 right, 2 back, 3 left, 4 up, 5 down.
    const t = ["", "rotateY(-90deg)", "rotateY(180deg)", "rotateY(90deg)", "rotateX(-90deg)", "rotateX(90deg)"];
    faces.forEach((src, i) => {
      const f = document.createElement("img");
      f.src = src;
      f.alt = "";
      f.decoding = "async";
      f.style.transform = `${t[i]} translateZ(calc(var(--h) * -1))`;
      cam.append(f);
    });
    pano.append(cam);
    host.prepend(pano);
    const fit = () => {
      const w = host.clientWidth || 900;
      const p = Math.round(w * 0.42); // ~100° horizontal field of view
      pano.style.setProperty("--p", `${p}px`);
      pano.style.setProperty("--h", `${p}px`);
    };
    fit();
    new ResizeObserver(fit).observe(host);
    // Already-cached faces appear at once; only a first load fades in.
    const imgs = [...cam.querySelectorAll("img")];
    if (imgs.every((im) => im.complete && im.naturalWidth)) pano.classList.add("in", "instant");
    let ready = 0;
    imgs.forEach((im) => im.addEventListener("load", () => { if (++ready === 6) pano.classList.add("in"); }));
    return pano;
  }
  return { mount };
})();
