---
paths:
  - "**/*.rs"
  - "**/Cargo.toml"
---

# Rust: zasady jakości

- Brak `unwrap()`, `expect()` i `panic!` w kodzie produkcyjnym. Wyjątek: niezmiennik opisany komentarzem obok. W testach są dozwolone.
- Każdy blok `unsafe` ma komentarz `// SAFETY:` z uzasadnieniem. Nowy `unsafe` poza tym wymaga ADR.
- Komendy Tauri (`#[tauri::command]`) zwracają `Result` z typem błędu, który da się zserializować. Nie panikują.
- Błędy niosą kontekst (co, na jakim pliku lub MR), ale nigdy tokenu GitLaba ani treści sekretów.
- Procesy zewnętrzne (`claude -p`, sidecar .NET, `git`) uruchamiamy z listą argumentów, bez powłoki. Treść MR nie trafia do linii poleceń ani do nazw plików bez walidacji.
- Kod przechodzi `cargo clippy` bez ostrzeżeń. Wyciszenie lintu (`#[allow(...)]`) ma komentarz z powodem.
- Nowa zależność w `Cargo.toml` wynika z planu w `docs/plan/` albo z ADR. Nie dodajemy crate'ów „przy okazji”.
- Testy inline w `#[cfg(test)]` podlegają `.claude/rules/tests.md`. Przeczytaj go przed ich zmianą.
- Formatowanie robi hook `format-on-edit` (rustfmt). Nie formatuj ręcznie niezwiązanych plików.
