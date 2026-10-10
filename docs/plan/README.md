# Plan implementacji Lupy

Plan przekłada `docs/input/plan.md` i makiety z `docs/input/mocks/` na cztery fazy z zadaniami do ok. 400 LOC. Każde zadanie ma kryteria akceptacji, a każda faza bramkę z mierzalnym warunkiem. Aplikację implementuje Claude Code, zadanie po zadaniu, w kolejności zależności.

## Pliki

| Plik | Zawartość |
| --- | --- |
| [`architecture.md`](architecture.md) | moduły, układ repo, strefy zaufania, model danych, IPC, zależności, testy, komendy |
| [`phase-1.md`](phase-1.md) | czytnik zmian: cztery źródła, diff obok siebie, klawiatura, budżety wydajności |
| [`phase-2.md`](phase-2.md) | całe review: skrzynka, reguły zespołu, trasa, komentarze, bramka, publikacja, interdiff, różnica ostrzeżeń |
| [`phase-3.md`](phase-3.md) | AI (Claude Code) i semantyka (Roslyn), dowody z wykonania |
| [`phase-4.md`](phase-4.md) | emulator, tryb nauki, pętla uczenia, metryki, audyty przed 1.0 |
| [`open-questions.md`](open-questions.md) | rozbieżności z materiałami wejściowymi i pytania do zespołu |

Decyzje: [ADR 0002](../adr/0002-architecture-and-module-boundaries.md) (architektura), [ADR 0003](../adr/0003-review-target-model.md) (model celu review), [ADR 0004](../adr/0004-skip-build-vs-buy-phase.md) (bez fazy 0).

## Fazy w skrócie

| Faza | Wynik dla zespołu | Zadania | Spike'i i ADR | Bramka (skrót) |
| --- | --- | --- | --- | --- |
| 1 · Czytnik | lokalne zmiany, commit, branch i MR otwierane w Lupie i czytane szybciej niż w GitLabie | 33 | S1 widok diffa, S2 Git i diff, S3 możliwości GitLaba | diff zgodny z GitLabem na 20 MR, budżety wydajności, 3 osoby przez tydzień, metryki bazowe |
| 2 · Całe review | skrzynka → briefing → trasa → bramka → publikacja → interdiff; self-review przed pushem | 38 | P2-30 izolacja wykonania kodu MR (w tym sposób kopiowania kodu) | ≥ 80% review z Lupy przez 2 sprinty, 0 komentarzy w złej linii na 50 |
| 3 · AI i semantyka | briefing, hipotezy, strażnik testów, pytania; Roslyn; testy na base, pokrycie, mutacje | 24 | P3-01 izolacja `claude -p`, S4 pakowanie sidecara | precyzja ≥ 0,6 i recall ≥ 0,5 na zestawie ewaluacyjnym, akceptacja ≥ 40% |
| 4 · Dopracowanie | emulator, tryb nauki, pętla uczenia, metryki, OAuth, audyty | 9 | — | metryki lepsze niż baza, audyty bez otwartych znalezisk |

Każda faza zostawia działające narzędzie, zgodnie z `docs/input/plan.md`. AI wchodzi w fazie 3, gdy viewer i publikacja działają na prawdziwych MR, a metryki bazowe są zmierzone.

## Ścieżka krytyczna

```text
P1-01 → P1-03 → P1-06 → P1-08 → P1-12 → P1-15 → P1-18a → P1-23 → bramka 1
                                   S2 ┘          S3 ┘   S1 ┘
bramka 1 → P2-08 → P2-09 → P2-10 → P2-11 → P2-12 → P2-13 → P2-19 → P2-20 → P2-21 → P2-23 → bramka 2
                                                    P2-15 → P2-16 → P2-17 ┘
P2-30 (ADR) → P2-31 → P2-32 → P2-33              (równolegle do trasy, P2-31 czeka na ADR)
P3-01 (ADR) → P3-21 → P3-03 → P3-04a → P3-05 → P3-06 → P3-07 → bramka 3
```

Strumienie bez wzajemnych zależności (np. P1-17a powłoka UI i P1-06 Git, P2-30 i trasa, w fazie 3 AI i Roslyn) mogą iść równolegle w osobnych sesjach Claude Code.

## Jak realizować zadanie

1. Przeczytaj zadanie, jego zależności i powiązane ADR. Zależność od spike'a lub ADR oznacza: nie zaczynaj, dopóki nie jest zamknięty lub zaakceptowany.
2. Jedno zadanie to jedna zmiana do ok. 400 LOC (bez wygenerowanych typów i lockfile'i). Większe zadanie dziel przed startem na części z sufiksem `a`, `b` (jak P1-14a i P1-14b) i dopisz podział w pliku fazy.
3. Spełnij kryteria akceptacji i podaj w opisie zmiany komendy oraz ich wyniki (`CLAUDE.md`, „Dowód działania”).
4. Uruchom `/self-review`. Zadania oznaczone **security** przegląda też `security-reviewer`.
5. Zaznacz zadanie jako zrobione w tabeli stanu poniżej, z linkiem do zmiany.

**Definicja ukończenia:** kryteria akceptacji spełnione, CI zielone (od P1-02; wcześniej komendy z `architecture.md` uruchomione lokalnie z wynikiem w opisie zmiany), `/self-review` bez uwag blokujących, nowa zależność wymieniona w `architecture.md`, decyzja zapisana jako ADR (jeśli zapadła), komendy aktualne w `CLAUDE.md`.

## Stan

**Warunek startu:** człowiek akceptuje ADR 0002, 0003 i 0004 (zmiana statusu na „Zaakceptowany”). Do tego czasu według kolejności źródeł z `CLAUDE.md` obowiązuje `docs/input/plan.md`, który zakłada fazę 0. Wzorce komend do `allow` w `.claude/settings.json` wpisuje człowiek na podstawie propozycji z P1-01.

| Faza | Status | Uwagi |
| --- | --- | --- |
| 1 | do zrobienia | najpierw akceptacja ADR, potem P1-01 i spike'i S1, S2, S3 |
| 2 | do zrobienia | P2-30 może zacząć się wcześniej, bo nie zależy od kodu |
| 3 | do zrobienia | P3-01 wymaga odpowiedzi na Q2 z `open-questions.md` |
| 4 | do zrobienia | — |

Pojedyncze zadania odhacza się w pliku fazy, dopisując na końcu tytułu `✅ <link do zmiany>`.
