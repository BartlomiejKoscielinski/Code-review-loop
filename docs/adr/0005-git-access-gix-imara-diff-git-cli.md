# 0005. Dostęp do Gita: gix, imara-diff i git CLI

- Status: Zaakceptowany (decyzja techniczna delegowana przez @Bartka w sesji 2026-10-10, `docs/plan/decisions.md`)
- Data: 2026-10-10
- Decydenci: @Bartek

## Kontekst

Lupa liczy diffy lokalnie (`docs/input/plan.md`, „Lokalnie i szybko”) i musi pokazywać te same zmiany co Git i GitLab, bo inaczej komentarze trafią w złe linie. Budżety: otwarcie MR z cache < 300 ms, następny hunk < 16 ms. Zespół pracuje na Windows, z `core.autocrlf` i `.gitattributes` w repo MAUI. Spike S2 ([`spikes/gix-diff-parity`](../../spikes/gix-diff-parity/README.md)) zmierzył zgodność i wydajność na repo CommunityToolkit.Maui i dotnet/maui.

## Rozważane opcje

1. `git` CLI do wszystkiego: pełna zgodność, ale proces na każde wywołanie i parsowanie tekstu.
2. `git2` (libgit2): biblioteka natywna, osobny kod C do budowania na Windows.
3. `gix` do odczytu obiektów i drzew, `imara-diff` do diffu linii, `git` CLI tam, gdzie liczy się dokładne zachowanie Gita (working tree, fetch, worktree).

## Decyzja

Wybieramy opcję 3.

- `gix` 0.89 (bez domyślnych funkcji: `revision`, `blob-diff`, `sha1`) rozwiązuje rewizje, liczy merge-base, porównuje drzewa z wykrywaniem zmian nazw (50%, jak `git diff -M`) i czyta bloby. Zgodność z `git diff --raw -M` sprawdzają testy `crates/lupa-git/tests/read_repo.rs`.
- `imara-diff` 0.2 (`Algorithm::Histogram` + `postprocess_lines`) liczy diff linii. Hunki z kontekstem liczy `lupa-diff`, bo drukarka unified z `imara-diff` dodaje linię kontekstu za dużo przy początku pliku.
- `git` CLI obsługuje status working tree (`git status --porcelain=v2 -z`), identyfikatory treści (`git hash-object --stdin-paths` bez `-w`), fetch i kopie robocze. Wywołania idą bez powłoki, z listą argumentów.

## Konsekwencje

- Lista zmienionych plików dla 3105 zmian liczy się w ok. 31 ms, a diff 1014 plików w 222 ms (S2). Diff liczymy leniwie per plik.
- Filtry, `core.autocrlf` i LFS w working tree działają jak w Gicie, bez własnej implementacji.
- Wymagany jest `git` na maszynie dewelopera. Zespół go ma.
- Znane odchylenia od Gita:
  - **ślizgacz:** ok. 0,3% plików, blok zmian przesunięty o 1–3 linie przy tych samych sumach `+`/`−`. P2-15 sprawdza pozycje komentarzy z diffem GitLaba;
  - **wykrywanie zmian nazw blisko progu 50%:** gix liczy podobieństwo inaczej niż Git (Git od 2.33 najpierw paruje pliki o tej samej nazwie bazowej). Przy kilku kandydatach o podobieństwie ok. 50% parowanie może się różnić. Testy obejmują zmiany nazw z wysokim podobieństwem i zmianę nazwy katalogu;
  - **merge-base przy kilku najlepszych bazach (criss-cross):** zachowanie względem `git merge-base` nie jest jeszcze sprawdzone. Do sprawdzenia w P1-12.
- Kolejne kroki: P1-07 (working tree przez `git` CLI), P1-08 (`lupa-diff` na `imara-diff`), P1-12 (pusty base dla pierwszego commita, rodzice merge commita).

## Źródła

- Spike S2: [`spikes/gix-diff-parity/README.md`](../../spikes/gix-diff-parity/README.md)
- gix, stan funkcji: https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md
- imara-diff: https://github.com/pascalkuthe/imara-diff
- `git status --porcelain=v2`: https://git-scm.com/docs/git-status#_porcelain_format_version_2
- `git hash-object`: https://git-scm.com/docs/git-hash-object
