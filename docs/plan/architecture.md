# Architektura Lupy

Dokument opisuje moduły, granice zaufania, model danych, strategię testów i komendy. Wynika z `docs/input/plan.md` oraz z ADR [0002](../adr/0002-architecture-and-module-boundaries.md) i [0003](../adr/0003-review-target-model.md). Zadania, które to budują, są w plikach `phase-*.md`.

## Zasada podziału

**Rust liczy fakty i trzyma sekrety, TypeScript układa z faktów decyzje i je pokazuje, .NET rozumie C#.**

- Rdzeń w Ruście (`crates/`, `app/src-tauri/`) ma całe I/O: Git, diff, cache, SQLite, HTTP do GitLaba, token w magazynie systemowym, uruchamianie procesów (`git`, `claude -p`, sidecar, build). Interfejs nigdy nie czeka na sieć, bo rdzeń odpowiada z cache i dosyła wyniki zdarzeniami.
- Logika domenowa w TypeScripcie (`packages/domain/`) to czyste funkcje bez I/O: algorytm trasy, reguły bramki, budżet LOC i czasu. Łatwo je testować i są takie same dla każdego w zespole.
- Interfejs (`app/src/`) to React + CodeMirror 6. Nie ma dostępu do sieci ani do sekretów, rozmawia wyłącznie z rdzeniem przez IPC Tauri.
- Sidecar .NET (`sidecar/`, od fazy 3) robi semantykę C# z Roslyn. Nie ma tokenu ani dostępu do GitLaba. Uruchamia go rdzeń z API Rusta, bez capability dla webview. Gdy ładuje projekty przez MSBuild, działa w strefie D, a jego wyniki są niezaufane.

Ten podział realizuje mitygację z `docs/input/plan.md` („Rust tylko dla Gita, diffa i IPC; logika domenowa w TS i .NET”), z jednym doprecyzowaniem: klient GitLaba i uruchamianie procesów też są w Ruście, bo tam jest token i tam musi być granica bezpieczeństwa.

## Układ repo

```text
Cargo.toml                 # workspace Rust
package.json               # npm workspaces: app, packages/*
app/
  src/                     # UI: React + TypeScript + CodeMirror 6 (Vite)
  src-tauri/               # crate lupa-app: okno, komendy IPC, capabilities, składanie modułów
crates/
  lupa-git/                # odczyt repo (gix), fetch i worktree przez `git` CLI
  lupa-diff/               # diff linii (histogram), diff słów, klasyfikacja plików, przeniesienia
  lupa-syntax/             # Tree-sitter: symbole hunków, dopasowania reguł, krawędzie grafu (faza 2)
  lupa-gitlab/             # klient REST v4, wykrywanie możliwości instancji
  lupa-secrets/            # token w Windows Credential Manager
  lupa-store/              # SQLite: migracje, sesje, viewed, cache
  lupa-review/             # cel review (ADR 0003), sesje, kolejka zadań w tle
  lupa-rules/              # `.review/` z gałęzi docelowej: schemat, walidacja (faza 2)
  lupa-exec/               # uruchamianie kodu MR w izolacji (faza 2, ADR z P2-30)
  lupa-ai/                 # abstrakcja dostawcy, runner `claude -p`, cache wyników (faza 3)
  lupa-testkit/            # budowanie repozytoriów testowych, atrapy GitLaba i `claude`
packages/
  domain/                  # trasa, bramka, budżet: czyste funkcje TS
  ipc/                     # typy IPC generowane z Rusta (nie edytujemy ręcznie)
sidecar/                   # faza 3
  Lupa.Sidecar/            # .NET 10, Roslyn, protokół po stdio
  Lupa.Sidecar.Tests/
bench/                     # pomiary budżetów wydajności
```

Nazwy crate'ów i pakietów są wiążące dla planu. Zmiana wymaga aktualizacji tego pliku w tej samej zmianie.

## Strefy zaufania

