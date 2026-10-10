# Decyzje domyślne

Plan nie czeka na odpowiedzi. Poniższe sprawy rozstrzygnąłem rozsądnymi wartościami domyślnymi. Każdą da się zmienić później: wystarczy powiedzieć, a zmiana trafi do planu.

| # | Sprawa | Decyzja | Gdzie w planie |
| --- | --- | --- | --- |
| D1 | Wersja GitLaba | Lupa sama odczytuje ją przy starcie (`GET /api/v4/version`) i włącza tylko funkcje obsługiwane przez instancję. Starsze wersje dostają warianty zastępcze. Nie trzeba jej znać wcześniej. | P1-14b, P2-01, P2-17, P2-21 |
| D2 | Dostęp do Claude | Lupa używa logowania do Claude Code, które każdy w zespole już ma (licencja na osobę w Claude Enterprise, tzw. seat). Bez osobnego klucza API. Przed fazą 3 warto upewnić się u admina Claude, że narzędzie wewnętrzne może z tego korzystać. Do fazy 3 nic to nie blokuje. | P3-01 |
| D3 | Ostrzeżenia z buildu | Lupa buduje kod lokalnie w izolacji (P2-30). Pobieranie raportów z CI można dodać później. | P2-30, P2-32 |
| D4 | Aktualizacje | Do fazy 2 instalator udostępniany ręcznie. Auto-update dochodzi w P2-34 z kanałem wybranym wtedy. | P1-29, P2-34 |
| D5 | Logowanie do GitLaba | Personal Access Token wklejony raz w ustawieniach Lupy, trzymany w Menedżerze poświadczeń Windows. Na fazę 1 wystarczy zakres `read_api`, od fazy 2 potrzebny `api`. OAuth tylko, jeśli kiedyś będzie potrzebny. | P1-14a, P4-06 |
| D6 | Progi jakości AI w bramce fazy 3 | Przyjmujemy wartości z planu i korygujemy je po pierwszym pomiarze. | faza 3 |
| D7 | Maszyna do pomiarów wydajności | Twój komputer służbowy. | P1-28 |
| D8 | CI | GitHub Actions z runnerem Windows, bo tam jest repo Lupy. | P1-02 |
| D9 | Błędy po merge w metrykach | Metryka opcjonalna. Jeśli zespół nie oznacza błędów w GitLabie etykietą, Lupa ją pomija. | P1-30, P4-04 |
| D10 | Osobista konfiguracja Claude Code (`~/.claude`) | Lupa wykrywa hooki, pluginy i serwery MCP, które mogłyby działać w jej sesjach. Jeśli są, wyłącza je flagami albo ostrzega i nie uruchamia AI. Szczegóły ustali spike P3-01. | P3-01 |
| D11 | Zestaw ewaluacyjny AI | Claude buduje go z historii repo MAUI: commity, które naprawiały błędy, dają gotowe przypadki „czy AI zauważyłoby ten błąd”. Od Ciebie tylko dostęp do repo. | P3-22 |
| D12 | Konfiguracja dla całego zespołu | Plik konfiguracji (URL GitLaba itp.) leży obok instalatora. | P2-37 |

## Czego naprawdę potrzebuję od Ciebie

Tylko dwóch rzeczy, i dopiero gdy aplikacja da się uruchomić:

1. Wkleić token GitLaba w ustawieniach Lupy na swoim komputerze.
2. Otworzyć w Lupie prawdziwy MR i powiedzieć, czy diff wygląda dobrze.

## Różnice względem `docs/input/plan.md` (dla porządku)

Plan w kilku miejscach odbiega od materiału wejściowego, bo weryfikacja dokumentacji pokazała inne fakty. Nic nie trzeba z tym robić. Jeśli chcesz, możesz kiedyś zaktualizować `docs/input/plan.md`, którego agent nie edytuje.

- Bez fazy 0 i z czterema fazami zamiast pięciu ([ADR 0004](../adr/0004-skip-build-vs-buy-phase.md)).
- Tryb `--bare` nie działa z logowaniem z seatów, więc izolację Claude zapewniają inne flagi i kopia plików MR bez konfiguracji Claude (P3-01, P3-21).
- `--allowedTools` nie ogranicza narzędzi, robi to `--tools`.
- Część endpointów GitLaba wymaga nowszych wersji (D1).
- Kopia kodu do buildu to niekoniecznie `git worktree`, bo worktree współdzieli `.git` z repo dewelopera (P2-30).
- Ekran wyboru źródła zmian („Otwórz”) nie ma makiety. Jego opis jest w P1-16.
