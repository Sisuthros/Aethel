# Aethel Core v0.1 — Julkaisusuunnitelma
> **3 kuukautta** | 4 sprinttiä | Tavoite: Semantic Truth Slice v0.1

---

## Sprintti 1: Foundation Repair (1.–14. pv)
**Tavoite:** Workspace vihreä, keyword-ongelmat pois, parser tuottaa oikean AST:n

### Päivä 1–3: Keyword cleanup
- [ ] Poista `Receipt`, `Budget`, `Context`, `SignedAttestation`, `CryptographicProof`, `AuditLog`, `HumanReview`, `TrustedRegion`, `UntrustedRegion` keyword-listasta → menevät `Ident`-tokenina
- [ ] Pidä keywordeina vain: `fn`, `let`, `mut`, `return`, `if`, `else`, `while`, `for`, `in`, `match`, `type`, `struct`, `enum`, `use`, `mod`, `pub`, `uses`, `ask`, `verify`, `commit`, `once`, `Claim`, `Verified`, `Policy`, `effect`, `break`, `continue`, `new`, `reason`
- [ ] Päivitä `test_lex_keywords` vastaamaan
- [ ] Poista `KwReceipt`-käsittely parserista

### Päivä 4–7: Parser robustness
- [ ] Effect body: handle uudet rivit oikein (Semi skip)
- [ ] Policy body: handle useita `claim` entryjä oikein
- [ ] Trailing commas kaikkialla (struct/e enum/funktio parametrit)
- [ ] Kommentit `//` ja `/* */` — tarkista että skip toimii
- [ ] Parse error recovery — älä kaada koko moduulia yhden virheen takia

### Päivä 8–10: CI pipeline
- [ ] Korjaa `.github/workflows/ci.yml`:
  - Poista `strip_effect_defs`-riippuvuus
  - Negatiivinen testi: `invalid_unverified.aet` → exit 1 oikein (shell-testillä, ei cargo run --release)
  - Positiivinen testi: `valid_verified.aet` → exit 0
  - Pinnattu Rust toolchain (MSRV 1.85)
