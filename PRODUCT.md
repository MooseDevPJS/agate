# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Tauri 2 desktop app: Rust backend (download, install, launch, auth), static HTML/CSS/JS frontend rendered in the system WebView2. No bundler. Chosen by the user over pure-Rust UI (egui/iced).

## Users

A single Minecraft Java player on Windows who owns a paid Microsoft account. Wants to jump into the game quickly across several setups (vanilla, a few Fabric mods, OptiFine) on different Minecraft versions, without the weight of the official launcher or full-featured mod managers.

## Product Purpose

A light, fast launcher: create profiles (instances), each with its own Minecraft version and loader, add simple mods from Modrinth or install OptiFine, press Play. Play installs whatever is missing (game files, libraries, assets, the right Java runtime) and launches. Success: from cold start to in-game in as few clicks as possible, with settings carried between profiles.

## Positioning

Modrinth App's core loop (profiles + Modrinth mods) stripped to essentials, plus first-class OptiFine and shared game settings (keybinds, options, server list) synced across every profile so each new instance feels like home.

## Operating Context

Used on a desktop PC, often in the evening, right before playing. Sessions are short: open, pick profile, Play, close. Occasional longer visits to set up a new profile or browse mods.

## Capabilities and Constraints

- Profiles: name, Minecraft version (releases, optionally snapshots), loader Vanilla / Fabric / OptiFine, memory, per-profile game directory.
- Mods: Modrinth search filtered by the profile's version and Fabric; install with required dependencies; remove.
- OptiFine: fetched from optifine.net for the profile's version, patched locally.
- Java: auto-downloaded (Eclipse Temurin) matching the version's required major.
- Accounts: Microsoft login via device code; offline name for singleplayer.
- Shared settings: options.txt and servers.dat merged into a shared store after each session and seeded into every profile before launch.
- Undecided: Forge/NeoForge/Quilt, modpacks, skins — out of scope for now.

## Brand Commitments

Name: undecided; working name "Blocco". User asked for clean, minimal, fast; Modrinth App as reference for structure.

## Product Principles

1. Play is always one click away; everything else is secondary.
2. Install on demand, never ask the user to fetch files.
3. Show real progress for every long operation.
4. Fewer features, each fully working.
