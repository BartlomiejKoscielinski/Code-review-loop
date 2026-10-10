---
name: spike
description: Zakłada spike w spikes/<nazwa>/README.md z hipotezą, metodą, kryterium sukcesu i limitem czasu albo zamyka go, zapisując wynik.
argument-hint: "start <nazwa> | close <nazwa>"
arguments: action spike
disable-model-invocation: true
allowed-tools: Read Glob Grep Edit(/spikes/**)
---

# Spike

Akcja: `$action`, nazwa: `$spike`. Zasady spike'ów są w `spikes/README.md`.

Nazwa musi być angielskim kebab-case (np. `claude-cli-readonly-isolation`). Jeśli nie jest albo akcja jest inna niż `start` lub `close`, pokaż poprawne użycie i zakończ.

## start

1. Jeśli `spikes/$spike/` istnieje, zatrzymaj się i pokaż jego `README.md`.
2. Ustal z rozmową cztery elementy. Brakujących nie wymyślaj, tylko o nie zapytaj:
   - **Pytanie:** jedno pytanie techniczne, na które spike odpowiada, i decyzja (ADR), którą odblokuje;
   - **Hipoteza:** oczekiwana odpowiedź;
   - **Metoda:** kroki eksperymentu, środowisko (Windows, wersje narzędzi);
   - **Kryterium sukcesu:** mierzalny warunek potwierdzenia lub obalenia;
   - **Limit czasu:** w godzinach.
3. Utwórz `spikes/$spike/README.md` z tymi sekcjami oraz `Status: w toku` i datą rozpoczęcia. Sekcję „Wynik” zostaw z jednym zdaniem: wynik zapisze `/spike close $spike`.

## close

1. Przeczytaj `spikes/$spike/README.md`. Jeśli nie istnieje, zakończ.
2. Uzupełnij sekcję „Wynik”:
   - **Werdykt:** hipoteza potwierdzona, obalona albo nierozstrzygnięta;
   - **Dowody:** komendy i ich wyniki z rozmowy lub z plików w `spikes/$spike/`, z wersjami narzędzi;
   - **Czas:** faktyczny czas wobec limitu;
   - **Rekomendacja:** co zapisać w ADR i jakie ryzyka zostają.
3. Zmień status na `zamknięty`. Nie twórz ADR automatycznie, tylko zaproponuj `/adr <tytuł>`.
