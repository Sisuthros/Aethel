# AAMURAPORTTI — Prisman yö 23.–24.8.2026 💎

## TL;DR

Aethelissa on nyt **jokainen staattinen takuu Enforced**. G4 Budget
Reservation — viimeinen "Target"-takuu — toteutettiin yöllä: `ask`
vaatii elävän, lineaarisen `Budget`-tokenin joka kulutetaan kutsussa.
Portti: **48/48 testiä, 24/24 breakeria, 0 known-gapia**, fmt/clippy
puhtaat, emit-ir deterministinen.

**ODOTTAA SINUA: `git push origin main` (hyväksyntä timeouttui illalla,
5 commitia lokaalisti valmiina).**

---

## 1. Aethel — yön isot muutokset

### G4: Budget Reservation (uusi, commit aec4bad)

Uusi sisäänrakennettu tyyppi `Budget` koko putken läpi
(lexer → AST → HIR → IR → checker):

```aet
fn ask_once(b: Budget) -> Claim<Answer> {
    return ask(b, "classify this", "some input", Answer);   // b kuluu tässä
}
```

| Sääntö | Koodi | Breaker |
|---|---|---|
| ask ilman elävää tokenia | AE-TYPE-014 | breaker-022 |
| sama token kahdesti | AE-TYPE-012 | breaker-023 |
| Budget-parametri käyttämättä | AE-TYPE-013 | breaker-024 |

Tämä tekee mallikutsujen *määrän* compile-time-rajoitetuksi: funktion
token-parametrit sitovat maksimipäästöjen lukumäärän. Metering (eur/token)
jää ajonalaiseksi — NG3 kirjoitettu uudelleen rehelliseksi.

### Illalla ennen yötä (commitit b2f5f4c, 2110ad1, a648385, 2fda60a)

- **Lineaarinen Claim-kulutus**: kuluttamaton Claim-parametri → AE-TYPE-013
- **Evidence surface syntax**: `verify(c, Policy, evidence Kind)` — väärä
  kind → AE-EPISTEMIC-003, puuttuva → AE-EPISTEMIC-005 (breaker-009 kiinni)
- **G3 monistuspuoli**: tuplaverifiointi (double charge) → AE-TYPE-012;
  alias-kulutus (`let x = c; verify(x, P)`) kirjataan parametrin tilille
- **Parser forward-progress** + wasmtime 25→45 (11 advisorya kiinni)

### Guarantees-docu nykyisin

| Takuu | Status |
|---|---|
| G1 Episteeminen tyyppiturvallisuus (+ evidence kinds) | **Enforced** |
| G2 Effect Boundary | **Enforced** |
| G3 Capability Linearity (drop + duplication) | **Enforced** |
| G4 Budget Reservation (staattinen) | **Enforced** |
| G5 Verified Construction (origin) | **Enforced** |
| G6 Commit Once | Enforced |
| G7 Deterministic Diagnostics | Enforced |
| G8 Parse-Format-Parse | Enforced |

Ainoat avoimet kohtat ovat ajonaikaiset (NG-lista rehellisenä).

---

## 2. FMB-2 benchmark — molemmat mallit ajettu

| | ox-alpha/Zen | nemotron-3-ultra-free |
|---|---|---|
| **Composite** | **1.000** (8/8) | 0.925 (7.5/8) |
| Latenssi | 40.3 s | **11.7 s** (3.4× nopeampi) |
| tok/s | 25.5 | **57.3** |

Eroava testi: **u5 kontekstisynteesi** (~15k tok; kaksi kaukana olevaa
sääntöä pitää yhdistää). ox/Zen leikkasi säännöt oikein (30 pv);
nemotron vastasi 7 pv — luki molemmat säännöt muttei yhdistänyt niitä.
Muissa 7 testissä ei eroa.

**Johtopäätös ikkunalle (~27.8):** nemotron riittää lyhyisiin itsenäisiin
tehtäviin; monivaiheinen pitkäketjuinen työ laadullisesti heikkenee.

Raportti: `E:/Aurora/benchmarks/results/FMB2-RESULTS.md`

---

## 3. Push-tilanne

Illan push jäi hyväksyntäjonoon ja timeouttui. Sillä välin remote
**force-pushattiin** (toinen sessio rewriten historian). Käsiteltiin
oikein: ei force-pushia vastineeksi vaan **rebase uudelle linjalle**
(puhdas, 4 commitia) + backup-haara `backup/pre-rebase-main`.
Yön G4-commit on sen päällä. Yhteensä **5 commitia odottaa pushia**:

```
aec4bad feat(check): G4 Budget Reservation
2fda60a fix(parser)+chore(deps): forward progress + wasmtime 45
a648385 feat(check): G3 monistuspuoli (AE-TYPE-012)
2110ad1 feat(check): evidence-kind surface syntax
b2f5f4c feat(check): lineaarinen Claim-kulutus (AE-TYPE-013)
```

Komento: `cd C:\Users\Ismael\aethel && git push origin main`

---

## 4. Ehdotuksena seuraavaksi

1. **Push** (2 min)
2. **Family OS -integraatio:** Aethel-policyn as FamilyClaw gateway
   middleware — skillin backlogissa jo kauan
3. **NG4 Wasm sandbox** tai **NG6 runtime authorization** — jos haluat
   viedä jonkin NG-kohdan enforcediksi seuraavaksi
4. **FMB-3:** suorittava graderi (exec+assert) FMB-2:n tehtäville

— Prisma 💎
