"use strict";
/* Interface language. Italian is the source text; `EN` maps it to English.
   Placeholders {name} are filled by t(); values are HTML-escaped unless the
   name starts with "_" (already-built markup). */

const EN = {
  // shell
  "Profili": "Profiles", "Nuovo profilo": "New profile", "Impostazioni": "Settings", "Accedi": "Sign in",
  "Nessun account": "No account", "Nessun profilo ancora.": "No profiles yet.", "Microsoft": "Microsoft", "Offline": "Offline",
  "Riduci a icona": "Minimize", "Ingrandisci": "Maximize", "Chiudi": "Close", "Annulla": "Cancel",
  // time
  "ora": "now", "{n} min fa": "{n} min ago", "{n} h fa": "{n} h ago", "ieri": "yesterday", "{n} giorni fa": "{n} days ago",
  "Nuovo": "New", "In gioco": "Playing",
  // progress
  "Accesso all'account": "Signing in", "Lettura versione": "Reading version", "Librerie": "Libraries", "Risorse di gioco": "Game assets",
  "Installazione OptiFine": "Installing OptiFine", "Avvio del gioco": "Starting the game", "Estrazione Java {v}": "Extracting Java {v}",
  "Preparazione": "Preparing", "In corso…": "Working…", "{a} di {b}": "{a} of {b}", "{a} di {b} file": "{a} of {b} files",
  // hero
  "Gioca": "Play", "Ferma": "Stop", "Il gioco è aperto": "The game is running", "come {name}": "as {name}",
  "Serve un account per giocare": "Sign in to play", "Apri la cartella del profilo": "Open the profile folder",
  "Memoria": "Memory", "Ultima partita {_when}": "Last played {_when}", "Mai avviato": "Never played", "ultima versione": "latest version",
  "Mod": "Mods", "Texture pack": "Texture packs", "Shader": "Shaders", "Mondi": "Worlds", "Screenshot": "Screenshots", "Log": "Log", "Profilo": "Profile",
  // content
  "Installati": "Installed", "Sfoglia Modrinth": "Browse Modrinth", "Cerca {what} per {mc}": "Search {what} for {mc}",
  "mod": "mods", "texture pack": "texture packs", "shader": "shaders",
  "Il gioco è aperto: cambia texture pack e shader dal gioco, o chiudilo prima.": "The game is running: change texture packs and shaders in game, or close it first.",
  "Nessuna mod installata": "No mods installed", "Sfoglia Modrinth per trovare mod per Minecraft {mc}. Fabric API e le altre dipendenze arrivano da sole.": "Browse Modrinth for mods for Minecraft {mc}. Fabric API and other dependencies come along automatically.",
  "Nessun texture pack": "No texture packs", "Cambia l'aspetto di blocchi, oggetti e interfaccia. Quello che installi viene attivato subito.": "Change how blocks, items and menus look. What you install is turned on right away.",
  "Nessuno shader": "No shaders", "Luci, ombre e acqua realistiche. Iris viene installato in automatico con il primo shader.": "Realistic light, shadows and water. Iris is installed automatically with your first shader.",
  "Luci, ombre e acqua realistiche, tramite OptiFine.": "Realistic light, shadows and water, through OptiFine.",
  "Disattiva": "Disable", "Attiva": "Enable", "Attiva {name}": "Enable {name}", "Attivo": "Active", "Usa": "Use", "Rimuovi": "Remove", "Rimuovi {name}": "Remove {name}",
  "Aggiunto a mano": "Added by hand", "Installato": "Installed", "Installo…": "Installing…", "Installa": "Install", "di {author}": "by {author}",
  "{n} download": "{n} downloads", "Pagina {a} di {b}": "Page {a} of {b}", "Precedenti": "Previous", "Successivi": "Next",
  "Modrinth non risponde: {err}. Controlla la connessione e riprova.": "Modrinth isn't responding: {err}. Check your connection and try again.",
  "Nessun risultato per “{q}”": "No results for “{q}”", "Nessun risultato": "No results",
  "Niente di compatibile con Minecraft {mc}. Prova un nome più corto o in inglese.": "Nothing compatible with Minecraft {mc}. Try a shorter name.",
  "OptiFine non usa mod": "OptiFine doesn't use mods",
  "Le mod di Modrinth funzionano con i profili <b>Fabric</b>. Per prestazioni e shader con Fabric installa <b>Sodium</b> e <b>Iris</b>: Iris arriva da solo quando aggiungi uno shader.": "Modrinth mods work with <b>Fabric</b> profiles. For performance and shaders on Fabric, use <b>Sodium</b> and <b>Iris</b>: Iris comes along when you add a shader.",
  "Cambia caricatore": "Change loader", "Questo profilo è vanilla": "This profile is vanilla",
  "Passa il profilo a Fabric per installare mod da Modrinth con un clic: le dipendenze arrivano da sole.": "Switch the profile to Fabric to install Modrinth mods in one click: dependencies come along.",
  "Gli shader richiedono Fabric o OptiFine": "Shaders need Fabric or OptiFine",
  "Con Fabric il launcher installa Iris in automatico insieme allo shader che scegli. Con OptiFine gli shader funzionano subito.": "With Fabric the launcher installs Iris together with the shader you pick. With OptiFine, shaders work right away.",
  "Installati: {list}": "Installed: {list}", "{name} installata": "{name} installed", "{name} installato e attivato": "{name} installed and turned on", "Contenuto": "Content",
  // worlds, shots, logs
  "Ancora nessun mondo": "No worlds yet", "Mondi e server": "Worlds & servers", "Server": "Servers",
  "Nessun server salvato": "No saved servers", "Aggiungi un server dal menu Multigiocatore del gioco: comparirà qui.": "Add a server from the game's Multiplayer menu: it shows up here.",
  "Per entrare direttamente in un mondo o in un server serve Minecraft 1.20 o successivo.": "Jumping straight into a world or server needs Minecraft 1.20 or later.", "I mondi creati in questo profilo compaiono qui.": "Worlds you create in this profile show up here.",
  "Per entrare direttamente in un mondo serve Minecraft 1.20 o successivo.": "Jumping straight into a world needs Minecraft 1.20 or later.",
  "Giocato {_when}": "Played {_when}", "Entra": "Join", "Apri la cartella di {name}": "Open the folder of {name}",
  "Nessuno screenshot": "No screenshots", "Premi F2 in gioco per scattarne uno: lo trovi qui.": "Press F2 in game to take one: it shows up here.",
  "Output dell'ultima sessione di gioco": "Output of the last game session", "Copia": "Copy", "Log copiato": "Log copied",
  "Avvia il gioco per vedere qui il suo output.": "Start the game to see its output here.",
  // profile form
  "Elimina profilo": "Delete profile", "Cancella mondi, mod e configurazione di questo profilo. Non si può annullare.": "Deletes this profile's worlds, mods and settings. This can't be undone.",
  "Il tuo primo profilo": "Your first profile", "Caricatore": "Loader", "Versione di Minecraft": "Minecraft version", "Build di OptiFine": "OptiFine build",
  "Nome": "Name", "Importa da un altro launcher": "Import from another launcher", "Nessun profilo da importare": "Nothing to import",
  "Non ho trovato istanze di Modrinth App, CurseForge o Prism Launcher su questo computer.": "No Modrinth App, CurseForge or Prism Launcher instances on this computer.",
  "Torna al nuovo profilo": "Back to the new profile", "{n} mondi": "{n} worlds", "1 mondo": "1 world", "{n} mod": "{n} mods", "Copio…": "Copying…", "Importato": "Imported",
  "Errore": "Error", "Non supportato": "Not supported", "Blocco avvia solo profili Vanilla e Fabric.": "Blocco runs Vanilla and Fabric profiles only.",
  "Scegli i profili da copiare in Blocco: mondi, mod, texture pack, shader e impostazioni. Gli originali non vengono toccati.": "Pick the profiles to copy into Blocco: worlds, mods, texture packs, shaders and settings. The originals stay untouched.",
  "Importa {n}": "Import {n}", "1 profilo importato": "1 profile imported", "{n} profili importati": "{n} profiles imported", "Icona": "Icon", "Blocco": "Block", "Erba": "Grass", "Gemma": "Gem", "Piccone": "Pickaxe", "Spada": "Sword",
  "Albero": "Tree", "Pozione": "Potion", "Cuore": "Heart", "Casa": "House", "Fiamma": "Flame", "Es. Sopravvivenza con gli amici": "E.g. Survival with friends", "Colore": "Colour",
  "4 GB bastano per vanilla e poche mod; 6 GB con gli shader. Il computer ha {mem}.": "4 GB is enough for vanilla and a few mods; 6 GB with shaders. This computer has {mem}.",
  "Condividi comandi e impostazioni": "Share controls and settings",
  "Tasti, opzioni video e audio e lista server restano uguali in tutti i profili.": "Keys, video and sound options and the server list stay the same in every profile.",
  "Avanzate": "Advanced", "Blocco {v}": "Blocco {v}", "Aggiorna a {v}": "Update to {v}", "Installo…": "Installing…",
  "Scarico… {n}%": "Downloading… {n}%", "Aggiornato": "Up to date", "Controllo non riuscito": "Check failed", "Controllo…": "Checking…",
  "Controlla aggiornamenti": "Check for updates", "Aggiorna": "Update", "Installa Blocco {v}: l'app si riapre da sola.": "Install Blocco {v}: the app reopens on its own.",
  "Blocco controlla da solo all'avvio. L'aggiornamento si installa e l'app si riapre: profili e mondi restano dove sono.": "Blocco checks on its own at startup. The update installs and the app reopens: profiles and worlds stay where they are.",
  "Chiudi il gioco prima di aggiornare.": "Close the game before updating.",  "Java personalizzato": "Custom Java", "Automatico": "Automatic",
  "Percorso di javaw.exe. Vuoto: il launcher scarica la versione giusta.": "Path to javaw.exe. Empty: the launcher downloads the right version.",
  "Argomenti JVM": "JVM arguments", "Salva modifiche": "Save changes", "Crea profilo": "Create profile",
  "Caricamento…": "Loading…", "Elenco non disponibile": "List unavailable", "Riprova": "Retry",
  "Non riesco a scaricare l'elenco delle versioni ({err}).": "Can't download the version list ({err}).",
  "solo anteprima": "preview only", "(anteprima)": "(preview)", "Nessuna versione disponibile": "No versions available",
  "Mod, shader e texture pack da Modrinth.": "Mods, shaders and texture packs from Modrinth.",
  "optifine.net non risponde ({err}).": "optifine.net isn't responding ({err}).",
  "Prestazioni e shader. Disponibile fino alla {v}.": "Performance and shaders. Available up to {v}.",
  "Prestazioni e shader. Disponibile fino alla {v}: per la {latest} non è ancora uscito.": "Performance and shaders. Available up to {v}: not out yet for {latest}.",
  "Minecraft così com'è, con i texture pack.": "Minecraft as it is, with texture packs.",
  "Dai un nome al profilo.": "Give the profile a name.", "Scegli una versione.": "Pick a version.",
  "Versione cambiata: controlla che mod e pack siano compatibili": "Version changed: check that mods and packs are compatible",
  "Profilo salvato": "Profile saved", "Profilo creato": "Profile created", "Profilo eliminato": "Profile deleted",
  "Eliminare “{name}”?": "Delete “{name}”?",
  "Verranno cancellati mondi, mod e configurazione di questo profilo. Le impostazioni condivise e gli altri profili restano.": "This profile's worlds, mods and settings will be deleted. Shared settings and other profiles stay.",
  // settings
  "Account": "Account", "Account Microsoft": "Microsoft account", "Account offline": "Offline account",
  "Accedi con l'account Microsoft che possiede Minecraft per giocare online. Un nome offline basta per il giocatore singolo.": "Sign in with the Microsoft account that owns Minecraft to play online. An offline name is enough for singleplayer.",
  "Offline · solo giocatore singolo": "Offline · singleplayer only", "In uso": "In use", "Rimuovi account": "Remove account",
  "Accedi con Microsoft": "Sign in with Microsoft", "Nome offline": "Offline name", "Aggiungi": "Add",
  "Classico": "Classic", "Slim": "Slim", "Cambia skin": "Change skin", "Modello": "Model",
  "Accedi per vedere il tuo giocatore": "Sign in to see your player",
  "PNG 64×64. Trascina il giocatore per girarlo.": "64×64 PNG. Drag the player to turn it.",
  "Mantello": "Cape", "Mantello Microsoft": "Microsoft cape", "Nessuno": "None", "Questo account non ha mantelli.": "This account has no capes.",
  "I mantelli del tuo account Microsoft. Quello scelto si vede in gioco su tutti i server.": "Your Microsoft account's capes. The one you pick shows in game on every server.",
  "Mantello aggiornato": "Cape updated",
  "Mantello personalizzato": "Custom cape", "Crea da un'immagine": "Create from a picture", "Cambia immagine": "Change picture",
  "Un mantello tuo, che vedi solo tu in gioco. Funziona nei profili Fabric 26.x: il launcher aggiunge da solo la mod Blocco Cape.": "A cape of your own that only you see in game. Works in Fabric 26.x profiles: the launcher adds the Blocco Cape mod by itself.",
  "Già un modello di mantello: usato così com'è.": "Already a cape template: used as it is.",
  "Immagine": "Picture", "Riempi": "Fill", "Adatta": "Fit", "Bordi": "Edges", "Usa in gioco": "Use in game", "Modifica": "Edit",
  "Rimuovi il mantello personalizzato": "Remove the custom cape", "Non riesco a leggere questa immagine": "Can't read this picture",
  "Mantello salvato: lo vedrai al prossimo avvio di un profilo Fabric 26.x": "Cape saved: you'll see it the next time you start a Fabric 26.x profile",
  "Eliminare il mantello personalizzato?": "Delete the custom cape?", "Sparirà dal launcher e dal gioco. I mantelli Microsoft non cambiano.": "It goes away from the launcher and the game. Microsoft capes don't change.",
  "Elimina": "Delete", "Mantello nascosto": "Cape hidden",
  "Preferenze": "Preferences", "Scorciatoie da tastiera": "Keyboard shortcuts", "Indietro / chiudi": "Back / close",
  "Gioca o ferma": "Play or stop", "Vai al profilo": "Go to profile", "Scheda successiva": "Next tab", "Cerca su Modrinth": "Search Modrinth", "Animazioni": "Animations", "Auto": "Auto", "Sempre": "Always", "Ridotte": "Reduced",
  "Auto segue Windows, dove gli effetti di animazione ora sono disattivati.": "Auto follows Windows, where animation effects are currently off.",
  "Auto segue Windows, dove gli effetti di animazione ora sono attivi.": "Auto follows Windows, where animation effects are currently on.", "Lingua": "Language", "Memoria per i nuovi profili": "Memory for new profiles",
  "Nascondi il launcher mentre giochi": "Hide the launcher while playing", "Riappare quando chiudi il gioco.": "It comes back when you close the game.",
  "Mostra snapshot": "Show snapshots", "Versioni di prova settimanali di Mojang.": "Mojang's weekly test versions.",
  "ID client Azure": "Azure client ID",
  "Se hai un'app Azure approvata da Mojang, inserisci il suo ID client. Vuoto: si usa l'ID pubblico del launcher di Minecraft.": "If you have an Azure app approved by Mojang, enter its client ID. Empty: the public Minecraft launcher ID is used.",
  "Salva": "Save", "Cartella dati": "Data folder", "Apri": "Open",
  "Versioni, Java, profili e impostazioni condivise (cartella shared) sono in {path}.": "Versions, Java, profiles and shared settings (shared folder) live in {path}.",
  "ID client salvato": "Client ID saved",
  "Apri <b>{uri}</b> nel browser.": "Open <b>{uri}</b> in your browser.",
  "Inserisci questo codice e accedi con il tuo account Microsoft.": "Enter this code and sign in with your Microsoft account.",
  "Apri la pagina": "Open the page", "Copia codice": "Copy code", "Codice copiato": "Code copied", "In attesa dell'accesso…": "Waiting for sign-in…",
  "Rimuovere {name}?": "Remove {name}?", "L'account verrà disconnesso da questo launcher. Potrai accedere di nuovo quando vuoi.": "The account will be signed out of this launcher. You can sign in again any time.",
  "Il nome offline usa 3–16 lettere, numeri o _": "Offline names use 3–16 letters, digits or _",
  "Account {name} aggiunto": "Account {name} added", "Accesso effettuato come {name}": "Signed in as {name}",
  "Il codice è scaduto. Riprova.": "The code expired. Try again.", "Skin aggiornata: la vedrai anche in gioco": "Skin updated: you'll see it in game too",
  // play
  "Aggiungi un account prima di giocare": "Add an account before playing",
  "Il gioco si è chiuso: {reason}": "The game closed: {reason}",
  "Il gioco si è chiuso con un errore (codice {code}). Guarda il Log.": "The game closed with an error (code {code}). Check the Log.",
  "Altre informazioni": "More information",
};

let LANG = "it";
const setLang = (l) => { LANG = l === "en" ? "en" : "it"; document.documentElement.lang = LANG; };

const escHtml = (v) => String(v ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));

/** Translated HTML: placeholder values are escaped. */
function t(key, vars = {}) {
  const s = LANG === "en" ? EN[key] ?? key : key;
  return s.replace(/\{(\w+)\}/g, (_, k) => (k.startsWith("_") ? String(vars[k] ?? "") : escHtml(vars[k])));
}

/** Translated plain text (for toasts, titles and attributes that escape on their own). */
function tp(key, vars = {}) {
  const s = LANG === "en" ? EN[key] ?? key : key;
  return s.replace(/\{(\w+)\}/g, (_, k) => String(vars[k] ?? ""));
}
