---
name: Agate
description: A light Minecraft launcher that opens on the title screen of the version you're about to play.
colors:
  bg: "#141311"
  rail: "#0e0d0c"
  raised: "#1c1a18"
  high: "#26231f"
  seam: "#2b2825"
  seam-strong: "#3c3833"
  ink: "#f4efe5"
  ink-2: "#bfb7a8"
  ink-3: "#8c8579"
  ink-dark: "#12110f"
  chapter-grass: "#7cc34f"
  chapter-amber: "#f0a93c"
  chapter-sky: "#5ab2df"
  chapter-redstone: "#e65a48"
  chapter-amethyst: "#ab86e3"
  bad: "#ef6b5d"
  scrim: "rgb(10 9 8 / .62)"
  overlay-control: "rgb(14 13 12 / .82)"
  shadow-soft: "rgb(0 0 0 / .35)"
  nametag-bg: "rgb(0 0 0 / .45)"
  nametag-ink: "#ffffff"
  window-close: "#c42b1c"
typography:
  display:
    fontFamily: "Archivo, system-ui, sans-serif"
    fontSize: "58px"
    fontWeight: 800
    lineHeight: 1
    letterSpacing: "-0.02em"
    fontVariation: "\"wdth\" 116"
  headline:
    fontFamily: "Archivo, system-ui, sans-serif"
    fontSize: "30px"
    fontWeight: 800
    letterSpacing: "-0.02em"
    fontVariation: "\"wdth\" 112"
  code:
    fontFamily: "Archivo, system-ui, sans-serif"
    fontSize: "36px"
    fontWeight: 800
    letterSpacing: "0.1em"
  play:
    fontFamily: "Archivo, system-ui, sans-serif"
    fontSize: "20px"
    fontWeight: 800
  title:
    fontFamily: "Archivo, system-ui, sans-serif"
    fontSize: "16px"
    fontWeight: 650
  brand:
    fontFamily: "Archivo, system-ui, sans-serif"
    fontSize: "17px"
    fontWeight: 800
  body:
    fontFamily: "Archivo, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.45
    fontFeature: "tnum"
  small:
    fontFamily: "Archivo, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 500
  label:
    fontFamily: "Archivo, system-ui, sans-serif"
    fontSize: "12.5px"
    fontWeight: 600
  mono:
    fontFamily: "ui-monospace, Cascadia Mono, Consolas, monospace"
    fontSize: "12px"
    lineHeight: 1.55
rounded:
  xs: "2px"
  sm: "5px"
  md: "7px"
  lg: "12px"
  pill: "22px"
  round: "50%"
spacing:
  xs: "6px"
  sm: "10px"
  md: "16px"
  lg: "24px"
  xl: "32px"
components:
  play-button:
    backgroundColor: "{colors.chapter-grass}"
    textColor: "{colors.ink-dark}"
    rounded: "{rounded.lg}"
    height: "56px"
    padding: "0 28px 0 22px"
  button-primary:
    backgroundColor: "{colors.chapter-grass}"
    textColor: "{colors.ink-dark}"
    rounded: "{rounded.md}"
    height: "36px"
  button-secondary:
    backgroundColor: "{colors.raised}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    height: "36px"
  button-danger:
    textColor: "{colors.bad}"
    rounded: "{rounded.md}"
    height: "36px"
  input:
    backgroundColor: "{colors.rail}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    height: "40px"
  content-card:
    backgroundColor: "{colors.raised}"
    rounded: "{rounded.lg}"
  nametag:
    textColor: "#ffffff"
    rounded: "{rounded.xs}"
---

# Agate design system

## Overview

**North star: the title screen.** Each profile opens like Minecraft's main menu: the slowly turning panorama **of that profile's own version** (read from the game files: the asset index first, the jar as fallback) fills the top of the page, with the player's 3D skin standing in the scene and the name tag above it, just like in-game. Below, the content dissolves into the page. It's an Operate interface: the controls are standard, and the world comes in only through the imagery, the type, the colours and one signature move.

**Signature move:** the Play button is also the progress bar (it fills in proportion to the bytes actually downloaded); in the side rail the row repaints in place with percentage and a thin bar.

## Colors

- **Obsidian** ground: warm near-black in three steps (`rail` → `bg` → `raised`/`high`), separation by 1px seams. No blue-slate.
- Warm ink (`ink`, `ink-2`, `ink-3`); `ink-dark` on coloured buttons.
- Five profile colours (`chapter-*`): the current one becomes `--c` and colours Play, the active tab, toggles, focus and selection. Everything else stays neutral.
- Over the panorama: two functional scrims (dark on the left for the title, a fade to `bg` at the bottom). Never decorative gradients elsewhere.

## Typography

One family, **Archivo** (variable, weight and width axes, self-hosted, OFL). Titles at width 112–118% and weight 800 (poster-like); interface at normal width. Tabular numerals. Monospace only for the log.

## Layout

Frameless window with a custom 36px title bar overlaying the content (the panorama reaches the top edge). 252px rail (216px under 1060px). Profile: hero `clamp(330px, 58vh, 540px)` → sticky tabs (Mod, Texture pack, Shader, Mondi, Screenshot, Log, Profilo) → content up to 1120px. Settings: 300px player card + column.

## Elevation & Depth