| Strefa | Co w niej działa | Co ma | Czego nie ma nigdy |
| --- | --- | --- | --- |
| A. Rdzeń | proces Tauri (Rust) | token GitLaba (w pamięci, na czas wywołania), SQLite, dostęp do repo dewelopera | wykonywania kodu z MR |
| B. Webview | UI React | dane do wyświetlenia | tokenu, sieci (CSP), HTML z treści MR |
| C. Claude | `claude -p` jako proces potomny | pusty katalog roboczy narzędzia, migawka wybranych plików MR bez `.claude/`, `CLAUDE.md` i `.mcp.json` (przez `--add-dir`), narzędzia `Read`/`Grep`/`Glob`; szczegóły w ADR z zadania P3-01 | tokenu w środowisku, zapisu, powłoki, MCP, hooków, skilli i konfiguracji z gałęzi MR |
| D. Wykonanie | build, testy, Stryker, sidecar ładujący projekty przez MSBuild, emulator | kopia kodu w katalogu cache (sposób kopiowania ustala ADR z P2-30: worktree współdzieli `.git` z repo dewelopera, więc może być wykluczony), limit czasu | tokenu w środowisku, dostępu do repo dewelopera; szczegóły w ADR z zadania P2-30 |

Dane płynące ze stref C i D (odpowiedzi modelu, SARIF, logi testów) są niezaufane tak samo jak treść MR. Rdzeń je waliduje, a UI pokazuje jako tekst.

## Model danych (rdzeń)

Pełny opis celu review jest w [ADR 0003](../adr/0003-review-target-model.md). Skrót:

```text
ReviewTarget { kind: WorkingTree | Commit | Range | MergeRequest,
               repo, base, head, start?, previous?, configRef, sink }
ChangedFile  { oldPath?, newPath?, oldBlob?, newBlob?, status, class: code | generated | binary | formatting | moved }
Hunk         { file, oldRange, newRange, lines, wordDiff, symbol? }
Unit         { id, symbol, hunks[] }                      # faza 2
Edge         { from, to, kind: def-use | test | xaml | di | same-file | co-change, strength }
Route        { threads[ { steps[ { id, title, reason, units[], estimateMin } ] } ], passes[] }
Draft        { target, position, body, label, blocking, origin: human | ai, syncState }
Finding      { step, lines, label, blocking, confidence, rationale, draft, evidence, state }  # faza 3
```

Wszystko, co da się policzyć z obiektów Gita, jest kluczowane identyfikatorem bloba lub commita. Bloby są niezmienne, więc ten cache nie wymaga unieważniania. Zmiany niezacommitowane dostają identyfikator z treści pliku (patrz ADR 0003).

## Przepływ: od wyboru celu do publikacji

1. UI wywołuje `open_target(spec)`. Rdzeń rozwiązuje refy, zwraca listę plików z cache albo zaczyna liczyć i wysyła postęp zdarzeniami.
2. Diffy plików liczą się leniwie: najpierw bieżący plik, potem kolejne w tle.
3. Faza 2: rdzeń dokłada symbole, krawędzie i dopasowania reguł, a `packages/domain` układa trasę i bramkę.
4. Faza 3: rdzeń zleca briefing i findingi `claude -p`, waliduje wynik i dosyła go do UI.
5. Publikacja: rdzeń wysyła drafty do GitLaba (cel MR) albo zapisuje notatki lokalne (pozostałe cele).

## IPC

- Komendy Tauri zwracają `Result<T, IpcError>`. `IpcError` ma kod i komunikat z kontekstem (co, na jakim pliku lub MR), z którego konstruktor usuwa wartości wyglądające na tokeny (reguła `.claude/rules/rust.md`).
- Każda komenda jest na liście w `app/src-tauri/build.rs` (generuje uprawnienie `allow-<komenda>`) i w capability okna. Test pilnuje, że capability daje tylko uprawnienia komend aplikacji.
- Typy TypeScript w `packages/ipc` są generowane z Rusta. Narzędzie wybiera zadanie P1-03. CI sprawdza, że wygenerowane typy są aktualne.
- Długie operacje mają identyfikator zadania, zdarzenia postępu i komendę anulowania (od P1-15).
- UI waliduje odpowiedzi na granicy (reguła `.claude/rules/typescript.md`).

