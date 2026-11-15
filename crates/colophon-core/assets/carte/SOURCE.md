# Provenance de `carte.bin`

Le fond des cartes de voyage (P-s4). Un fichier unique, compilé à partir de
Natural Earth par `scripts/carte.sh compiler` (code : `src/carte/compilation.rs`,
format : `src/carte/actif.rs`). Il se recompile à l'octet près à partir des
mêmes fichiers : `scripts/carte.sh verifier`.

- **Source** : Natural Earth, <https://www.naturalearthdata.com>, téléchargé le
  12/11/2026 depuis `https://naciscdn.org/naturalearth/`.
- **Licence** : domaine public (« All versions of Natural Earth raster + vector
  map data found on this website are in the public domain »,
  <https://www.naturalearthdata.com/about/terms-of-use/>). Attribution non
  exigée ; faite quand même : *Made with Natural Earth.*
- **Modifications** : coordonnées arrondies au dix-millième de degré, lignes et
  anneaux simplifiés par Douglas-Peucker (0,01° pour le 1:50m, 0,001° pour le
  1:10m), frontières reconstruites comme les arêtes partagées entre deux pays.
  Aucune donnée ajoutée, aucune propriété (noms, codes) conservée.

| Couche | Version | Rôle dans l'actif | SHA-256 du zip |
|---|---|---|---|
| `ne_50m_land` | 4.1.0 | terre et côtes du monde | `0b8e670cf80dce9cbebe2a193bc44ba5602758c22e1fa603980553646d7ff162` |
| `ne_50m_admin_0_countries` | 5.1.1 | frontières du monde | `5fed433373581fa648920435f937d95f2d3c0200e067409c6478dcdf1b853139` |
| `ne_10m_admin_0_countries` | 5.1.1 | terre et frontières d'une région | `ce1ac7036499a0edd641fbc093cd209a98f96a49d2eca8480aaacad35138a7f6` |
| `ne_10m_coastline` | 5.0.0-pre9 | côtes d'une région | `bfa04cdbcbef07ef90dfca1dabb48062eca29900a113df0f389303e255484017` |
| `ne_10m_lakes` | 5.0.0 | lacs d'une région | `0803a06f9c3cb4671d89b68c48b142aad9366ba40f665245e12a913fbc61722a` |
| `ne_10m_rivers_lake_centerlines` | 5.0.0 | rivières d'une région | `ded71b01870855ccfe19b51f2ec14c9bb48fae23c0e9f3c11974d426433b5c38` |

Les villes (`ne_10m_populated_places_simple`) n'y sont pas : la carte n'écrit
aucun nom, et le dépôt a déjà `cities5000.tsv` pour les lieux.

- **Taille** : 5 068 313 octets (4 333 698 octets une fois passé par `gzip -9`) ;
  plafond du plan : 6 Mo.
- **Empreinte** (FNV-1a 64 du fichier, relue par un test) : `c3d0f609680e20fe`