Flat with tonal layering. Real depth only in the game art: CSS 3D panorama cube, 3D player with an elliptical shadow at its feet. One soft shadow for dialogs, toasts and the Play button.

## Shapes

5px small controls, 7px buttons/inputs, 12px cards, lists and Play. The 2px name tag copies the one in-game.

## Components

- **Hero**: panorama, title, facts, Play + folder, "come <account>"; 3D figure on the right (draggable to rotate).
- **Mod**: list rows (icon, title, version, toggle, remove). **Texture pack / Shader** while browsing: cards with the Modrinth image in 16:9 and the icon overlapping; installed ones as rows with "Usa"/"Attivo".
- **Worlds**: cards with the world's icon, last played, "Entra" (Quick Play, 1.20+).
- **Screenshot**: 16:9 grid, click opens the file.
- **Player card** (Settings): large 3D figure with the cape you see on yourself, Classic/Slim model, Change skin, a grid of capes under **Mantello Microsoft** (seen by everyone; tiles at least 72px, "Nessuno" to hide it) and **Mantello personalizzato** (seen only by you, Fabric 26.x).
- **Cape editor**: inline in the card, never a modal. While editing, the figure turns its back (165°) and shows the cape live; controls: Riempi/Adatta, Bordi colour (default: the picture's average colour), Cambia immagine, Salva/Annulla. A 2:1 picture is treated as a ready-made template and used as it is.
- **Floating scrollbar** for the page: drawn over the content (6px, 9px on hover), it never takes up width, so switching tabs never resizes the hero.
- **Worlds and servers**: two sections in the same tab; a server card shows its saved icon, name, address and "Entra".
- **Update chip**: when a new version exists, a small pill (green dot + "Aggiorna") sits in the titlebar beside the window buttons, on the same dark overlay as the hero controls; it never appears while checking or on errors. One click installs (no dialog: nothing is lost) and the chip turns into the download percentage. The same state lives in Settings → Preferenze, last row "Agate <version>", with "Controlla aggiornamenti".
- Segmented control, toggle, colour swatch with cube, native `<dialog>` only for destructive actions, toasts at bottom right.

## Logo and icons

- **Logo**: the Agate mark, a geometric A whose right leg is cut by a slanted gap, with an isometric block standing under the arch. Masters in `brand/`: `agate.svg` (white mark on a black rounded square, radius ≈ 23%, the app icon) and `agate-mark.svg` (mark alone, `currentColor`, viewBox 760×552). Exact geometry: every leg edge has the same slope, the block is isometric and centred on the A's axis. In the rail (`Art.logo`) the mark is drawn alone in `--ink` at 29px wide, with no badge (a black square on the dark rail would disappear); on hover the block rises slightly under the arch (no motion when animations are reduced). `src-tauri/icons/icon.ico` (16–256px) and `icon.png` (512px) are rendered from the same geometry; `brand/agate-1024.png` for anything else.
- **Never**: recolour the mark with a chapter colour, add effects (glow, gradients), or put it back on a grass block. White on black, or ink on the obsidian surfaces.
- **Profile icons** (`Art.icon`): 10 designs in 16×16 pixel art (Blocco, Erba, Gemma, Piccone, Spada, Albero, Pozione, Cuore, Casa, Fiamma) with `shape-rendering: crispEdges`. Palette: outline `#19140f`, highlight `#fff6e0`, fixed materials (wood, earth, stone, glass) and three tones of the profile colour (`--p1` light, `--p2` base, `--p3` dark): every icon works with all 5 colours. The colour swatches show the chosen icon.

## Copy and help

Few words on screen. Labels name the thing; explanations live in a **"?" tip** (`.tip`, help icon) that opens on hover and keyboard focus. Long explanations stay only in empty states, where they teach what to do. Preferences are **rows** (label + tip on the left, control on the right) in a bordered group; rare options go under **Avanzate**, a disclosure with a rotating chevron. Every string goes through `t()`/`tp()` (`ui/i18n.js`): Italian is the source, English the translation; the language is chosen in Preferences.

## Motion

- Hero: title, facts and actions fade in in sequence (420ms, 70ms steps); the figure fades in when the skin is ready. Page and profile changes use only opacity, never vertical movement (two stacked slides read as a "jump").
- Lists, cards, worlds and screenshots: fade in with a 28ms stagger per item (max 12); the rail rises only when the app opens.
- Tabs: a single underline slides to the selected tab (320ms).
- Player: slow idle rotation that stops on hover/drag, arms swaying, cape fluttering.
- Play: a sheen on hover; while busy, the fill follows the bytes.
- Switching tabs and pages: the old content fades out (130ms) before the new one rises.
- Profile icons hop on hover; the chosen icon or colour gives a small pop.
- Everything uses `--ease` (exponential out, no overshoot). Preference **Animazioni**: Auto (follows Windows, which with "Animation effects" off reports reduced motion), Sempre, Ridotte; the `reduce-motion` class on `<html>` turns them off.

## Do's and Don'ts

- Do: use the real game art (panorama, skin, world icons, Modrinth images) instead of decoration.
- Do: show real progress and explain errors (crash reason read from the log).
- Don't: glow, glass, decorative textures, pixel fonts in the interface, eyebrows above titles.
- Don't: fill every button in a list with colour: only the primary action is filled.