- [ ] `cargo check --workspace --all-targets` vihreäksi (0 varoitusta)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` vihreäksi

### Päivä 11–14: Test infrastructure
- [ ] Parser snapshot testit (insta) — 5+ fixturea
- [ ] Lexer snapshot testit
- [ ] Moniriviset syötteet, tyhjät tiedostot
- [ ] Määritä `cargo test --workspace --all-features` vihreäksi

---

## Sprintti 2: Pipeline (15.–35. pv)
**Tavoite:** AST → HIR → Checker → IR pipeline toimii oikeasti

### Päivä 15–20: HIR lowering real
- [ ] `lower_module(AST) → HIR` kaikille item-tyypeille
- [ ] `lower_expr()` kaikille Expr-varianteille (Verify, Reason, CommitOnce mukaan)
- [ ] `lower_type()` kaikille Type-varianteille (Claim, Verified mukaan)
- [ ] Lower effect definitions → HIR EffectDef/EffectOperation
- [ ] Lower struct definitions → HIR StructDef/StructField
- [ ] Jos loweraus epäonnistuu → diagnostic, ei panic

### Päivä 21–27: Type environment (proper)
- [ ] `TypeEnvironment` scoped: enter_scope/exit_scope oikeasti
- [ ] Funktioparametrit menevät scopeen (param name → type)
- [ ] `let`-bindaukset menevät scopeen (name → type)
- [ ] Block-scope: sisäkkäiset blockit luovat uuden scopen
- [ ] Type resolution: `path.resolve_type(name)` → `Option<IrType>`
- [ ] Jos unresolved → diagnostic AE-RESOLVE-001, ei Unit default

### Päivä 28–35: Checker real (ei funktionimiä)
- [ ] `check_expr()` palauttaa `(IrExpr, IrType)` — expression + sen tyyppi
- [ ] `check_stmt()` palauttaa `(IrStmt, Option<IrType>)`
- [ ] Literals: `42` → `Int`, `"hello"` → `String`, `true` → `Bool`
- [ ] Path: etsi var, palauta sen tyyppi; unresolved → error
- [ ] MethodCall: etsi effect, tarkista argumenttityypit
- [ ] Verify: Claim<T> → Verified<T, Policy> tyyppimuunnos
- [ ] CommitOnce: tarkista argumentit (ei Claim<T> sinne missä Verified<T> vaaditaan)
- [ ] Reason: palauta Claim<String>

---

## Sprintti 3: Type System (36.–56. pv)
**Tavoite:** Claim<T> vs Verified<T, Policy> pakotetaan oikeasti

### Päivä 36–42: Core epistemic rule
- [ ] `Claim<T>` → `Verified<T, Policy>` vaatii `verify(claim, Policy)` callin
- [ ] Ilman verifya effect boundary → AE-EPISTEMIC-001
- [ ] verify called with wrong policy → AE-EPISTEMIC-003
- [ ] Verify non-Claim → AE-EPISTEMIC-002
- [ ] Direct Verified construction → blocked
- [ ] Policy registry: tarkista että policy on olemassa

### Päivä 43–49: Effect system
- [ ] `uses X:` → `X` on effect reference funktion scopeen
- [ ] Effect method call `payments.refund()` tarkistaa:
  - Onko `X` declared `uses` clause?
  - Onko `refund` declared operation?
  - Ovatko argumenttityypit oikein?
  - Onko Claim<T> → Verified<T, Policy> vaatimus täytetty?
- [ ] Effect call ilman `uses` → diagnostic
- [ ] Tuntematon operation → diagnostic
- [ ] Tyyppivirhe argumentissa → diagnostic

### Päivä 50–56: Coverage hardening
- [ ] Tipo mismatch (int vs string) → diagnostic
- [ ] Undeclared variable → diagnostic  
- [ ] Undeclared type → diagnostic
- [ ] Effect shadowing → diagnostic
- [ ] Multiple effects samassa `uses` clause
- [ ] Effect method chaining
- [ ] Nested blocks scope resolution

---

## Sprintti 4: Verification & Release (57.–84. pv)
**Tavoite:** Julkaisukelpoinen v0.1-alpha

### Päivä 57–63: Adversarial tests (10+)
- [ ] **Rename file** → still fails if invalid
- [ ] **Rename function** (`refund_invalid` → `process_refund`) → still fails
- [ ] **Rename parameter** (`claim` → `proposal`) → still fails
- [ ] **Different effect** (PaymentGateway → ProductionDeploy) → still fails
- [ ] **Different operation** (refund → process) → still fails
- [ ] **Completely different names** → still works based on types
- [ ] **Verify with correct policy** → passes ✓
- [ ] **Verify with wrong policy** → fails AE-EPISTEMIC-003
- [ ] **Effect not declared in uses** → fails
- [ ] **Pass Claim<A> where Verified<B, Policy>** → fails
- [ ] **Attempt direct Verified construction** → fails
- [ ] **Same compilation twice** → byte-identical IR

### Päivä 64–70: Semantic IR JSON
- [ ] `aethel emit-ir program.aet` → JSON stdout
- [ ] JSON on schema-versioned (`"ir_version": "0.1"`)
- [ ] Byte-stable: sama input → sama output
- [ ] Deterministic ordering (sort maps by key)
- [ ] Ei absoluuttisia polkuja (vain file_id)
- [ ] Document IR JSON schema `docs/ir-schema.md`

### Päivä 71–77: Documentation & README
- [ ] README: tarkka nykytila (ei ylilyöntejä)
- [ ] `docs/guarantees.md` — mitä kieli takaa
- [ ] `docs/non-guarantees.md` — mitä ei vielä takaa (rehellisesti)
- [ ] `docs/examples/` — 5+ toimivaa esimerkkiä
- [ ] `docs/adr/` — architecture decision records ajantasalle
- [ ] `docs/evidence/AETHEL_V0_1_SEMANTIC_TRUTH_SLICE.md` — todisteet

### Päivä 78–84: Release prep
- [ ] CHANGELOG.md v0.1-alpha
- [ ] `cargo publish --dry-run` (kaikki crate)
- [ ] Tag v0.1.0-alpha
- [ ] GitHub release
- [ ] Release announcement (lyhyt ja rehellinen)

---

## Riskiarvio

### Korkea riski (voi viivästyttää):
- **Pipeline integration** — AST→HIR→checker on monimutkaisin osa
- **Type environment scope** — helppo tehdä väärin, scope-leaks ovat yleisiä  
- **Adversarial tests** — tämä paljastaa eniten bugeja

### Keskiriski:
- **Effect system** — `uses X:` → method call → operation lookup on monivaiheinen
- **Semantic IR** — deterministisyys vaatii huolellista JSON käsittelyä

### Matala riski:
- **Keyword cleanup** — mekaaninen, helposti testattava
- **CI pipeline** — standardi Rust CI
- **Documentation** — aikaa vievää mutta ennustettavaa

---

## Definition of Done (v0.1-alpha)

1. ✅ `cargo build` vihreä (0 varoitusta)
2. ✅ `cargo test --workspace` vihreä
3. ✅ `cargo clippy --workspace -- -D warnings` vihreä
4. ✅ `cargo fmt --all -- --check` vihreä
5. ✅ 12+ adversarial testiä vihreinä
6. ✅ CLI: `cargo run -p aethel-cli -- check invalid.aet` → exit 1, AE-EPISTEMIC-001
7. ✅ CLI: `cargo run -p aethel-cli -- check valid.aet` → exit 0
8. ✅ CLI: `cargo run -p aethel-cli -- emit-ir valid.aet` → JSON output
9. ✅ README vastaa todellisuutta (ei ylilyöntejä)
10. ✅ CI vihreä (sisältää negatiiviset testit)
11. ✅ Kaikki demo-häkit poistettu (ei filename/string/function-name checks)
12. ✅ 10+ esimerkkiä `examples/` hakemistossa

---

*Laadittu 13.7.2026 — Prisma 💎*
*Arvio: 3 kuukautta (lokakuu 2026)*