## Zależności (kandydaci)

Reguły repo wymagają, żeby każda nowa zależność wynikała z planu. Poniższa lista to zgoda planu na tych kandydatów. Zadanie, które dodaje zależność, sprawdza jej aktualną wersję, licencję i utrzymanie, a wybór między kandydatami zapisuje w opisie zmiany.

| Moduł | Kandydaci | Zadanie |
| --- | --- | --- |
| powłoka | `tauri` 2, `tauri-build` 2, `@tauri-apps/cli` (P1-01); `@tauri-apps/api` (P1-03) | P1-01, P1-03 |
| Git | [`gix`](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md); [`git2`](https://crates.io/crates/git2) (wymaga libgit2) jako zapas po spike'u S2 | P1-06 |
| diff | histogram z [`imara-diff`](https://github.com/pascalkuthe/imara-diff); `gix` ma własną kopię `gix-imara-diff`, więc S2 rozstrzyga, której używamy | P1-08 |
| składnia | `tree-sitter`, gramatyki C# i XML | P1-22 |
| SQLite | `rusqlite` z migracjami w kodzie | P1-04 |
| sekrety | [`keyring`](https://crates.io/crates/keyring) 4.x (Credential Manager przez `windows-native-keyring-store`) | P1-14a |
| HTTP | `reqwest` (rustls, firmowe CA dodawane, nie wyłączane) | P1-14b |
| async, serializacja, błędy | `tokio`, `serde`, `serde_json`, `thiserror` | P1-03 (`serde_json` jako dev-dependency już w P1-01) |
| typy IPC | `specta` z `tauri-specta` albo `ts-rs` | P1-03 |
| testy Rust | atrapa HTTP (`wiremock` albo `httpmock`), `tempfile`, `insta` | P1-06, P1-14b |
| YAML | parser YAML dla `.review/` wybrany w zadaniu (`serde_yaml` nie jest już rozwijany) | P2-04 |
| UI | `react`, `react-dom`, `vite`, `@codemirror/*`, `@codemirror/merge`, wirtualizacja listy (`@tanstack/react-virtual`) | P1-17a, P1-18a, P1-21 |
| walidacja na granicy | `zod` | P1-03 |
| Markdown bez HTML | `react-markdown` bez wtyczek przepuszczających surowy HTML | P1-27 |
| testy TS | `vitest`, `jsdom`, `@testing-library/react`, `@testing-library/jest-dom` (P1-01); `@playwright/test` (P1-17b, P1-28) | P1-01, P1-28 |
| narzędzia TS | `typescript` 6.0 (typescript-eslint 8 nie wspiera jeszcze 7.x), `@vitejs/plugin-react`, `eslint`, `@eslint/js`, `typescript-eslint`, `eslint-plugin-react-hooks`, `globals`, `prettier` | P1-01 |
| .NET | `Microsoft.CodeAnalysis.CSharp.Workspaces`, `Microsoft.CodeAnalysis.Workspaces.MSBuild`, `Microsoft.Build.Locator`, xUnit | P3-11 |

Stan wiedzy o `serde_yaml` (repozytorium zarchiwizowane przez autora) jest **NIEZWERYFIKOWANY** w dokumentacji. Zadanie P2-04 sprawdza go przed wyborem parsera.

## Strategia testów

Zasady z `.claude/rules/tests.md` obowiązują wszędzie: testy deterministyczne, bez sieci, bez prawdziwego GitLaba i bez prawdziwego `claude -p`.

| Poziom | Narzędzie | Co sprawdza |
| --- | --- | --- |
| Rust jednostkowe i integracyjne | `cargo test` + `lupa-testkit` | Git, diff, pozycje komentarzy, klient GitLaba na atrapie HTTP, runner `claude` na atrapie procesu |
| Zgodność z Gitem | testy porównujące z `git diff --histogram` na repozytoriach z `lupa-testkit` | że nasz diff jest identyczny z Gitem i GitLabem |
| TS domena | `vitest` | trasa, bramka, budżet; testy właściwości (ta sama zmiana w innej kolejności wejścia daje tę samą trasę) |
| UI komponenty | `vitest` + Testing Library | stany ekranów z makiet |
| UI przepływy | Playwright na buildzie Vite z atrapą IPC | skrzynka → briefing → trasa → bramka → publikacja |
| Wydajność | `bench/`: Playwright + `performance.mark` dla UI, pomiary w Ruście dla rdzenia, a start i WebView2 w prawdziwej aplikacji według metody ze spike'a S1 | budżety z `docs/input/plan.md` |
| .NET | `dotnet test` (xUnit) | sidecar na projektach testowych |
| Bezpieczeństwo | testy z wrogimi danymi w `lupa-testkit` | XSS w opisie, path traversal w nazwach plików, prompt injection, ogromne pliki |
| Ewaluacja AI | `lupa-eval` (ręcznie, z prawdziwym `claude`) | precyzja i recall promptów, poza CI |

`lupa-testkit` buduje repozytoria testowe skryptem przy uruchomieniu testu. Gotowych repozytoriów binarnych nie commitujemy.

## Komendy (do wpisania w `CLAUDE.md` przez P1-01)

| Ekosystem | Build | Testy | Lint i format |
| --- | --- | --- | --- |
| Rust | `cargo build --workspace` | `cargo test --workspace` | `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` |
| TypeScript | `npm run build` | `npm test`, `npm run test:e2e` | `npm run lint`, `npm run typecheck` |
| Aplikacja | `npm run app:build` (skrypt opakowujący `tauri build`) | `npm run bench`, `npm run check:ipc` | — |
| Narzędzia | — | `cargo run -p lupa-ai --bin lupa-eval` (ręcznie, z prawdziwym `claude`), `cargo run -p lupa-rules --bin lupa-rules-check` | — |
| .NET (od P3-11, wzorce do `allow` proponuje zadanie, dodaje człowiek) | `dotnet build sidecar` | `dotnet test sidecar` | `dotnet format --verify-no-changes sidecar` |
| Hooki | — | `node --test .claude/hooks/hooks.test.mjs` | — |

P1-01 wpisuje komendy do `CLAUDE.md` i **proponuje** w opisie zmiany wzorce do listy `allow` (`npm run lint *`, `npm run typecheck *`, `npm run test:e2e *`, `npm run bench *`, `npm run check:ipc *`, `npm run app:build *`). Uprawnienia w `.claude/settings.json` zmienia człowiek, bo agent nie rozszerza sobie uprawnień sam. Nie dodajemy ogólnego `npm run tauri *`, bo obejmowałby `tauri add`, czyli instalację pakietów z pominięciem reguł `ask`.

## Wydajność: gdzie są budżety

| Budżet z `docs/input/plan.md` | Odpowiada za niego | Mierzy |
| --- | --- | --- |
| start do skrzynki z cache < 1 s | leniwe ładowanie UI, odczyt SQLite przed siecią | P1-28 |
| otwarcie MR z cache < 300 ms | cache list plików i diffów po SHA, diff pierwszego pliku przed resztą | P1-28 |
| następny hunk < 16 ms | nawigacja po gotowym modelu hunków, bez IPC w ścieżce klawisza | P1-28 |
| 10 000 linii, 60 fps | wirtualny viewport CodeMirror, dekoracje liczone poza wątkiem UI | S1, P1-28 |
| AI strumieniowo | `stream-json`, zdarzenia IPC, brak modali | P3-02 |

## Dostępność i czytelność

Od fazy 1: obsługa całości z klawiatury, widoczny fokus, kontrast WCAG AA w obu motywach, paleta diffa dla daltonistów, tryby gęstości i rozmiar fontu. Font JetBrains Mono jest dołączony do aplikacji. Faza 4 ma audyt (P4-07).
