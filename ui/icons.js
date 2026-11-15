"use strict";
/* Agate's own art: the logo (the Agate mark) and the profile icons
   (16×16 pixel art, tinted with the profile colour). All SVG, crisp at any size. */

const Art = (() => {
  // Palette. 1/2/3 are the profile colour (light, base, dark); the rest are fixed materials.
  const FILL = {
    k: "#19140f", h: "#fff6e0",
    1: "var(--p1)", 2: "var(--p2)", 3: "var(--p3)",
    w: "#a56f3e", W: "#6d4526",
    d: "#8b5a34", D: "#63401f", e: "#a8744a", E: "#4f3218",
    s: "#d8d1c4", S: "#a59d90", g: "#bfe3f0",
  };

  const ICONS = {
    block: [
      "......kkkk......",
      "....kk1111kk....",
      "..kk11111111kk..",
      "kk111111111111kk",
      "k22111111111133k",
      "k22221111113333k",
      "k22222211333333k",
      "k22222223333333k",
      "k2h222223333333k",
      "k22222223333333k",
      "k22222223333333k",
      "k22222223333333k",
      "kk222222333333kk",
      "..kk22223333kk..",
      "....kk2233kk....",
      "......kkkk......",
    ],
    grass: [
      "......kkkk......",
      "....kk1111kk....",
      "..kk11111111kk..",
      "kk111111111111kk",
      "k22111111111133k",
      "k22221111113333k",
      "k2d22221133D333k",
      "k2dd2dddD3DDD3Dk",
      "kdddddddDDDDDDDk",
      "kdedddddDDDEDDDk",
      "kdddddedDEDDDDDk",
      "kddeddddDDDDDEDk",
      "kkddddddDDDDDDkk",
      "..kkddddDDDDkk..",
      "....kkddDDkk....",
      "......kkkk......",
    ],
    gem: [
      "................",
      "................",
      "....kkkkkkkk....",
      "...k1h112222k...",
      "..k1h11122222k..",
      ".k111111222223k.",
      "k11111122222333k",
      ".k111122222333k.",
      "..k1112222333k..",
      "...k11222333k...",
      "....k122233k....",
      ".....k1233k.....",
      "......k23k......",
      ".......kk.......",
      "................",
      "................",
    ],
    pickaxe: [
      "................",
      "...kkkkkk.......",
      "..k1h11122k.....",
      "...kkkkk1222k...",
      "........kk223k..",
      ".......kWkk223k.",
      "......kWk..k23k.",
      ".....kWk....k3k.",
      "....kWk.....k3k.",
      "...kWk......k3k.",
      "..kWk........kk.",
      ".kWk............",
      "kWk.............",
      "kk..............",
      "................",
      "................",
    ],
    sword: [
      "............kkk.",
      "...........khhk.",
      "..........kh13k.",
      ".........kh13k..",
      "........kh13k...",
      ".......kh13k....",
      "..k...kh13k.....",
      "..kk.kh13k......",
      "..kWkh13k.......",
      "...kW13k........",
      "...kkWk.........",
      "..kWkkWk........",
      ".kWk..kk........",
      "kWk.............",
      "kk..............",
      "................",
    ],
    tree: [
      "................",
      ".....kkkkkk.....",
      "...kk111122kk...",
      "..k11h1111222k..",
      ".k111111112222k.",
      ".k111111122223k.",
      ".k111112222333k.",
      ".k122222222333k.",
      "..k2222223333k..",
      "...kk222233kk...",
      ".....kkWWkk.....",
      "......kWwk......",
      "......kWwk......",
      ".....kWWwwk.....",
      ".....kkkkkk.....",
      "................",
    ],
    potion: [
      "................",
      "......kkkk......",
      "......kWWk......",
      "......kggk......",
      ".....kkggkk.....",
      "....kggggggk....",
      "...kgh22222gk...",
      "...kh1222223k...",
      "...k12222223k...",
      "...k22222233k...",
      "...k22222333k...",
      "....k223333k....",
      ".....kkkkkk.....",
      "................",
      "................",
      "................",
    ],
    heart: [
      "................",
      "................",
      "................",
      "...kkk....kkk...",
      "..k1h1k..k112k..",
      ".k1h111kk11222k.",
      ".k111111112222k.",
      ".k111111122223k.",
      "..k1111222233k..",
      "...k11222233k...",
      "....k122233k....",
      ".....k1233k.....",
      "......k23k......",
      ".......kk.......",
      "................",
      "................",
    ],
    house: [
      "................",
      ".......kk.......",
      "......k12k......",
      ".....k1h22k.....",
      "....k11h222k....",
      "...k11122223k...",
      "..k1111222233k..",
      ".kkkkkkkkkkkkkk.",
      "..kssssssssssk..",
      "..ksggsssWWssk..",
      "..ksggsssWwssk..",
      "..kSSSSSSWwSSk..",
      "..kSSSSSSWwSSk..",
      "..kkkkkkkkkkkk..",
      "................",
      "................",
    ],
    flame: [
      "................",
      "........k.......",
      ".......k1k......",
      "..k...k11k..k...",
      ".k1k..k1h1k.k1k.",
      ".k11kk1hh12kk2k.",
      ".k1111hhh12223k.",
      ".k111hhhh12223k.",
      "..k11hhhh1223k..",
      "..k11hhh12223k..",
      "..k112hh122233k.",
      "...k1222222333k.",
      "....kk222233kk..",
      "......kkkkk.....",
      "................",
      "................",
    ],
  };

  /** One run of rects per same-colour stretch of a row, crisp-edged. */
  function pixels(rows) {
    let out = "";
    rows.forEach((row, y) => {
      let x = 0;
      while (x < row.length) {
        const c = row[x];
        let n = 1;
        while (row[x + n] === c) n++;
        if (c !== ".") out += `<rect x="${x}" y="${y}" width="${n}" height="1" style="fill:${FILL[c]}"/>`;
        x += n;
      }
    });
    return out;
  }
  const cache = {};

  /** Profile icon as inline SVG; `color` is a profile colour name. */
  /** Profile icon as inline SVG. `color` is a profile colour name; without one it takes the
   *  colour of the surrounding element. */
  function icon(name, color, cls = "") {
    const body = (cache[name] ||= pixels(ICONS[name] || ICONS.block));
    return `<svg class="picon ${cls}" viewBox="0 0 16 16" shape-rendering="crispEdges" ${color ? `data-color="${color}"` : ""} aria-hidden="true">${body}</svg>`;
  }

  /** The Agate mark: the A with a block under its arch (brand/agate-mark.svg), in the text colour. */
  function logo(cls = "") {
    return `<svg class="logo ${cls}" viewBox="0 0 760 552" aria-hidden="true" fill="currentColor">
      <path d="M0 550 332.5 0 427.5 0 552.1 206 450.8 280 380 162.9 146 550ZM470.1 312 571.4 238 760 550 614 550Z"/>
      <g class="logo-cube"><path d="M287 367 380 313.5 473 367 380 420.5ZM276 400 366 452 366 552 276 500ZM394 452 484 400 484 500 394 552Z"/></g>
    </svg>`;
  }

  return { icon, logo, names: Object.keys(ICONS) };
})();
