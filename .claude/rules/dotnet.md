---
paths:
  - "**/*.cs"
  - "**/*.csproj"
  - "**/Directory.Build.props"
---

# .NET (sidecar Roslyn): zasady jakości

- `Nullable` włączone dla wszystkich projektów. Ostrzeżeń nie wyciszamy bez komentarza z powodem.
- Brak `.Result`, `.Wait()` i `async void` (poza handlerami zdarzeń). Metody asynchroniczne przyjmują `CancellationToken` i go przekazują.
- Build, testy i ładowanie projektów z MR do Roslyn (MSBuildWorkspace, ewaluacja MSBuild) wykonują kod z repozytorium MR.
  - Wolno je uruchamiać tylko w worktree w katalogu cache narzędzia, w osobnym procesie z limitem czasu i bez tokenu GitLaba w środowisku.
  - Potrzebna jest ścieżka zatwierdzona w ADR i przegląd `security-reviewer`.
  - Bez takiego ADR dozwolona jest tylko analiza bez MSBuild: drzewa składni Roslyn z samych plików `.cs` albo Tree-sitter.
  - Sidecar nigdy nie ładuje assembly z MR do własnego procesu.
- Komunikacja z rdzeniem Tauri tylko przez kontrakt ustalony w planie. Sidecar nie ma własnego dostępu do GitLaba ani do tokenu.
- Wyjątki nie przenoszą treści MR ani ścieżek użytkownika do logów bez potrzeby.
- Nowy pakiet NuGet wynika z planu w `docs/plan/` albo z ADR.
- Formatowanie robi hook `format-on-edit` (`dotnet format whitespace`) na podstawie `.editorconfig`.
