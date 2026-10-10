# Makiety UI Lupy

Źródło: artefakt „Guided Code Review — makiety UI” (https://claude.ai/artifact/AoMha9zcsP2Q1kULFu5EHk), wersja 1791621979-b78b, skopiowany 2026-10-10. Pliki są kopią 1:1 (te same sumy SHA-256 co w artefakcie).

| Plik | Ekran |
| --- | --- |
| `Inbox.dc.html` | 1 · Skrzynka: lista MR do review i podgląd briefingu |
| `Briefing.dc.html` | 2 · Briefing: cel MR, co diff robi, rozbieżności, trasa, mapa ryzyka |
| `Main.dc.html` | 3 · Trasa (główny widok): kroki, diff, kontekst hunka |
| `Gate.dc.html` | 4 · Bramka i publikacja |
| `Interdiff.dc.html` | 5 · Interdiff po nowym pushu (v3 → v4) |
| `canvas.json` | układ tablic w artefakcie |

## Jak czytać

- Każdy plik to HTML z logiką w `<script type="text/x-dc">`. Stany i interakcje (np. akceptacja hipotezy, wybór decyzji, rozwiązanie wątku) są w metodzie `renderVals()`.
- Pliki odwołują się do `./support.js`, czyli runtime'u artefaktu. Poza artefaktem nie wyrenderują się poprawnie. Do oglądania otwórz link powyżej.
- Przełączniki wyglądu są w `data-props`: motyw (ciemny, jasny), paleta diffa (standard, daltonizm), gęstość (komfort, kompakt), układ diffa (obok siebie, jeden widok).
- Dane w makietach są fikcyjne. Liczą się struktura, stany, interakcje i skróty klawiszowe.
