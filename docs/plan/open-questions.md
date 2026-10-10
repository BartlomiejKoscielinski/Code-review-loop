# Pytania otwarte i rozbieżności z materiałami wejściowymi

## Rozbieżności z `docs/input/plan.md` do poprawienia przez człowieka

`docs/input/` jest tylko do odczytu dla agenta, dlatego te punkty wymagają ręcznej zmiany albo decyzji.

| # | Miejsce | Rozbieżność | Propozycja |
| --- | --- | --- | --- |
| R1 | „Ryzyka”, wiersz „Hooki i MCP z gałęzi MR” | Mitygacja wymienia `--bare`, a wybrana ścieżka (seaty) wyklucza `--bare`, bo ten tryb nie czyta loginu OAuth ([headless](https://code.claude.com/docs/en/headless)). | Zastąpić odwołaniem do ADR z zadania P3-01. |
| R2 | „Warstwa AI”, lista wymagań izolacji | Zakłada, że kod MR przez `--add-dir` jest bezpieczny, jeśli nie wczytują się hooki i `.mcp.json`. Z `--add-dir` wczytują się jednak skille, komendy i subagenci z `.claude/` ([permissions](https://code.claude.com/docs/en/permissions)). | Plan (P3-01) podaje migawkę plików bez `.claude/`, `CLAUDE.md` i `.mcp.json`. Uzupełnić materiał. |
| R3 | „Build vs buy” i „Plan faz” | Faza 0 i „pięć faz” na diagramie, którego nie ma. | Decyzja w [ADR 0004](../adr/0004-skip-build-vs-buy-phase.md): cztery fazy, bez fazy 0. |
| R4 | „Integracja z Git i GitLab”, tabela endpointów | `scope=reviews_for_me` istnieje dopiero od GitLaba 18.4, a notatka i `reviewer_state` w `bulk_publish` od 19.2. | Plan ma warianty zastępcze (P2-01, P2-21). Dopisać wymagane wersje. |
| R5 | „Warstwa AI”, „Wywołanie” | `--allowedTools` nie ogranicza zestawu narzędzi, tylko zatwierdza je bez pytania. Ograniczenie daje `--tools` ([CLI reference](https://code.claude.com/docs/en/cli-reference)). | Doprecyzować w materiale. Plan używa `--tools`. |
| R6 | Makiety | Brak ekranu wyboru źródła (lokalne zmiany, commit, branch). | Opis ekranu „Otwórz” w P1-16. Ewentualnie dorysować makietę. |
| R7 | „Warstwa AI”, „Wybrana ścieżka” | Wymaganie: katalog roboczy sesji to zaufany folder narzędzia „z konfiguracją z gałęzi docelowej”. Plan daje pusty katalog roboczy bez konfiguracji Claude, a prompty z `.review/` gałęzi docelowej przekazuje w treści wywołania (P3-01, P3-03). | Zatwierdzić w ADR z P3-01 albo wskazać, jaka konfiguracja z gałęzi docelowej ma być w katalogu sesji. |
| R8 | „Integracja z Git i GitLab”, „Worktree per MR” | `git worktree add` współdzieli `.git` z repo dewelopera, więc kod MR uruchomiony w worktree może zapisać hooki lub `core.hooksPath` i wykonać się później poza izolacją. | ADR z P2-30 rozstrzyga: worktree z ochroną, osobny klon albo `git archive`. |

## Pytania do zespołu

| # | Pytanie | Kto odpowiada | Blokuje |
| --- | --- | --- | --- |
| Q1 | Jaka jest wersja GitLaba? Spike S3 odczyta ją z `GET /api/v4/version`. Poniżej 16.3 nie ma draftów z pozycją na liniach, co zmienia przebieg publikacji w fazie 2. | S3 (człowiek z tokenem) | P1-14b, faza 2 |
| Q2 | Czy użycie seatów Claude Code przez narzędzie wewnętrzne jest zgodne z umową Enterprise i jakie są wymogi retencji danych przy wysyłaniu kodu do Claude? | admin Claude Enterprise | P3-01 |
| Q3 | Czy CI repo MAUI może publikować SARIF jako artefakt? Jeśli tak, różnica ostrzeżeń nie wymaga lokalnego buildu kodu MR. | zespół, właściciel CI | P2-30 |
| Q4 | Skąd Lupa ma pobierać aktualizacje (np. pakiety w GitLabie, udział sieciowy)? Kto przechowuje klucz podpisu? | zespół | P2-34 |
| Q5 | Czy admin GitLaba zarejestruje aplikację OAuth? Do tego czasu Lupa używa Personal Access Token. | admin GitLaba | P4-06 |
| Q6 | Czy progi bramek faz 3 i 4 (precyzja ≥ 0,6, recall ≥ 0,5, akceptacja ≥ 40%, limit kosztu na MR) są akceptowalne? | zespół | start fazy 3 |
| Q7 | Która maszyna jest referencyjna dla budżetów wydajności (CPU, RAM, dysk)? | zespół | bramka fazy 1 |
| Q8 | Jaki minimalny zakres tokenu przyjmujemy w fazie 1 (`read_api`), a jaki od fazy 2 (`api`)? Propozycja planu: `read_api` w fazie 1, zmiana na `api` przy pierwszym zapisie. | zespół, `security-reviewer` | P1-14a |
| Q9 | Repo Lupy jest dziś na GitHubie, więc CI to GitHub Actions z runnerem Windows (P1-02). Czy tak zostaje, czy repo przeniesiemy do GitLaba z własnymi runnerami? | @Bartek | P1-02 |
| Q10 | Gdzie i pod jaką etykietą zespół rejestruje błędy po merge? Od tego zależy metryka w P1-30, P4-03 i bramka fazy 4. | zespół | P1-30 |
| Q11 | Czy na maszynach deweloperów można wymagać konfiguracji `~/.claude` bez hooków, pluginów i MCP, jeśli spike P3-01 pokaże, że flagi jej nie wyłączają? | zespół, `security-reviewer` | P3-01 |
| Q12 | Kto przygotuje zestaw 20 MR z MAUI z wstawionymi błędami (P3-22) i kto da Claude dostęp do repo MAUI dla S2, P1-15 i bramek? | zespół | S2, P1-15, P3-22 |
| Q13 | Jak dystrybuować konfigurację organizacji (P2-37): razem z instalatorem, przez politykę systemową czy z udziału sieciowego? | zespół | P2-37 |
