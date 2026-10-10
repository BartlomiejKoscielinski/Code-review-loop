---
paths:
  - "docs/**/*.md"
---

# Dokumentacja i ADR

## Styl

- Po polsku, zwięźle, w trybie oznajmującym. Identyfikatory, ścieżki, komendy i nazwy plików po angielsku, w backtickach.
- Każde twierdzenie o bibliotece, API lub narzędziu ma link do oficjalnej dokumentacji (najlepiej z wersją). Twierdzenia, których nie da się zweryfikować, oznacz jako **NIEZWERYFIKOWANE** i zleć je subagentowi `fact-checker`.
- Bez placeholderów. Jeśli czegoś jeszcze nie wiadomo, napisz wprost, kto lub która sesja to ustali.
- `docs/input/` tylko czytamy. Plan i decyzje trafiają do `docs/plan/` i `docs/adr/`.

## Format ADR

- Plik `docs/adr/NNNN-krotki-tytul-po-angielsku.md` z czterocyfrowym kolejnym numerem. Nowy ADR tworzy skill `/adr`.
- Sekcje według `docs/adr/0000-template.md`: Status, Data, Decydenci, Kontekst, Rozważane opcje, Decyzja, Konsekwencje, Źródła.
- Statusy: Proponowany, Zaakceptowany, Odrzucony, Zastąpiony przez NNNN.
- Zaakceptowanego ADR nie edytujemy, poza statusem i odnośnikiem do następcy. Zmiana decyzji oznacza nowy ADR.
