---
name: adr
description: Tworzy nowy ADR w docs/adr/ z szablonu 0000-template.md, z kolejnym numerem i statusem Proponowany.
argument-hint: "<tytuł decyzji>"
disable-model-invocation: true
allowed-tools: Read Glob Grep Edit(/docs/adr/**)
---

# Nowy ADR

Tytuł decyzji: $ARGUMENTS

1. Jeśli tytuł jest pusty, zapytaj o niego i zakończ.
2. Znajdź istniejące ADR (`docs/adr/[0-9][0-9][0-9][0-9]-*.md`). Nowy numer to największy numer plus jeden, w zapisie czterocyfrowym. `0000` to szablon.
3. Sprawdź, czy któryś ADR nie dotyczy już tej decyzji. Jeśli tak, pokaż go i zapytaj, czy nowy ADR ma go zastąpić.
4. Nazwa pliku: `docs/adr/NNNN-<slug>.md`. Slug to krótki angielski kebab-case (do 6 słów), nawet gdy tytuł jest po polsku.
5. Przeczytaj `docs/adr/0000-template.md` i wypełnij go po polsku według `.claude/rules/docs.md`:
   - nagłówek `# NNNN. <tytuł>`, status `Proponowany`, dzisiejsza data;
   - kontekst, opcje i decyzja z bieżącej rozmowy oraz z `docs/input/` i `docs/plan/`;
   - każde twierdzenie o bibliotece lub API z linkiem. Brak źródła oznacz **NIEZWERYFIKOWANE** i zaproponuj subagenta `fact-checker`.
   Nie wymyślaj decydentów ani faktów. Jeśli brakuje informacji, zapytaj, zanim zapiszesz plik.
6. Jeśli nowy ADR zastępuje stary, zmień w starym tylko linię statusu na `Zastąpiony przez [NNNN](NNNN-<slug>.md)`.
7. Pokaż ścieżkę i krótkie streszczenie. Status `Zaakceptowany` ustawia człowiek po review.
