# 0006. Lokalne zmiany przez migawkę na kopii indeksu

- Status: Zaakceptowany (decyzja techniczna delegowana przez @Bartka w sesji 2026-10-10, `docs/plan/decisions.md`)
- Data: 2026-10-10
- Decydenci: @Bartek

## Kontekst

ADR 0005 zakładał, że lokalne zmiany liczymy z `git status --porcelain=v2 -z` i `git hash-object --stdin-paths` bez `-w`. Implementacja P1-07 i jej review pokazały trzy problemy:

1. **Zła treść do porównania.** Bez zapisu bloba nie da się odczytać treści pliku po filtrach Gita. Przy `core.autocrlf=true` plik ma na dysku CRLF, a w repo LF, więc diff surowych bajtów pokazałby każdą linię jako zmienioną.
2. **Własne parsowanie wyjścia Gita jest kruche.** Konflikty (`U`), symlinki, zagnieżdżone repozytoria i nazwy plików z cudzysłowami lub znakami nowej linii wymagałyby osobnej obsługi.
3. **Ryzyko zawieszenia.** Przekazywanie listy ścieżek przez stdin przy czytaniu stdout dopiero po zakończeniu procesu może się zakleszczyć przy setkach plików.

## Rozważane opcje

1. Zostać przy `git status` i `hash-object` bez `-w`, a filtry Gita (EOL, LFS) odtwarzać we własnym kodzie.
2. `git diff --raw <base>` i `hash-object -w` do osobnego magazynu obiektów, z własną obsługą przypadków brzegowych (pierwsza wersja P1-07).
3. Migawka przez Gita: kopia indeksu w cache Lupy, `git add -A` i `git write-tree` z magazynem obiektów w cache, potem `git diff-tree -M <base> <drzewo>`.

## Decyzja

Wybieramy opcję 3. Zastępuje ona punkt ADR 0005 o working tree. Reszta ADR 0005 (gix do odczytu commitów, `imara-diff` do diffu linii) obowiązuje bez zmian.

- `WorkTree::snapshot` kopiuje `.git/index` do katalogu cache i uruchamia `git add -A` oraz `git write-tree` z ustawieniami:
  - `GIT_INDEX_FILE` = kopia indeksu;
  - `GIT_OBJECT_DIRECTORY` = katalog obiektów w cache Lupy;
  - `GIT_ALTERNATE_OBJECT_DIRECTORIES` = obiekty repo dewelopera (w cudzysłowie, gdy ścieżka zawiera separator).
- Zmiany liczy `git diff-tree -r -z -M --raw --no-abbrev <base> <drzewo>`. Nieznany status lub tryb jest błędem, a nie zgadywaniem.
- Procesy `git` nie dziedziczą zmiennych przekierowujących (`GIT_DIR`, `GIT_INDEX_FILE`, `GIT_OBJECT_DIRECTORY`, `GIT_CONFIG_PARAMETERS` i podobnych).
- Katalog cache nie może leżeć w working tree. Ścieżki są liczone od korzenia repo, także przy otwarciu podkatalogu.

## Konsekwencje

- Ignorowanie plików, atrybuty, `core.autocrlf`, filtry, symlinki, submoduły, zagnieżdżone repozytoria i konflikty działają jak w Gicie. Testy: `crates/lupa-git/tests/worktree.rs` i `worktree_edge_cases.rs`.
- Migawka opisuje treść, nie stan indeksu. Plik usunięty z indeksu, ale wciąż na dysku z tą samą treścią, nie jest zmianą.
- Nie zapisujemy indeksu ani obiektów w repo dewelopera (test `does_not_write_objects_into_the_repository`). Zostają dwa wyjątki, oba nieszkodliwe:
  - filtr clean LFS zapisuje obiekty do `.git/lfs/objects`;
  - Git może odświeżyć czas modyfikacji obiektu, który już jest w repo.
- `git add -A` uruchamia filtry i fsmonitor skonfigurowane przez dewelopera dla jego własnego repo, tak jak zrobiłby to sam Git. Repozytorium MR to inny model zaufania: `WorkTree` nie jest do niego używany (P2-30).
- Duże nieśledzone i nieignorowane pliki są hashowane przy każdym odświeżeniu. Limity rozmiaru dochodzą w P1-10, a sprzątanie magazynu w cache w P1-04 (LRU).
- 3000 zmienionych plików liczy się w ok. 2 s w kontenerze Linux, razem z budową repo testowego. Pomiar na Windows to zadanie P1-13 (budżet 300 ms dla 5000 plików bez zmian).

## Źródła

- `git add`, `git write-tree`, `git diff-tree`: https://git-scm.com/docs/git-add, https://git-scm.com/docs/git-write-tree, https://git-scm.com/docs/git-diff-tree
- Zmienne środowiskowe Gita (`GIT_INDEX_FILE`, `GIT_OBJECT_DIRECTORY`, `GIT_ALTERNATE_OBJECT_DIRECTORIES`): https://git-scm.com/book/en/v2/Git-Internals-Environment-Variables
- Review P1-07 (code-reviewer, 2026-10-10) i ADR 0005.
