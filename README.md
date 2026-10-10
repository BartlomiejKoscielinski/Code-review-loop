# Lupa

Lupa (nazwa robocza) to lokalna aplikacja desktop do prowadzonego code review merge requestów z GitLaba self-managed. Rdzeń działa w Tauri 2 (Rust), interfejs w React + TypeScript z CodeMirror 6, analizę C# robi sidecar .NET z Roslyn, a asystentem jest Claude Code wywoływany przez `claude -p` w trybie tylko do odczytu. Na start tylko Windows. Dostęp do Claude wyłącznie przez seaty Claude Code, bez API.

**Bieżąca faza: 0, planowanie.** Repo zawiera tylko fundament pracy z Claude Code. Kod aplikacji, strukturę katalogów i komendy build ustali sesja planowania na podstawie `docs/input/`.

## Mapa repo

| Ścieżka | Zawartość |
|---|---|
| `CLAUDE.md` | instrukcje projektu dla Claude Code, ładowane w każdej sesji |
| `docs/input/` | wymagania (`plan.md`) i makiety (`mocks/`) od zespołu; agent tylko czyta |
| `docs/plan/` | plan implementacji (wypełni sesja planowania) |
| `docs/adr/` | decyzje architektoniczne; szablon `0000-template.md` |
| `spikes/` | ograniczone czasowo eksperymenty przed decyzją |
| `.claude/settings.json` | wspólne uprawnienia, hooki i włączone pluginy |
| `.claude/rules/` | reguły jakości per język, ładowane tylko przy pracy z pasującymi plikami |
| `.claude/skills/` | `/adr`, `/spike`, `/self-review`; uruchamia je człowiek |
| `.claude/agents/` | subagenci tylko do odczytu: `fact-checker`, `plan-critic`, `code-reviewer`, `security-reviewer` |
| `.claude/hooks/` | hooki Node: `protect-inputs`, `guard-tests`, `guard-delete`, `format-on-edit` oraz ich testy |

Repozytorium kodu jest obecnie na GitHubie (`origin`). GitLab self-managed to backend produktu, z którego Lupa pobiera merge requesty.

## Co instaluje każdy członek zespołu

### 1. Narzędzia

| Narzędzie | Po co | Instalacja |
|---|---|---|
| Claude Code | asystent; sprawdź: `claude --version` | https://code.claude.com/docs/en/setup |
| Git | repo | https://git-scm.com/download/win |
| Node.js LTS | hooki Claude Code, UI | https://nodejs.org |
| Rust (rustup) | rdzeń Tauri, `rustfmt` | https://rustup.rs |
| .NET 10 SDK | sidecar, `csharp-ls`, `dotnet format` | `winget install Microsoft.DotNet.SDK.10` ([źródło](https://dotnet.microsoft.com/download/dotnet-core/current)) |

### 2. Serwery LSP

Pluginy LSP nie zawierają serwerów, tylko je uruchamiają. Binarki muszą być na `PATH` powłoki, z której startujesz `claude` ([docs](https://code.claude.com/docs/en/plugins/code-intelligence)).

```powershell
rustup component add rust-analyzer
npm install -g typescript-language-server typescript
dotnet tool install --global csharp-ls   # wymaga .NET 10 SDK
```

Sprawdź: `rust-analyzer --version`, `typescript-language-server --version`, `csharp-ls --version`. Samo `Get-Command rust-analyzer` nie wystarczy, bo rustup instaluje proxy także bez komponentu.

### 3. Pluginy Claude Code

`.claude/settings.json` włącza dla całego zespołu trzy pluginy z oficjalnego marketplace'u `claude-plugins-official`. Wpis w ustawieniach nie pobiera pluginu, więc każdy instaluje go raz u siebie ([docs](https://code.claude.com/docs/en/plugins/install)):

```powershell
claude plugin install rust-analyzer-lsp@claude-plugins-official --scope project
claude plugin install typescript-lsp@claude-plugins-official --scope project
claude plugin install csharp-lsp@claude-plugins-official --scope project
```

Jeśli pojawi się błąd `Marketplace "claude-plugins-official" not found`, najpierw uruchom `claude plugin marketplace add anthropics/claude-plugins-official`. Działanie sprawdzisz w `/plugin`: zakładka **Errors** nie powinna zgłaszać `Executable not found in $PATH`.

Pluginy, których świadomie nie instalujemy:

| Plugin | Powód |
|---|---|
| `code-review` | działa tylko z GitHubem (`gh pr`) i dubluje wbudowane `/code-review` |
| `feature-dev` | dubluje wbudowanych agentów Explore i Plan oraz nasz proces plan → ADR |
| `security-guidance` | wymaga bash i Pythona 3.10+, instaluje pakiety przez pip i wysyła diff do modelu przy każdym Stop; wrócimy do niego w fazie implementacji |
| `example-skills` | 12 skilli naraz (~1,2 tys. tokenów w każdej sesji), z czego 10 nie dotyczy projektu |
| `gitlab` (MCP) | adres `gitlab.com` wpisany na sztywno, nie działa z self-managed |
| `context7` (MCP) | zewnętrzny serwis; dokumentację sprawdza `fact-checker` w oficjalnych źródłach |

### 4. Weryfikacja hooków

```powershell
node --test .claude/hooks/hooks.test.mjs
```

Hooki przetestowano na Linuksie, z symulacją ścieżek Windows. Przy pierwszym uruchomieniu na Windows puść testy i zgłoś błędy. Gałąź `dotnet format` w `format-on-edit` nie została uruchomiona, bo w środowisku bootstrapu nie było .NET SDK. Sprawdzi ją pierwsza osoba, która doda projekt `.csproj`.

### 5. Ustawienia osobiste

Twoje osobiste ustawienia trafiają do `CLAUDE.local.md` i `.claude/settings.local.json`. Oba pliki są w `.gitignore`. Nie wpisuj tokenów do żadnego pliku w repo; serwery MCP i narzędzia czytają je ze zmiennych środowiskowych (`${VAR}`).
