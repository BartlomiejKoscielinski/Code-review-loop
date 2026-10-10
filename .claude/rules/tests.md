---
paths:
  - "**/tests/**"
  - "**/test/**"
  - "**/__tests__/**"
  - "**/__snapshots__/**"
  - "**/*.{Test,Tests,UnitTest,UnitTests,IntegrationTest,IntegrationTests,FunctionalTest,FunctionalTests,E2ETest,E2ETests}/**"
  - "**/tests.rs"
  - "**/*_test.rs"
  - "**/*_tests.rs"
  - "**/*.{test,spec}.{ts,tsx,mts,cts,js,jsx,mjs,cjs}"
  - "**/*Tests.cs"
  - "**/*Test.cs"
  - "**/*.snap"
  - "**/*.verified.*"
  - ".claude/hooks/*.test.mjs"
---

# Testy: zasady jakości

Wzorce `paths` odpowiadają funkcji `isTestFile` w `.claude/hooks/_lib.mjs`. Przy zmianie aktualizuj oba miejsca.

## Nowy test

- Test musi nie przechodzić bez testowanej zmiany. Przed oddaniem pokaż, że failuje na kodzie bez zmiany (wynik komendy w odpowiedzi), albo napisz, dlaczego nie da się tego pokazać.
- Jeden test sprawdza jedno zachowanie, a jego nazwa to zachowanie opisuje.
- Testy są deterministyczne: bez sieci, bez prawdziwego GitLaba i bez prawdziwego `claude -p`, z kontrolą zegara i losowości. Zewnętrzne zależności zastępujemy atrapami na granicy modułu.
- Dane testowe nie zawierają prawdziwych tokenów ani prawdziwych treści MR.

## Istniejące testy

- Nie osłabiamy ich: nie usuwamy asercji, nie dopasowujemy oczekiwanych wartości do nowego kodu, nie dodajemy `#[ignore]`, `.skip`, `[Fact(Skip = ...)]` i nie zwiększamy limitów czasu ani tolerancji.
- Edycja lub usunięcie istniejącego testu wymaga zgody człowieka. Hook `guard-tests` pyta o zgodę, a `guard-delete` pyta przy usuwaniu.
- Snapshot aktualizujemy tylko z uzasadnieniem w opisie zmiany.
- Czerwony test to informacja. Najpierw ustal, czy błąd jest w kodzie, czy w teście, i pokaż dowód.
