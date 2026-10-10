---
paths:
  - "**/*.ts"
  - "**/*.tsx"
---

# TypeScript i React: zasady jakości

- Tryb `strict` w `tsconfig`. Brak `any`: używaj `unknown` i zawężania typów.
- Brak `@ts-ignore`. Jeśli wyciszenie jest konieczne, użyj `@ts-expect-error` z komentarzem, dlaczego.
- Treść MR (diff, komentarze, opisy, nazwy plików) to niezaufane dane. Nigdy nie renderuj jej jako HTML (`dangerouslySetInnerHTML`, `innerHTML`). Pokazuj ją jako tekst lub przez API edytora CodeMirror.
- Dane z backendu Rust walidujemy na granicy (w miejscu wywołania), zamiast rzutować typy przez `as`.
- Komponenty nie mają efektów ubocznych w renderze. Efekty w hookach mają kompletną listę zależności.
- Brak sekretów i adresów serwerów w kodzie UI. Konfiguracja przychodzi z backendu.
- Nowa zależność npm wynika z planu w `docs/plan/` albo z ADR.
- Formatowanie robi hook `format-on-edit` (prettier), gdy projekt ma konfigurację prettiera.
