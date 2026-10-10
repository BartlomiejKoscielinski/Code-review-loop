# 0002. Architektura i granice modułów

- Status: Zaakceptowany (@Bartek, 2026-10-10, zgoda na start implementacji w sesji planowania)
- Data: 2026-10-10
- Decydenci: @Bartek

## Kontekst

`docs/input/plan.md` rekomenduje Tauri 2 na Windows (WebView2), rdzeń w Ruście dla Gita, diffa i cache, UI w React + TypeScript z CodeMirror 6 i sidecar .NET z Roslyn. Jako mitygację ryzyka „Stack Rust + TS w zespole .NET” plan proponuje Rust tylko dla Gita, diffa i IPC, a logikę domenową w TS i .NET.

10.10.2026 @Bartek potwierdził w sesji planowania, że stack zostaje, a aplikację implementuje wyłącznie Claude Code. Ryzyko „wiedza u jednej osoby” zmienia się więc w ryzyko „kod, którego zespół nie umie samodzielnie zmienić”. Mitygacją są małe zmiany, testy, ADR i review w świeżym kontekście z `CLAUDE.md`.

Do ustalenia zostało, gdzie leżą klient GitLaba, token i uruchamianie procesów. Od tego zależy granica bezpieczeństwa.

## Rozważane opcje

1. Rdzeń Rust tylko dla Gita, diffa i IPC; GitLab i procesy w TypeScripcie.
2. Rdzeń Rust ma całe I/O i sekrety (Git, diff, SQLite, GitLab, token, procesy), TypeScript ma czystą logikę domenową i UI, .NET ma semantykę C#.
3. Cała logika w Ruście, TypeScript tylko jako widok.

## Decyzja

Wybieramy opcję 2: „Rust liczy fakty i trzyma sekrety, TypeScript układa z faktów decyzje i je pokazuje, .NET rozumie C#”.

- Webview nie ma tokenu ani dostępu do sieci (restrykcyjne CSP), więc wroga treść MR wyświetlona w UI nie może wykraść tokenu.
- Procesy zewnętrzne (`git`, `claude -p`, sidecar, build) uruchamia wyłącznie rdzeń, z listą argumentów i listą dozwolonych zmiennych środowiska.
- Algorytm trasy, reguły bramki i budżet sesji są czystymi funkcjami w `packages/domain`. Ta sama wersja MR daje każdemu tę samą trasę.
- Układ repo, nazwy modułów, strefy zaufania, zależności i komendy opisuje `docs/plan/architecture.md`.

## Konsekwencje

- Granica bezpieczeństwa jest w jednym procesie i jednym języku, łatwiej ją przeglądać (`security-reviewer`).
- Logikę domenową testujemy szybko w Vitest, bez Tauri.
- Typy IPC trzeba generować z Rusta i pilnować ich aktualności w CI (zadanie P1-03).
- Trzy ekosystemy w CI (Rust, Node, .NET od fazy 3) wydłużają build. Zadanie P1-02 ustawia cache.
- Kolejne kroki: zadania P1-01 … P1-05 w `docs/plan/phase-1.md`.

## Źródła

- `docs/input/plan.md`, sekcje „Architektura” oraz „Ryzyka, metryki sukcesu, otwarte pytania”.
- Tauri 2 na Windows używa WebView2: https://v2.tauri.app/start/prerequisites/
- Sidecar w Tauri 2 (`bundle.externalBin`, plugin shell): https://v2.tauri.app/develop/sidecar/
- Sprawdzenie faktów: raport `fact-checker` z 2026-10-10 (sesja planowania).
