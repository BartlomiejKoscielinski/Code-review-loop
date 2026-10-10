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

## Pytania do zespołu

| # | Pytanie | Kto odpowiada | Blokuje |
| --- | --- | --- | --- |
| Q1 | Jaka jest wersja GitLaba? Spike S3 odczyta ją z `GET /api/v4/version`. Poniżej 16.3 nie ma draftów z pozycją na liniach, co zmienia przebieg publikacji w fazie 2. | S3 (człowiek z tokenem) | P1-14, faza 2 |
| Q2 | Czy użycie seatów Claude Code przez narzędzie wewnętrzne jest zgodne z umową Enterprise i jakie są wymogi retencji danych przy wysyłaniu kodu do Claude? | admin Claude Enterprise | P3-01 |
| Q3 | Czy CI repo MAUI może publikować SARIF jako artefakt? Jeśli tak, różnica ostrzeżeń nie wymaga lokalnego buildu kodu MR. | zespół, właściciel CI | P2-30 |
| Q4 | Skąd Lupa ma pobierać aktualizacje (np. pakiety w GitLabie, udział sieciowy)? Kto przechowuje klucz podpisu? | zespół | P2-34 |
| Q5 | Czy admin GitLaba zarejestruje aplikację OAuth? Do tego czasu Lupa używa Personal Access Token. | admin GitLaba | P4-06 |
| Q6 | Czy progi bramek faz 3 i 4 (precyzja ≥ 0,6, recall ≥ 0,5, akceptacja ≥ 40%, limit kosztu na MR) są akceptowalne? | zespół | start fazy 3 |
| Q7 | Która maszyna jest referencyjna dla budżetów wydajności (CPU, RAM, dysk)? | zespół | bramka fazy 1 |
| Q8 | Jaki minimalny zakres tokenu przyjmujemy w fazie 1 (`read_api`), a jaki od fazy 2 (`api`)? Propozycja planu: `read_api` w fazie 1, zmiana na `api` przy pierwszym zapisie. | zespół, `security-reviewer` | P1-14 |
