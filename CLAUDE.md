# Booklibre (ex-Colophon), pilote technique

Ce fichier possède la **technique** : le moteur, l'app, le gate, les pièges de code,
l'architecture. La **gestion** (le cap, la publication, les fournisseurs, le verdict
papier, le lancement, les arbitrages produit) vit hors du dépôt, dans
`~/Documents/08_IA/02_Outputs/colophon/CLAUDE.md`. Une session cloud ne voit que ce
fichier : tout ce qu'il faut pour coder est ici, et rien d'autre n'est supposé.

Le glossaire du domaine est `CONTEXT.md`. Un synonyme dans un diff est un rapport de bug
qui attend.

Logiciel libre d'albums photo : un dossier en entrée, composition automatique, tout
modifiable, PDF prêt à imprimer. Tauri 2 (Rust + React), GPL-3.0, version 1.1.0 : le tag `v1.1.0` est le
geste d'Alexis, et `release.yml` le refuse si les quatre fichiers de version ou la section
`## [1.1.0]` du CHANGELOG manquent. Depuis la 1.1, la release compile le relais réel et les
tarifs (variables du dépôt `COLOPHON_RELAIS_REEL` et `COLOPHON_TARIFS_REEL`) ; `mas.sh` les retire. Le tag `v1.0.0` (`b222272`) existe, ne sort jamais (import
Photos cassé) et ne se repose pas ; le Mac App Store garde sa 1.0.0 (build 173, déjà corrigé).

**Le logiciel s'appelle Booklibre (R-s1), le code s'appelle encore Colophon.** Tout ce
qu'un utilisateur lit dit Booklibre : `productName`, fenêtre, menus, i18n, installeurs,
CLI `booklibre`, page de fin (« Composé le … avec Booklibre {version}. », titre
« Colophon » inchangé), `/Producer` et `x:xmptk`, liens (`booklibre.app`,
`hello@booklibre.app`, `github.com/alexis-morain/booklibre`). Ne bougent pas, exprès :
l'identifiant `fr.morain.colophon`, les crates et modules, `COLOPHON_*`, `album.json`,
le dossier d'import `~/Pictures/Colophon` (`photos::dossier_propose`, les photos déjà
importées y vivent) et **l'`UpgradeCode` du MSI**, épinglé dans `tauri.conf.json` à la
valeur lue par `msiinfo` dans le MSI publié de `v1.0.1` (le UUID v5 de
`Colophon.exe.app.x64`) : sans lui, Tauri le dérive du `productName` et la 1.1
s'installerait à côté de la 1.0.1. `i18n.test.ts` refuse « Colophon » dans toute chaîne
sauf trois anglaises qui nomment la page ; `le_renommage_garde_l_identifiant_et_l_upgrade_code`
tient la configuration. L'updater interroge `booklibre.app/latest.json` puis GitHub ;
`maj-site.yml` publie le premier à la publication d'une release, inerte sans
`CLOUDFLARE_API_TOKEN`. Le nom est dans chaque PDF : le banc
`banc_octets_d_un_album_sans_objet_libre` rend désormais **214 821 o** (les 214 814 cités
plus bas sont ceux de l'ancien nom, et le code de R-s1 les rend encore avec lui), et
`EMPREINTE_SANS_FOND` de `cover.rs` a été reprise de même.

## Si tu es une session cloud, commence par ça

```bash
./scripts/install-cloud-deps.sh
```

Il pose deux choses que l'image n'a pas, et sans lesquelles le gate ment :

- les **dépendances système de Tauri** (webkit2gtk et compagnie), sans quoi
  `cargo build` échoue sur `colophon-app` et rien d'autre ne se lit dans l'erreur ;
- les **paquets npm** de `crates/colophon-app`. Mesuré le 27/08 : sans `node_modules`,
  `npx tsc --noEmit` sort 1088 erreurs de résolution de modules, aucune ne parlant d'un
  type du projet, et `vitest` ne démarre jamais. Le gate rend 2 sans que rien du projet
  soit en cause. La CI ne voit pas ce piège, elle fait son `npm ci` en étape séparée.

Le script ne sort jamais en échec et se saute tout seul partout ailleurs qu'en session
cloud. Compter environ quatre minutes pour le premier `cargo build --release` derrière.

Il n'est **pas** branché en hook : ce dépôt n'exécute rien tout seul au démarrage d'une
session. On l'appelle, explicitement, et ça se voit.

## Le gate

`./scripts/check.sh` est la seule définition de ce qui est vert. La CI ne fait que
l'installer et le lancer, sur macOS, Ubuntu et Windows. Une règle qui n'existerait que
dans la CI serait une règle que personne ne peut vérifier avant de pousser.

Un seul saut est légitime hors du Mac, et le script l'annonce lui-même : **`pdf-png`**,
qui dépend de `sips`, donc de macOS. Tout autre saut est un échec, pas une tolérance.
Le linter d'albums, lui, ne se saute plus jamais : fiches absentes, gate rouge.

### Le linter tourne partout, les photos ne voyagent pas

Le linter recompose les trois jeux de référence avec le code du jour, puis les audite :
c'est lui qui attrape une régression du Composer avant qu'elle ne parte. Les photos
(5,7 Go) restent sur le Mac ; ce qui voyage, c'est leur relevé — les fiches versionnées
de `crates/colophon-core/fiches/`, régénérées par `scripts/fiches.sh` quand l'analyse
change. Sans les photos, `check.sh` compose depuis les fiches (`--depuis-fiches`, qui
s'arrête avant les pixels et le dit : ni vignettes, ni PDF, ni couverture) et audite
pareil — l'audit lit le `releve.json` posé à côté de l'album, et **refuse de noter**
quand il n'a ni photos ni relevé, plutôt que de laisser le compteur de résolution muet.
**Le GPS sort des fiches arrondi** (`releve::gps_public`, dans `Releve::ecrire`) : la
décimale la plus grossière sous laquelle `places::nearest` nomme encore la même ville,
une décimale (≈ 11 km) pour 99 % des points, jamais le mètre. Les fiches sont publiques
et ce sont des vacances. Un arrondi aveugle au kilomètre changeait deux titres de
chapitre (Roissy, Crozon) et rougissait l'identité ; garder la ville point par point la
tient par construction, parce que tout ce que la composition demande à une coordonnée
passe par `nearest`. Le chemin des photos lit toujours le chiffre de l'appareil.

Sur le Mac, `check.sh` prouve à chaque passage que les deux chemins rendent le même
`album.json` à l'octet (`root` excepté, normalisé par `scripts/identite-fiches.py`), la
même `curation.json`, et le même verdict de linter, compteur par compteur. Cette
identité est le test de la fonctionnalité **et** le test de fraîcheur des fiches, comme
la fixture de scène l'est pour le dump de géométrie : une fiche qui a vieilli rougit là,
avec `./scripts/fiches.sh` pour remède.

### Le régime de fusion

Le vert des trois OS suffit pour tout, et la fusion depuis le téléphone est légitime,
y compris pour une PR qui touche `layout`, `gabarit`, `audit`, `pipeline`, `prevol` ou
les seuils. (L'ancien régime 1 — `check.sh` local avant fusion pour ces fichiers — est
aboli depuis que le gate est portable, 27/08.)

## La vague en cours

**Les chantiers A à L du test à l'écran sont fusionnés (#55 à #77), avec #41 et #52.**
Ce qu'ils ont posé, et qui se lit plus bas : la fiche d'une photo et son original
(F-s1), l'alerte de qualité (G-s1), les dates dans les légendes (H-s1), Envoi pour
Cloudprinter seul avec le dossier préparé et la couverture composée (K-s1), la sortie
d'un export bloqué par la résolution et le glisser depuis la réserve (L-s1), lopdf 0.42
(#41) et le passe-plat JPEG vérifié (#52). Le code de commande de K-s2 reste dans le
moteur et la CLI ; l'écran de K-s3 est retiré de l'app (S-s3a), et Commander revient par le
relais (S-s3), inerte dans un build de release publié.
Ce qui suit est l'état laissé par les vagues 2 à 6.

**2.6, la page qui tourne, est close — et la vague 2 avec elle (28/08).** Session 1
livrée le 27/08 : le geste au coin, le clavier, le mouvement réduit, le *fait quand*
tenu sur ses cinq points et mesuré. Les sept verdicts humains du bundle sont tombés le
28/08, tous favorables : la session 2 n'avait rien à coder, et **l'état actuel est
figé** — `DUREE_TOUR`/`DUREE_MIN` restent, une demande pendant un tour reste avalée,
pas de chantier tactile. **La lamelle est levée (1.2-c)** : la page se courbe, en rendu
maison, douze lamelles CSS 3D dont `feuille.ts::courbe` donne les angles — aucune
bibliothèque, donc aucun audit dû ; une bibliothèque de courbure qui entrerait un jour
passerait l'audit GPL-3.0 avant sa première ligne. Le geste vit dans la visionneuse,
**jamais dans l'éditeur** — la seule surface de lecture du projet, les autres vues étant une
grille, une table lumineuse et une revue photo par photo.

**La feuille**, et c'est le modèle du livre : elle porte au recto la page de droite de la
planche N, au verso la page de gauche de la planche N + 1 — donc un tour ne demande que
les deux planches qu'il joint. `feuille.ts` décide (faces, coin, relâchement, courbe),
`raster.ts` dessine chaque page une fois dans un canvas que personne ne monte,
`Feuilletage.tsx` assemble quatre morceaux par `drawImage`. **Ce qui bouge est une image
du PDF, jamais un redessin de la scène.** Trois règles à ne pas défaire : sans les images
des deux planches, aucune feuille ne se monte et le changement se fait sec ; une feuille
en vol garde la main jusqu'au bout, et un saut la retire ; `prefers-reduced-motion`
supprime le mouvement au lieu de le raccourcir.

**Une seule visionneuse (1.2-c)** : « Feuilleter » (barre de l'éditeur, menu, ⇧⌘P,
Envoi) est l'aperçu fidèle. Sur la couverture elle montre le **livre fermé** —
`Livre3D`, son avant découpé dans le PDF de couverture (`lecture.ts::decoupeAvant`) —
qui s'ouvre : il se met de face, puis son plat tourne autour du dos comme une feuille
(`feuilleDe(COUVERTURE, 1)`, verso = page gauche de la planche 1, à gauche la table), et
← depuis la planche 1 le referme. La **loupe** montre une page seule, tirée à sa taille
(`lecture.ts::largeurDeLoupe`, jamais un agrandissement CSS), flèches page par page,
Échap la referme (`Tourneur::fermerLoupe`) ; « Couverture à plat » est la loupe de la
couverture. La barre de la visionneuse rend le focus après un clic : le clavier d'App se
tait devant un bouton.

**La vague 3 est close** (3.1, 3.2 et 3.3 dans `main`). 3.1 a fait de `focal` un point
de l'image, invariant au ratio ; 3.2 est la bascule, qui en est le premier usage ; 3.3
retire le verdict de la reprise à travers une bascule (« non mesurable ») au lieu de
compter les replis machine comme des mains. **La vague 4 est close sauf 4.2**, différée
(compenser le papier sans épreuve imprimée serait deviner). **5.1 et 5.2 sont dans
`main`** (les sidecars Takeout et la photothèque, voir « Le moteur » et « La
photothèque du Mac ») ; **5.3 aussi** (le RAW, voir « Le moteur ») : la vague 5 est
codée en entier, son *fait quand* attend le parcours depuis Photos dans la fenêtre.

**6.1 est close, ses quatre sessions faites.** s1 : le moteur *lit* les polices du
système. s2 : il sait **sortir une face de son fichier** (`Face::extraire`, chirurgie
de table, jamais de glyphe) — les deux étaient de capacité, le PDF ne bougeait pas d'un
octet. s3 : **le composite**, où chaque octet de chaque PDF change et pas un pixel de
l'image. s4 : **la police voyage avec l'album** (voir « La police de l'album »).
Le sous-ensemblage a été reporté avec sa mesure, et il n'est entré dans aucune des
quatre.

**Les deux verrous de la vague 6 sont tournés (04/09)** : les objets libres se codent
contre la scène, sans prérequis canvas ; **la rotation est en V1**. La seconde va contre
la recommandation de la note, en connaissance de son coût, et elle a imposé l'ordre de
travail : l'angle entre dans la géométrie au premier objet stocké.

**6.2 est close, ses deux sessions faites** : s1 le modèle stocké, l'angle dans la scène,
l'émission PDF, le port TypeScript et sa parité ; s2 l'éditeur (voir « Les objets libres »
et « L'éditeur des objets libres »).

**6.3 s1 est close (06/09)** : l'ornement typographique entre dans le modèle et sort
dans le PDF (voir « Les ornements »). **6.3 s2 est close (09/09)**, en deux PR : le
compteur `ornement_sur_photo` avec `scene::recouvre` d'abord, puis le sélecteur, la
naissance hors des cases photo et l'attribution dans À propos (voir « L'ornement se
pose »). **6.3 est donc close aussi**, et les 32 actifs de Commons sont dans le pack
(#58) : 35 ornements, quatorze fleurons, treize filets, huit séparateurs.

**6.4 est close** : ce que les objets libres obligeaient partout ailleurs. Le linter
gagne **deux compteurs**, `objet_hors_marge` et `objet_deborde` (voir « Le linter et les
objets libres ») ; `--reprise` gagne la classe **`objet_libre`**, sans compteur parent,
parce que le Composer n'en propose jamais et qu'aucun compteur ne pouvait donc prévenir
la main ; le prévol gagne **deux bloquants**, `objet_coupe` et `objet_pli` ; et le schéma
d'`album.json` **monte à 3** (voir « Le schéma, et sa migration étagée »). Rien de neuf à
l'écran, aucun seuil existant touché, et les trois jeux de référence rendent les mêmes
verdicts qu'avant, compteur par compteur.


**L'intérieur sort page par page chez qui relie page par page (08/09).** La session B du
plan Cloudprinter : `core::imposition` découpe une planche composée en deux pages, à la
sortie et nulle part ailleurs (voir « L'imposition »). Cloudprinter passe en
`pages_simples`, Prodigi redevient livrable, et le prévol perd `planches_doubles` et gagne
`imposition`.

**Un seuil réglé pour taire une règle ne mesure rien (08/09).** La session C, qui clôt le
plan Cloudprinter. **`zone_sure` avertit au lieu de bloquer**, et c'est la mesure qui l'a
décidé : la ligne de base d'une légende vaut `bleed + marge × CAPTION_SAFE`, soit la
moitié de la marge du format — 7,00 mm sur cinq formats, 6,75 sur `portrait-20x25` —, donc
ni la raccourcir ni changer de gabarit ne la déplace, et le remède affiché était faux quoi
qu'on décide du reste. Bloquante, la règle refusait **tout album légendé** chez Prodigi et
Lulu, et le seul silence de Cloudprinter venait d'un `safe_mm` de 5 que rien chez eux
n'écrit. Molle, les trois vrais chiffres entrent : **7,0 / 10 / 12,7**, le premier étant
le bas de la fourchette de leur mail (« at least 7-10 mm »), et la fourchette voyage en
réserve. C'est la doctrine de 6.4 étendue d'un cran : le pli bute, la marge avertit. Le
vrai correctif est en amont — légende à `max(0,5 × marge, plancher)` — il déplace toutes
les légendes de tous les albums, donc c'est sa propre vague.

**Et le prévol voit enfin la couverture.** La fiche porte la **feuille à plat** quand
c'est nous qui la livrons (les trois cotes du cartonné vivaient dans le profil sans
atteindre personne) ; `couverture_resolution` mesure la photo de couverture sur la
surface de **ce** fournisseur — `cover::photo_rect_du_profil`, seule porte, parce qu'une
feuille cartonnée fait 239 × 258 mm là où un feuillet volant fait 216 × 216 — et
`dos_nu` prévient qu'un dos sous `SPINE_TEXT_MIN_MM` sortira sans titre. Mesuré sur
`corse-2013` : une 12 Mpx tombe à **241 ppi** en couverture cartonnée et passe chez
Prodigi, avec les mêmes pixels. Le verdict à trois est refait dans
`docs/mesures/2026-09-08-le-verdict-a-trois.json` : **Prodigi livrable**, Lulu toujours
dehors pour son CMJN, et plus un seul bloquant imputable à un profil chez Cloudprinter —
ce qui reste est le plancher de 250 ppi de l'album, que le Composer n'a jamais garanti.

**Le prévol regarde enfin les fichiers qu'on s'apprête à envoyer (08/09).** Il mesurait
l'album contre un profil et n'avait jamais ouvert le dossier : le 08/09 la couverture
posée pour la commande était le rendu `generique` — 426 × 216, sans dos, sans rempli —
pendant que la fiche annonçait 490,48 × 258, et le rapport disait `ok: true`.
**Quarante-quatre octets** séparaient les deux feuilles, sous le même nom, et ce qui a
rattrapé l'erreur est une empreinte notée à la main dans un journal. Deux règles,
`fichier_interieur` et `fichier_couverture` : **absent ne produit rien** — la plupart des
albums n'ont jamais été exportés et le prévol tourne dans l'app à tout moment —, présent
et faux **bloque**, avec les deux cotes dans le `cause` et la commande qui répare dans le
`remede`. Trois choses à ne pas défaire. **La lecture du disque vit dans `prevol()`**, qui
tient le `dir` ; elle entre dans `check()` en valeur (`FichiersPoses`, `Lecture::Pages` ou
`Lecture::Illisible`) comme `dims` y entre déjà, et `check()` reste pure — aucun test de
prévol n'a besoin d'un PDF sur le disque. **Le compte de pages attendu se calcule**, il ne
se lit pas dans `pages_fichier`, qui est le compte *déclaré à la commande* : sur 48
planches il vaut 96 chez Lulu pour un fichier qui en porte 48, et le brancher rougirait
sur un fichier juste. Et **la taille s'attend page par page**, parce que chez qui relie un
seul fichier les deux feuillets de couverture n'ont pas la taille d'une page d'intérieur.
La couverture ne se contrôle que chez `Fichiers::Deux` : ailleurs elle voyage *dans*
l'intérieur et `album-cover.pdf` est un sous-produit que personne n'envoie. Un fichier
illisible se dit et ne panique jamais — c'est la leçon de l'analyseur SVG de 6.3 s1.
**Ce que ça ne ferme pas** : un fichier juste de géométrie et vieux de contenu passe.
Pas de contrôle de fraîcheur, et surtout pas par `mtime` — un fichier plus vieux
qu'`album.json` n'est pas forcément faux, une géométrie se défend, une date de fichier
non. Le vrai remède est une empreinte du contenu écrite à l'export, et c'est le
manifeste ci-dessous.

**Un export dit de quel album il sort, et pour quel imprimeur (08/09).**
`core::export` pose **`export.json`** à côté d'`album.json` — une entrée par fichier
livré, avec son profil, l'empreinte de l'album au rendu, sa taille et sa date — et le
prévol gagne deux bloquants, **`fichier_profil`** (rendu pour un autre imprimeur) et
**`fichier_perime`** (l'album a changé depuis, ou le fichier a été remplacé). C'est
l'angle mort de #33 qui se ferme : une légende ne déplace aucun `MediaBox`, donc aucune
règle de géométrie ne pouvait voir ce fichier-là. **Un manifeste à côté de l'album,
jamais une empreinte dans le PDF** : la mettre dans `/ID` ou dans le XMP changerait les
octets de tous les PDF exportés, casserait le banc d'octets et forcerait une remesure
PDF/A-2b, alors que le prévol ne regarde jamais que des fichiers du dossier — un
manifeste posé là couvre toute sa portée pour une fraction du risque. **La preuve
inversée tient : `banc_octets_d_un_album_sans_objet_libre` rend toujours 214 814 o.**

Quatre choses à ne pas défaire. **L'empreinte neutralise `version` et `root`, et rien
d'autre** — l'estampille de schéma change à une migration sans que le livre change, et
`root` est l'endroit où vivent les photos, pas ce que montre le livre ; tout le reste
entre, parce que tout le reste s'imprime. **Un fichier sans entrée au manifeste ne dit
rien**, pas même un avertissement : sinon tout export antérieur deviendrait un bloquant,
à commencer par les deux fichiers vérifiés de `.albums/papier`, qui restent sans
manifeste et le resteront. **Pas de second algorithme de hachage** : `export::empreinte`
est le FNV-1a doublé que `pdfx` écrivait pour le `/ID`, sorti de son module et partagé
par les deux appelants — le workspace n'a aucun crate de hachage et n'en gagne pas un.
Et **on ne hache pas les photographies** : 95 Mo à chaque prévol paierait très cher un
cas rare, donc une photo réécrite hors de Colophon n'est pas vue, et un `root` repointé
sur un autre dossier aux mêmes noms non plus. `octets` attrape le reste pour le prix d'un
`stat`. **Deux appelants notent, par la même fonction `export::noter`** : la CLI
(`main.rs`, après `--print` et après `--cover`) dans le dossier de l'album, et
`export::preparer` dans le dossier préparé (K-s1, voir « Envoi, le dossier préparé et
la commande »). `export_pdf` de l'app, le PDF sans contrainte, écrit un seul fichier à
la destination choisie et ne pose aucun manifeste.

**Et l'aperçu de la couverture a cessé d'écrire sous le nom de la livraison
(08/09).** `render_cover_preview` posait `album-cover.pdf`, au profil de
l'aperçu : prévisualiser sous un profil, en changer, puis préflighter affichait
un bloquant sur un fichier que personne n'avait demandé à exporter — et c'est la
même collision de noms qui a fait partir une couverture `generique` pour la
commande. L'aperçu s'appelle désormais **`album-cover.apercu.pdf`**, à côté
d'`album.pdf` ; la livraison ne bouge pas (`--cover`, `pdfx.sh`, le mail à
l'imprimeur la nomment), et le prévol ne juge qu'elle. L'intérieur séparait déjà
les deux, la couverture était le seul endroit qui les confondait. **Deux tables
de noms, et elles se nomment l'une l'autre** : `lib.rs::nom_apercu` et le
middleware `/__dev/pdf` de `vite.config.ts` traduisent les deux mêmes mots pour
les deux lecteurs, le TypeScript ne connaissant jamais un nom de fichier
(`raster.ts::Quoi` vaut `"album" | "couverture"`). Un test tient les constantes
côté Rust, un autre prouve qu'un `album-cover.apercu.pdf` d'une géométrie
absurde ne lève rien. **Aucune suppression automatique** : un vieux
`album-cover.pdf` laissé par un aperçu d'avant reste jugé comme une livraison,
rien dans ses octets ne l'en distingue et le remède dit déjà de le retirer.

Vagues 0 et 1 closes. **Verdict de 2.5 : le défaut reste `dom`**, gravé dans `rendu.ts`
et `scripts/mesure-rendu.md`, dettes canvas au parking lot. Une bascule future resterait
un commit qui ne fait que ça. VoiceOver et le rang via le menu natif : entendus et
validés le 28/08 avec le questionnaire du bundle.

## Le moteur

Chaîne : scan (JPEG, PNG, **HEIC et RAW via ImageIO, macOS seulement**), analyse (dHash +
pHash DCT, netteté, exposition, visages), curation, Composer, PDF aperçu + 300 dpi +
couverture. Hors macOS, `heic::system()` rend `None` et `scan.rs` compte `skipped_heic`
et `skipped_raw` : rien ne casse, et l'écran nomme les deux.

**Le RAW passe par le décodeur système du HEIC, jamais par un crate** (5.3, `heic.rs`) :
les crates purs Rust sont tous LGPL-2.1 et portent un dématriçage que la plate-forme a
déjà ; l'audit du 03/09 n'a rien eu à auditer. Un RAW porte deux images, le capteur et
l'**aperçu** JPEG que le boîtier a rendu. Tout ce qui précède le tirage lit l'aperçu
(`heic::apercu_vignette` dans `thumb.rs`), et le tirage le pose s'il tient le plancher de
250 ppi pour sa case (`print::apercu_suffit`), sinon il demande le capteur, rendu par
Apple. **La définition se juge toujours sur le capteur** (`heic::dimensions`), donc le
linter et le prévol ne bougent pas. Un boîtier dont l'aperçu est sous 1600 px sur un
capteur qui ne l'est pas paie un décodage, une fois, dans le cache : sinon l'invariant de
`thumb.rs` (une vignette sous le plafond est la taille de l'original) mentirait à l'écran.
**Les métadonnées d'un RAW viennent d'ImageIO** (`heic::exif`, avant le crate exif dans
`meta.rs`) : le crate n'ouvre que les signatures TIFF strictes et les marques HEIF, donc
ORF, RW2, RAF et CR3 tombaient au mtime. Hors macOS le crate reste le chemin (NEF, CR2,
ARW, DNG lisent), mtime le dernier repli. Trois pièges mesurés le 03/09 :
`CreateImageAtIndex` est paresseux, le dématriçage court au dessin ; `IfAbsent` sans
plafond rend nul ; un plafond sans `IfAbsent` rend l'aperçu natif. Mesuré sur huit
boîtiers : aperçu 15 à 153 ms, dématriçage 0,5 à 2,2 s ; Sony et Panasonic embarquent un
aperçu de 1616 et 1920 px, Fuji et Olympus un aperçu plus petit que le capteur, Canon et
Nikon la pleine taille. `heic::decodages_pleins()` compte, et deux bancs le tiennent :
`banc_raw_du_mac` et `banc_raw_compose_sans_decodage_plein`, sur les huit RAW de
référence (CC0, raw.pixls.us) de `~/Pictures/colophon-testsets/raw/`, hors dépôt. Non
vérifié faute d'échantillon : un RAW en portrait (les huit sont droits), la parité
d'orientation entre l'aperçu et le capteur y est supposée.

**Le catalogue a un seuil d'entrée** : le banc (`scripts/banc-gabarits.sh`) a retenu 186
générés sur 1893 dans `gabarit::RETENUS` ; `offerts()` = historique + retenus (387 au
dump), le Composer reste sur l'historique, `spec()` reparse tout nom `g_*`.

**Les sidecars Takeout comblent ce que l'EXIF a tu** (`meta.rs`, 5.1) : liste fermée
d'orthographes — jamais un listage de dossier —, l'EXIF gagne toujours, la date de
sidecar est un epoch UTC assumé naïf et dite fiable, `(0,0)` refusé, `favorited` et
`geoDataExif` jamais lus. `-edited` gagne dans un Takeout seulement (la base porte un
sidecar) et l'originale entre dans `curation.json` en `originale_editee`, portée par le
relevé. Un cache de sondes par photo et par processus paie les dix appelants de
`meta::read` (chrono ×1,03 ; ×1,72 sans lui).

**`font.rs` lit les faces du monde, et n'en embarque toujours qu'une.** `Face::parse`
(données, index) rend nom PostScript, nom lisible, métriques, genre (`glyf`/`cff`),
`variable`, et un **verdict codé** — `illisible`, `embarquement_interdit`,
`bitmap_seulement`, `cmap_illisible` — parce que le refus s'affichera un jour avec sa
raison. Une face refusée **se nomme quand même** : seule l'illisible perd son nom.
`installed()` marche les dossiers de la plate-forme aux manières de `scan.rs` (symlinks
non suivis, cachés ignorés, tri par chemin puis rang) et rend **toutes** les faces,
refusées comprises. **Jamais un glyphe n'est lu** : `lire_tables` ne tire du disque que
les huit tables de `TABLES_LUES`, la présence de `glyf`/`CFF `/`CFF2` venant du
répertoire — 787 faces d'un Mac de série en 29 ms, 4,2 % des octets. `metrics()` passe
par le même lecteur, `text_width_mm` et l'aval ne bougent pas. Trois pièges mesurés le
28/08 : `CFF2` est du CFF (sinon neuf faces indiennes tombent), un refus bitmap se
décide sur les contours **avant** les tables communes (une police `bdat` n'a pas de
`head`), et le nom se choisit **en anglais d'abord** (sinon Times s'appelle
« Times 標準體 »). La règle bitmap est « aucune table de contour », jamais « une strike
présente » : cette dernière refuserait Courier New, Monaco, Geneva et Cochin.

**Une face s'ouvre pour écrire, et alors elle garde ses octets** (`font::Embarquee`).
`Face` lit un fichier et le lâche, ce que veut la découverte ; composer une ligne est le
besoin inverse — chaque caractère demande un glyphe au `cmap` et sa chasse à `hmtx`, et
le PDF réclame le fichier lui-même. **C'est le seul endroit où une chaîne devient des
glyphes** : `text_width_mm` et l'émetteur y passent tous les deux, sinon l'album serait
mesuré sur un jeu de chasses et dessiné sur un autre. La règle de substitution y vit
aussi : un caractère que la face ne dessine pas devient `?`, jamais la case vide du
`.notdef` ; une face qui ne dessine pas `?` le laisse tomber. **WinAnsi a quitté le
projet** avec `/Widths` : plus de table de chasses par code, plus d'échappement octal.

**La police de l'album voyage avec lui, et rien ne cherche jamais une police par son
nom** (6.1 s4). `album.police` est **additif** — le précédent est `reglages` : absent veut
dire « la face du projet », donc aucune migration et un vieil album s'ouvre tel quel. Le
schéma n'a pas bougé pour lui (il est monté à 3 en 6.4, pour une autre raison). Il porte
le **nom du fichier** posé à côté d'`album.json`, et ce nom n'a que deux valeurs
(`police.ttf` pour du `glyf`, `police.otf` pour du CFF) : un `album.json` est réparable à
la main, donc `dir.join(ce qu'il dit)` serait une traversée de chemin. Les octets posés
sont ceux de `Face::extraire`, **jamais le fichier système recopié** — Helvetica Neue
sort à 14 % de sa `.ttc`. `font::face_album(dir, fichier)` est la seule porte : elle rend
la face **et** `defaut`, le code d'un fichier nommé et introuvable. **Ce cas ne fait
jamais échouer un export** : l'album sort dans la face du projet et l'écran le dit, en
haut d'Envoi comme dans le panneau *Police*.

**Le dossier qui compte est celui d'`album.json`, jamais `album.root`** (les photos).
`PdfWriter::new(album, dir)` le prend, `print`, `cover` et `build` l'ont tous en main.
`Ecrivain` passe alors en `Cow::Owned`, et **toute coupure de ligne se mesure sur la
face du document** (`Ecrivain::largeur_mm`) : `cover.rs` pour le dos et la quatrième,
`Scene::of_avec` pour la page de garde, dont le titre rétrécit jusqu'à tenir — mesuré
dans une face et dessiné dans une autre, il sortirait du massicot. `Scene::of` garde la
face du projet : le linter, le prévol et le dump raisonnent sur une planche, pas sur un
rendu. **Une recomposition détruit tout champ que `BuildOptions` ne porte pas** :
`police` y est, comme `reglages` et `densite`.

**Ce que la face ne dessine pas se compte et se montre** : `Embarquee::absents` nomme les
caractères qu'`glyphes` imprimera « ? » ; le linter les compte par texte, couverture
comprise (`caractere_absent`, quatorzième compteur, sans seuil), dans la face de l'album
(`compteurs_avec_face`) ; la commande `police_absents` les rend à l'app, qui les dessine
« ? » (`font.ts::afficher`, `substituer` sur la scène de `SpreadView`, `measureMm` mesure
le texte substitué) et les liste dans *Format* et *Envoi*. Jamais un glyphe de repli à
l'écran pour un caractère que le papier n'aura pas.

**L'app mesure les octets de l'album, jamais une police installée** (`font.ts`). La
commande `police_octets` rend les octets que l'émetteur embarquera, `chargerFace` les
enregistre en `FontFace` sous la famille interne **`colophon-album`**, et `--font-book`
comme le canvas lisent cette pile. La parité est vraie **par construction** : mêmes
octets des deux côtés. Le crénage et les ligatures sont coupés des deux côtés
(`featureSettings`, `ctx.fontKerning`), le moteur n'en dessinant aucun. Mesuré le
29/08 : l'écart écran/moteur est de 1,6·10⁻⁵ mm pour une borne de 6,4·10⁻² mm — et
nommer la face **installée** avec le crénage du navigateur décale de 0,067 mm sur une
ligne de 74,8 mm. Le sélecteur vit dans le panneau *Police*, montre les faces refusées
grisées avec leur raison (`police.ts`, sur le modèle de `reasons.ts`), et le
« Regular » final s'élague **à l'écran** : le moteur rend ce que le fichier déclare.
Il offre **dix familles, une par voix** (`police::selection`, une liste ordonnée de
noms connus dont la première présente gagne), les 787 faces restant derrière
« toutes les polices » — cacher une police installée serait le défaut que ce panneau
existe pour éviter. **Chaque nom est écrit dans ses propres octets** (`specimen.ts`,
commande `police_apercu`, même extraction que le choix mais sans rien poser à côté
de l'album), crénage et ligatures coupés comme dans `font.ts` : un spécimen montre
ce que le livre imprimera. Deux plafonds, et ils sont la raison du compteur — 4 Mo
par face, 24 Mo en tout, au-delà de quoi le nom reste dans la police de l'interface :
une face CJK sortie de sa collection pèse 28,6 Mo et la liste en demande des dizaines.

**Les notes de l'utilisateur entrent dans le score** (`meta.rs`) : une photo rejetée sort
avant comparaison, une étoilée vaut ×1,18 par étoile. `album.json.bak` à chaque sauvegarde.


**Le Composer garantit** : jamais un portrait dans une case paysage (écart ≤ 1,4), les
visages à 4 % des bords au moins, jamais deux quasi-doublons sur une planche, une
ouverture au quartile haut, jamais quatre gabarits d'affilée. Plancher 250 ppi visé,
**pas garanti** : l'audit en tolère trois, le prévol les nomme et laisse l'auteur trancher
(1.1-c, « Exporter malgré la résolution »). **Une photo sous le plancher
même seule sur sa page n'est pas posée** (1.1-a) : aucune case du catalogue du Composer
ne la tiendrait, partager une planche la mettait dans une case pas plus petite (mesuré),
elle reste dans la réserve avec son signe, et le bilan le dit (1.1-a2 :
`Composer::trop_petites`, `BuildReport::trop_petites` et `chapitres_vides`, une ligne à
l'écran du bilan et dans la CLI ; un chapitre vidé n'invente aucune planche, son titre est
nommé). Sur les trois jeux × cinq formats, plus une
case sous le seuil (`docs/mesures/2026-10-19-badges-pleine-page.json`, banc
`qualite::banc_badges_pleine_page`) ; l'écran et le moteur y lisent les mêmes tailles.

**« Remplacer par… » classe pour la case** (`reserve::classer`, `cellule`) : chaque
candidate porte son ppi dans cette case au cadrage par défaut, celles qui la tiennent
passent devant dans l'ordre d'hier, les autres suivent avec le signe de la case à la
place de leur date (`reserve.ts::alerteDeCandidat`), et le plafond de douze tombe après
ce rangement. Sans case (« Ajouter une photo… »), la liste est celle d'hier à l'octet.

**« Remplir les cases vides » ne pose que ce qui tient** (1.2-a, `reserve::remplir`,
`cases_vides`, `remplir_avec` ; commande `remplir_cases_vides`, CLI cachée `--remplir`,
menu Planche et pied de la table). Une case vide est une case de gabarit sans photo (un
`album.json` réparé à la main) ou une **planche vide** (`vide`, sans texte ni objet) ;
la page blanche d'un `solo` est un dessin, pas un trou, et une planche figée ne se touche
pas. Chaque case prend la première photo de `reserve::ranger` (l'ordre de `classer` sans
plafond) qui tient 250 ppi **et** 1,4 d'aspect, parmi ce que la curation a sorti faute de
place seulement (`hors_budget`, `meme_moment` : `RECONDUITES`) — doublons, refus, parasites
et retraits à la main restent à la main ; une taille inconnue ne se pose pas.
Une planche vide prend le `solo` de la forme de sa photo (`layout::solo_pour_aspect`),
que la pose porte. Le moteur décide, `reserve.ts::appliquerRemplissage` pose en un
`apply` et revalide chaque pose (`edits.ts::poserDansCase`). Rien n'est créé.

**La bascule (`core::bascule`) n'est pas une recomposition.** `core::recomposition` rebâtit
tout ce qui n'est pas épinglé ; la bascule ne rebâtit rien — mêmes planches, même ordre,
mêmes photos, mêmes recadrages —, elle ne change que `trim_mm` et le `template` d'une
planche devenue inapte, et jamais vers une capacité plus basse (une photo perdue ne se
voit pas, une cellule trahie se voit et se change). Elle ne décode aucune photo : les
tailles viennent du relevé, sinon des en-têtes des originaux. Elle n'écrit rien côté app,
le moteur rendant un album que l'éditeur applique par son historique — d'où ⌘Z. Son bilan
nomme d'abord les photos passées sous 250 ppi, seul dégât qu'aucune main ne rattrape.
**Pas de « proposition à côté »** (décision 3.3) : ⌘Z côté app, `album.json.bak` côté CLI
et `album.<id>.json` pour les variantes occupent déjà les trois places — un quatrième
mécanisme serait une chose de plus à expliquer. Après une bascule, `--reprise` rend
`"non mesurable"` (champ `bascule`, les deux formats nommés) : les replis machine se
confondraient avec des mains, le verdict se retire, les faits restent, `ok` reste vrai.
**L'aptitude d'un gabarit a une seule définition**, `gabarit::apte` / `gabarit::trahison` ;
le sélecteur, le cycle clavier et la bascule la lisent.

Titres de chapitre depuis le GPS (`core::places`, GeoNames CC-BY) ; seul
`DateTimeOriginal` date un chapitre. **Deux rythmes** choisis au lancement, stockés dans
`album.json`, relus à la recomposition (`layout::Densite`). **Une composition donne trois
propositions** (`build::variantes_offertes`), en `album.<id>.json`, effacées à la fermeture
de l'album ou au ⌘S, jamais par l'écriture automatique. **Deux planches ordinaires peuvent encadrer le livre** (`album.colophon`,
cochables depuis Envoi) : colophon en queue, coché d'office ; page de garde en tête,
décochée d'office depuis 1.1-d ; hors chapitres.

## La photothèque du Mac

**Elle n'entre jamais dans le moteur : elle produit un dossier** (`app/photos.m`,
`app/photos.rs`, `app/bibliotheque.ts`). L'import écrit les photographies choisies dans
un dossier visible, et `build_album_from_folder` le compose comme n'importe quel dossier
du Finder. Ni `scan.rs`, ni `meta.rs`, ni la curation, ni le linter, ni le prévol
n'apprennent qu'Apple Photos existe, et le gate reste vert sur Ubuntu et Windows **sans
un `#[cfg]` de plus dans le moteur**. Ce choix se payait d'une copie ; la mesure du 02/09
l'a rendu gratuit : `writeDataForAssetResource:` rend le fichier d'origine **à l'octet**
et clone les blocs APFS, 219 Mo de photographies pour **2 Mo de disque réel**.

**Le pont est un fichier Objective-C compilé par `cc`**, la raison de `heic.rs` : les
liaisons à la main gardent l'arbre vide, et surtout les blocs restent dans la langue qui
les possède, repliés derrière un `dispatch_semaphore`. La frontière Rust ne voit que du C
plat et cinq fonctions. Toute la politique (ordre, noms, refus du réseau, annulation,
rapport) vit côté Rust.

**Le piège central : PhotoKit ne lève jamais pour un défaut d'accès, il rend une liste
vide.** Trois causes distinctes rendent cette même liste vide, d'où `Etat` à trois
branches et non un booléen : *pas encore demandé*, *autorisé mais bibliothèque système
injoignable*, *réellement vide*. Le deuxième est un état d'utilisateur réel, mesuré sur
cette machine : PhotoKit ne lit **que** la bibliothèque système (`SystemLibraryPath` de
`group.com.apple.photolibraryd.private`), pas celle qui est ouverte dans Photos.app, et
un renommage de photothèque laisse le chemin mort — autorisation accordée, échecs
`NSXPCConnection` en boucle, zéro album. La phrase de cet état nomme le chemin et la
case à cocher ; sans elle on cherche le défaut dans Colophon pendant une heure.
**Ce chemin n'explique qu'une liste vide** (`photos::classer`) : mesuré le 17/10, il nomme
encore un dossier supprimé alors que PhotoKit lit la bibliothèque désignée depuis, et sous
bac à sable il ne se lit pas ; des albums rendus l'emportent toujours. **Un booléen du pont
s'écrit `@YES`/`@NO`**, jamais `@(a && b)` ni `@(x == y)`, qui sont des entiers en C : un `0`
JSON faisait refuser toute la liste par serde, et aucun import n'aboutissait depuis la 5.2
(`booleen_du_pont` garde les deux formes). Un import qui lève se dit à l'écran (`tenterImport`).
`bibliotheque.test.ts` tient la distinction, et il mord (vérifié par mutation).

**Trois règles à ne pas défaire.** Le réseau est refusé au premier passage
(`networkAccessAllowed = NO`) : ce qui est resté dans iCloud se compte, se **nomme**, et
ne se télécharge pas sans un oui devant un chiffre. Toute requête porte une garde de
délai, parce qu'en signature ad-hoc — la nôtre — une requête PhotoKit peut ne jamais
rendre la main. Et les noms sont préfixés par le rang (`0001-IMG_2193.jpg`) : dans une
bibliothèque, `originalFilename` collisionne massivement, et un suffixe « (1) »
rejouerait le dégât que 5.1 répare. Le rapport `import.json` est posé à côté des photos,
et `scan.rs` ignore déjà `json`.

**`PHAccessLevelReadWrite` n'est pas un choix** : l'énumération n'offre que `AddOnly` et
`ReadWrite`, il n'existe aucun niveau lecture seule, et `AddOnly` ne lit rien. macOS
annoncera donc que Colophon peut modifier la photothèque, ce qui est faux ; la
`NSPhotoLibraryUsageDescription` de l'`Info.plist` est le seul endroit où la vérité se
rétablit, et c'est elle que l'utilisateur lit.

## Envoi, le dossier préparé et la commande

**Envoi parle à qui fait un album, pas à un imprimeur** (A2, `EnvoiView.tsx`, `envoi.ts`).
`EnvoiView` tient l'état, `EnvoiPage` le dessine et se rend sans navigateur
(`envoi-page.test.tsx`). Les défauts d'abord, puis **deux portes** en cartes radio
(`envoi.ts::portesOffertes`, `porteEffective`) : « Télécharger le PDF » et « Faire imprimer
par Colophon ». **Sans relais compilé, la seconde n'existe pas** et la première n'est pas
une carte : rien n'annonce la commande (test). Le bouton suit la porte
(`actionEnvoi(p, porte)`) : « Télécharger le PDF » prépare le dossier Cloudprinter dans
une boîte, ou le PDF libre sous « Un autre imprimeur ? » (profil `generique`, `export_pdf`,
sans manifeste) ; « Faire imprimer » est A1 ci-dessous. Le « à partir de » de la porte
vient de la dernière grille vue (`commande.ts::retenirPrix`, mémoire du processus) :
**jamais une intention créée pour afficher un prix**, jamais un chiffre en dur. À droite,
le livre en volume (`Livre3D.tsx`, A4, voir « Le livre en volume et son fond »), son
fond sous lui, format en cm, pages, papier tiré du profil (`papierEnClair`, jamais ses
codes), et « Feuilleter », qui est ⇧⌘P **ouvert sur la couverture** quand son aperçu
vient d'être rendu, sur la première planche sinon (`envoi.ts::ouvertureDuFeuilletage`,
`basculerFidele(true)`). **Liste noire** : fond perdu, zone sûre, PDF/X, sRGB, RVB, CMJN, ppi, dpi, output
intent, imposition, prévol, profil ne s'affichent que dans un repli — la fiche sous
« Pour les imprimeurs » (porte PDF seulement), faux-titre et colophon sous « Pages de
début et de fin » (ouvert d'office quand un défaut y renvoie). Prodigi et Lulu restent
dans `printer.rs` et la CLI (`--profil`). Cloudprinter relie de 24 à 800 pages. Au
harnais, `POST /__dev/relais/panne {"absent": true}` rejoue un build sans relais.

**« Préparer pour Cloudprinter » est `export::preparer`** (commande `preparer_dossier`,
CLI `--preparer`) : la boîte native choisit un dossier, l'app y crée
`<Titre> – Cloudprinter` avec `album-print.pdf`, `album-cover.pdf`, `export.json` (par
`export::noter`) et `fiche.txt`, puis relit le prévol **sur ces fichiers-là**
(`prevol::prevol_dossier(dir, fichiers, profil)`, dont `prevol(dir)` est le cas
`dir, dir` ; `check()` reste pure). Les codes de papier et de finition vivent dans
`PrinterProfile::commande` (`pageblock_150mcs`, `cover_130mcg`, `cover_finish_matte`),
jamais dans un `match` sur l'identifiant. MD5 et SHA-256 sont écrits à la main dans
`export`, testés contre les vecteurs de la RFC 1321 et du NIST : le workspace n'a pas de
crate de hachage et n'en gagne pas. Le chemin du dossier reste dans `AppState`
(`montrer_dossier_prepare`), le front n'en reçoit que le nom.

**Par la porte Colophon, le bouton s'appelle « Faire imprimer »** (A1,
`envoi.ts::actionEnvoi(p, "colophon")`) : `preparer_pour_commande(titre)`, le même
`export::preparer` sous Cloudprinter, **sans boîte ni Finder**, dans
`<données>/commandes/<Titre> – Cloudprinter` (`dossier_pour_commande`, qui vide
`commandes/` d'abord : une préparation à la fois, jamais hors du dossier de données).
`Preparation.commande` vaut alors vrai et `commandeDirecte` ouvre le parcours tout seul,
sans « Commander » (qui revient si l'on retourne au repos). Relais `injoignable` : la porte
reste et dit la panne, avec « Réessayer ». L'état du relais
est tenu par Envoi (`useRelais`) et passé à `CommandeView`. Au harnais,
`POST /__dev/preparer-commande`.

**Le Composer pose une couverture** quand `BuildOptions.cover` est vide
(`build::photo_de_couverture`) : la mieux notée du quartile haut, paysage ou carrée,
parmi les photos qui tiennent 250 ppi sur la feuille de couverture des quatre profils ;
titre de l'album, sous-titre `garde::ligne_des_dates`. Une recomposition garde la
couverture choisie. **`couverture_vide` bloque** chez `Fichiers::Deux` (Cloudprinter,
Lulu) : c'est la planche blanche du 21/09. La date du colophon passe par
`BuildOptions.aujourdhui`, `None` lisant `pdfx::stamp()` : plus aucun `set_var` dans
le dépôt.

**La fenêtre ne juge que l'album, et chaque défaut mène quelque part** (13/10). La
commande `preflight` appelle `prevol::prevol_album` (`FichiersPoses::default()`) : les
PDF restés à côté d'`album.json` ne se jugent plus que dans le dossier préparé
(`prevol_dossier`) ; la CLI garde `prevol()` et ses conseils à drapeaux, et
`--prevol --sans-fichiers` (caché) est ce que lance le harnais. Les règles sont une
liste fermée, `prevol::REGLES` (vérifiée en debug à chaque défaut, donnée à l'app par
`moteur.codes.json`) ; chacune a une destination dans `envoi.ts::DESTINATIONS` —
planche, couverture (choix de photo ouvert), Format, « Préparer à nouveau » sous le
dossier préparé, `ici` pour une page à décocher, `composition` pour le fond perdu — et
une phrase simple sans chiffre, `envoi.simple.<règle>` dans `i18n.ts`, FR et EN. La
phrase du moteur passe sous « Détails ». **`format_sans_produit` bloque** chez un
profil à `commande` quand `commande::produit_pour` ne connaît pas le format.

**Une photo sous 250 ppi a une sortie dans Envoi** (L-s1, `ResolutionLigne`,
`envoi.ts::actionsDefaut`) : Voir, Remplacer, Retirer de la planche. La barre ne bouge
pas. Retirer et Remplacer **enregistrent l'album**, parce que le prévol lit le disque ;
une ligne dont la case ne porte plus la photo du rapport se tait.

**Exporter malgré la résolution** (1.1-c). `resolution` et `couverture_resolution` sont
**à vous de voir** (`prevol::A_VOUS_DE_VOIR`, donnée à l'app par `moteur.codes.json`) :
`bloquant: false`, donc `ok` veut dire « l'imprimeur ne refuse rien », et `--prevol` comme
`--preparer` sortent en 0. Le rapport porte **`acquittement`** : une entrée par page du
livre (planche k : gauche 2k − 2, droite 2k − 1 ; couverture : 1), le ppi le plus bas
arrondi vers le bas, la page 0 écartée (`imposition` la refuse déjà). Envoi range les
défauts en trois groupes (`envoi.ts::groupesDefauts`) et ne grise que sur un refus
(`boutonEnvoi`). Porte PDF : un `Dialogue` qui nomme les pages, puis l'export. Porte
Booklibre : une case au libellé **versionné** (`envoi.acquittement*`, archivé mot pour mot
dans `docs/acquittement-v1.md`, `i18n.test.ts` les compare : changer un mot, c'est une
version 2), qui se décoche quand la liste change. **La fenêtre n'envoie qu'un booléen**
(`relais_creer(…, acquitte)`) : `Prepare` garde la liste relue sur le dossier préparé,
`relais::acquittement_pour` la met en `[[page, ppi]]` avec `version`, `seuil_ppi`
(`relais::SEUIL_PPI`, le `min_ppi` de Cloudprinter) et `coche_le` (UTC à la seconde, à
l'appel), refuse sans case ou au-delà de 200 pages avant le réseau. Jamais un nom de
photo : le relais le garde 180 jours, sans effet sur prix ni états.

### Commander, par le relais (S-s3)

**L'app ne tient aucune clé.** Le relais d'Alexis (`colophon-relais`, dépôt privé, un
Worker) devise, encaisse par Stripe Checkout et commande chez Cloudprinter. L'app ne
connaît qu'une adresse compilée, **`relais::RELAIS_URL`** =
`url_pour(cfg!(debug_assertions), SANDBOX_EN_RELEASE, REEL)` : le sandbox `workers.dev` en
debug, ou en release avec `COLOPHON_RELAIS_SANDBOX=1` (le produit final à tester, intentions
en mode sandbox, sinon `Stock::vue` les cacherait) ; en release sinon, **`REEL`**, lu par
`option_env!("COLOPHON_RELAIS_REEL")`, et rien sans lui ; les deux à la fois, ou une autre
valeur que `1`, arrêtent le build (`sandbox_valide`, `relais_unique`) (le gate tourne en debug, d'où la
fonction : son test tient aussi la valeur de la release). La CI publique ne la pose pas : la
release publiée reste inerte. Posée, elle doit être `https://` puis un hôte seul
(`reel_valide`, `const`) : un chemin, un port, des identifiants ou une requête font
**échouer le build** à l'évaluation de la constante, et changer la variable recompile le
moteur (mesuré le 12/10, `option_env!` entre dans les dépendances de cargo, pas de
`build.rs`). Sans adresse, `relais_pays` rend `null` sans un appel et Commander ne s'affiche
pas (jamais grisé) ; une adresse qui échoue n'est pas une absence, et la phrase
`commande.indisponible` avec « Réessayer » prend sa place (`commande.ts::relaisDe`,
`relaisIndisponible` ; au harnais, `POST /__dev/relais/panne`). Commander demande aussi un dossier préparé dans cette ouverture, vert, et
**un produit à ses cotes** (`Fiche::produit`, `commande::produit_pour`, Cloudprinter seul) :
un 28 × 21 ou une taille libre affichent `commande.format.invendable` à la place, et
`relais_creer` les refuse avant le réseau. **Une release contre le sandbox** :
`COLOPHON_RELAIS_SANDBOX=1 npm run tauri build -- --bundles app`, signée par
`APPLE_SIGNING_IDENTITY`. **Un bundle contre le relais réel** :
`COLOPHON_RELAIS_REEL=https://<hôte> npm run tauri build` dans `crates/colophon-app` (l'hôte
seul, sans `/` final), puis `install-app.sh` comme d'habitude.

**Le client vit dans le moteur** (`core::relais`), derrière son trait `Transport` (GET,
POST JSON avec `Authorization: Bearer`, PUT d'un fichier en flux), écrit sur le modèle de
`commande::Http`, qui ne sait poser ni en-tête ni GET et que la CLI garde. Les appels
partent de Rust, jamais du webview : la CSP `connect-src` n'a pas bougé. Une réponse non
2xx devient `Erreur::Relais { statut, erreur }`, le seul champ `erreur` du relais. Les
deux PDF montent par les URL PUT que la création rend, **le gros d'abord**, avec
`Content-Length` et les en-têtes signés qu'elle rend à côté. **Le relais n'est pas cru sur
parole** : seuls `content-md5` et `content-type` sont recopiés, et l'URL doit être en
`https` sur un hôte `*.r2.cloudflarestorage.com`, sans port ni identifiants, sinon rien
ne part. L'ETag relu est comparé au MD5. La progression sort par l'événement
`relais:progres`, en octets. Pas de `papier` dans la demande : la liste blanche du
relais le refuse, et `relais::PAPIER` refuse avant tout appel un dossier préparé pour
un autre papier.

**Le secret ne quitte pas Rust.** `commande.json` (dossier de données, `0600`, écriture
atomique, `src-tauri/src/commande.rs`) garde par intention identifiant, secret de
lecture, mode, date, état, dernier code et niveau choisi. **Toute écriture passe sous
`AppState.stock`** (`commande::modifier`) ; une relecture fait ses appels hors du verrou
puis relit le fichier sous lui et ne pose que l'état des intentions relues
(`commande::relire`), jamais un cliché, et jamais un état terminal défait. Debug et
release partagent ce fichier : la liste comme la relecture ne voient que le mode du relais
compilé (`RELAIS_MODE`), donc un secret ne part jamais vers le relais de l'autre mode. La fenêtre ne
reçoit que `IntentionVue` (identifiant, mode, état, dernier code, niveau) et la grille ; les URL de dépôt restent dans
`AppState.envoi`. `relais::Secret` rend `***` en `Debug` et `Display`, et tout texte venu
de dehors passe par son masque. Le rapport de Signaler ne cite jamais `commande.json`
(`le_rapport_ne_cite_jamais_commande_json`).

**Le paiement s'ouvre sans plugin opener** (condition 33) : `ouvrir_paiement` demande
l'URL au relais et la donne au navigateur du système par `navigateur`, la porte
d'`open_report_url`, après `url_de_paiement` — préfixe `https://checkout.stripe.com/`
exact, donc ni autre schéma, ni autre hôte, ni suffixe, ni port, ni identifiants, et un
jeu fermé de caractères. L'URL ne passe jamais par la fenêtre.

**Le parcours** (`CommandeView.tsx`, la machine `commande.ts::avancer`) : pays, grille
TTC par niveau, envoi à deux barres, confirmation en toutes lettres
(`phraseConfirmation`), navigateur, attente. Un seul pays sans région (`saute`) : la
grille se demande seule, une fois par étape, et la grille rappelle la livraison avec
« Changer » (événement `changer`) ; un échec du devis rend le choix du pays. Le bouton de
la grille, « Continuer vers le paiement · {prix} », enchaîne l'envoi et la confirmation.
Une ligne d'explication par étape (`commande.explique.*`), et le bandeau
`commande.sandbox.bandeau` dès qu'une offre est en sandbox. Ce qui bouge seul (`enAttente` :
`attente_paiement`, `payee`, et les deux états de remboursement et d'annulation) se
relit toutes les cinq secondes tant qu'Envoi est ouvert, tout le reste à chaque
ouverture. Les dix états du relais ont leur phrase (`phraseIntention`), un état inconnu
une phrase neutre. La liste offre « Payer » (niveau gardé) et « Abandonner » avant tout
paiement. **Abandonner demande au relais** (`POST /abandon`, `relais::Relais::abandonner`,
S-s4b) : le relais expire chez Stripe toute session Checkout ouverte, puis abandonne.
`commande::abandonner` n'oublie que sur son 200 (`abandonnee`, `purgee`) ou son 404, dans
la fermeture de `modifier` et seulement si l'état local n'a pas bougé depuis l'appel ; sur
409 `avancee` l'intention reste avec l'état relu, sur 502, une panne ou une réponse hors
contrat elle reste telle quelle. **Aucune horloge dans la décision** : ni durée de session
ni date d'ouverture côté app. Refusé pendant l'envoi des fichiers (`abandon_envoi_en_cours`)
et pendant un autre abandon de la même intention (`abandon_en_cours`, marqueur
`AppState.abandons`, libéré par `Drop`). Un paiement rouvert après l'abandon reçoit le 409
`etat` du relais, refus définitif. `abandonnee` est terminal. L'attente se referme par « Fermer » sans rien oublier ; un refus définitif du paiement
(`devis_expire`, `intention_expiree`, `trop_de_sessions`) mène à une étape d'erreur
d'où l'on recommence au pays ; un second envoi de la même intention rend
`envoi_en_cours` et l'écran l'ignore. « Commander » ne s'offre que sur un dossier préparé **dans cette
ouverture** d'Envoi : la vue retient la préparation trouvée à son montage. Au harnais,
`/__dev/relais/*` rejoue le relais en mémoire, `POST /__dev/relais/suivant` force
l'état suivant, et `POST /__dev/relais/abandon {"reponse": 200|404|409|502}` la réponse du
prochain abandon. Banc : `banc_relais_sandbox` (`#[ignore]`, en debug, aucune clé),
`docs/mesures/2026-10-09-commander-sandbox.json`.

**Le parcours se montre en six étapes** (A3) : *Votre livre · Livraison · Prix · Fichiers
· Paiement · Suivi*, un bandeau en tête. **La machine ne change pas d'états** :
`commande.ts::etapeVisible(parcours, { preparation, suivi })` en est l'habit, et
`etapesDuParcours` rend passées, courante, futures, avec le seul retour que `avancer`
accepte — la Livraison depuis la grille ; les fichiers partis, rien ne revient en
arrière. « Votre livre » est l'écran de la préparation (`progresExport` d'App, page par
page) ; « Suivi » tient tant qu'un paiement vient d'aboutir (état local `suivi`,
« Fermer » le lâche). Chaque commande est une carte : `referenceCourte` (huit signes,
l'identifiant entier au survol), la date d'`IntentionVue`, « test » en sandbox, la frise
`JALONS` placée par `jalonsAtteints` — **« Livrée » n'est jamais atteinte**, aucun état ne
la dit ; un état inconnu ou hors du chemin ne pose aucun jalon. **À droite du parcours,
`Rassurance`** : chaque phrase est un fait du relais, et une phrase qu'on ne prouve plus
s'en va. Stripe Checkout encaisse (le relais ne voit qu'une session) ; rien n'est débité
sans session payée ; les fichiers montent en `https` et le balayage quotidien retire
les objets dès l'état 100, à trente jours au plus tard ; l'annulation rembourse sous
l'état 30 ; « en France » seulement quand le relais n'ouvre qu'elle (`livreEnFrance`).
Rien sur le suivi du colis (le niveau `cp_postal` n'en promet pas) ni sur un délai que la
grille ne rend pas. **Et aucun lien vers Signaler** (A4) : il ouvre une issue GitHub
publique, quelqu'un y citerait sa commande ; un contact commande attend le réel.

### Le prix pendant la construction (1.1-f)

**« À partir de 46 € livré » dans le pied du livre et des planches** (`ChoixImpression.tsx::PrixLivre`),
pour `2 × planches` pages relevées au minimum du produit, lu dans une table que publie un Worker
à part, `colophon-tarifs` (dépôt du relais, `wrangler.tarifs.toml`), rempli par cron quotidien :
un devis par nombre pair de pages, parce que Cloudprinter change de site en chemin (mesure du
20/10). **L'app lit une case et ne recalcule rien** (`impression.ts::prixPour`). Rien sans table,
sans pays livré, pour un format sans produit, au-delà de 200 pages ou d'un instantané de plus de
sept jours (`core::tarifs`, et le Worker le refuse aussi) : jamais un chiffre en dur.

**L'appel ne part qu'après « Par Booklibre »** (revue 1.1-c-f, C59) : la question « Comment
l'imprimer ? » (`QuestionAccueil`, sur l'accueil, puis les Préférences) est rangée dans
`colophon.impression` sur le modèle de `maj.ts`, et `impression.ts::appelPermis` décide. Une
lecture par ouverture, en mémoire. « Moi-même » oublie la table. Le pays vient de la table
(noms par `Intl.DisplayNames`), « Un autre pays » dit « pas encore livré » et continue ; il ne
part jamais. **Jamais dans la version App Store** : `tarifs_offerts` et `tarifs_lire` y sont
coupés par `cfg`, le front ne pose pas la question, `mas.sh` refuse un binaire qui porte
`colophon-tarifs` ou `/v1/prices`. **L'adresse est compilée comme celle du relais**
(`tarifs::url_pour`) : le sandbox en debug ou avec `COLOPHON_RELAIS_SANDBOX=1`, sinon
`COLOPHON_TARIFS_REEL` (hôte seul, `reel_valide`), posé par `release.yml` depuis la variable du
dépôt du même nom, seulement si elle l'est. Sans adresse, ni question ni prix. Un GET nu par
`relais::Transport`, sans bearer ni en-tête propre. Au harnais, `/__dev/relais/tarifs` rend une
table fixe.

### Le livre en volume et son fond (A4)

**`cover.fond`, un `#rrggbb` additif comme `album.police`, sans montée de schéma** : une
version plus ancienne l'ignore et le perd à l'enregistrement, la couverture retombe en
blanc. Il se lit par `Cover::fond_rgb` et rien d'autre ; une valeur hors grammaire vaut
« pas de fond », se signale au journal (à l'ouverture dans `load_album`, au rendu) et
n'empêche ni l'ouverture ni l'export. **L'aplat couvre la feuille entière** — fond perdu,
rempli, mors et dos compris, une seule opération en tête du flux, arrondie au centième de
point par excès — sur la feuille à plat comme sur les feuillets d'un fournisseur à un
fichier. **Sans fond, le PDF de couverture est celui d'avant à l'octet**
(`sans_fond_la_feuille_est_celle_d_avant_a_l_octet`, empreinte relevée sur `main`). Le
texte posé à même la feuille (titre sans photo, dos, quatrième) prend
`couleur::encre_lisible` : l'encre du livre tant qu'elle tient 4,5:1 en luminance WCAG,
sinon la plus contrastée d'elle et du blanc papier ; le texte sur la photo garde le blanc
et son ombre dure. `couleur.ts::encreSurFond` en est le miroir, `PAPIER` relu dans
`cover.rs` par le test. Mesures : `docs/mesures/2026-10-18-couleur-couverture.json`.

**`Livre3D.tsx` est en CSS 3D, sans dépendance** : six faces autour d'un centre, cotes de
`volume.ts::cotesLivre` (format et `fiche.dos_mm` à une seule échelle), avant =
`MiniCouverture`, dos et arrière à l'aplat du fond avec l'encre de `encreSurFond`, bloc de
pages blanc cassé rayé. Glisser (souris, trackpad, balayage horizontal à deux doigts) et
flèches au focus ; relâché, retour au trois-quarts par le chemin court (`repos`). Sous
`prefers-reduced-motion`, ni glisser ni transition : trois vues fixes au clic
(`VUES_FIXES`). `role="img"`, nom = format, pages, fond. Le titre de la vignette sur la
photo est un objet imprimé : blanc papier et ombre dure quel que soit le thème. Le dos se
lit de bas en haut, comme le PDF — l'éditeur de couverture le montrait de haut en bas
jusqu'à A4.

**`PaletteFond.tsx`**, sous le livre dans Envoi et dans l'éditeur de couverture (avec un
petit `Livre3D`) : blanc papier (`null`), crème `#f3ead8`, noir `#1a1a1a`, puis jusqu'à cinq
teintes de la photo (`fond.ts::teintesDe` : histogramme 4 bits par canal, centres
déterministes — la case la plus lourde puis la plus éloignée —, Lloyd pondéré, saturation
HSL plafonnée à `SATURATION_MAX`), puis « Autre… » qui ouvre le sélecteur natif, validé à
`change`. Un choix est un pas d'annulation (`edits.ts::setFond`, `null` retire le champ) ;
dans Envoi il s'enregistre aussi (`appliquerDepuisEnvoi`), le dossier préparé devenant
périmé par l'empreinte. Au harnais, `POST /__dev/couverture?profil=` rend l'aperçu de
couverture par `--cover --apercu` (caché), que `renderCoverPreview` appelle hors Tauri.

### La quatrième (1.2-d)

**Trois champs additifs sur `Cover`, sans montée de schéma, comme `fond`** :
`back_photo` (`PhotoQuatrieme { photo: Slot, position, taille }`, `haut`/`centre`/`bas`
× `petite`/`moyenne`/`grande`), `back_police` (une face `objet-*`, posée par
`poser_police_objet`, comptée par `elaguer_polices_objet`, `police_absents` et
`caractere_absent`), `back_taille_pt` (absente : `BACK_TEXT_PT_AT_210 × w/210`, bornée à
[6, 48]). **Une seule mise en page, `cover::quatrieme`** : le cadre carré dans la boîte
intérieure (`TITLE_INSET`), la bande que le texte prend (sous la photo, au-dessus si elle
est en bas, une interligne à l'écart), les lignes par `scene::replier` dans la face de la
quatrième (`Ecrivain::largeur_mm_dans`), et `coupees` : **plus aucune ligne jetée en
silence**, le prévol les compte. Sans les trois champs, la feuille est celle d'avant à
l'octet (`EMPREINTE_SANS_FOND`). `quatrieme.ts` en est le port, sous parité par
`quatrieme_samples` ; `QuatriemeView.tsx` dessine les lignes du port dans un SVG au repère
du panneau, **à la taille d'impression** (règle des blocs libres), et le champ s'ouvre au
clic, à la même taille par `cqw`. Le prévol gagne `quatrieme_deborde` (bloque : une ligne
sort du panneau, monte sur la photo ou ne s'imprime pas), `quatrieme_zone_sure` (avertit)
et `quatrieme_resolution` (bloque), mesurés par `check_mesure` dans la face que
`prevol()` lit sur le disque ; `check()` mesure dans la face du projet. **L'encre se lit** :
`scripts/couverture-encre.py --avec-mutant`, dans le bloc macOS de `check.sh`.

**Le moteur et la CLI gardent CloudCore et S3.** `core::commande` parle à CloudCore
(`devis`, `commander`, `etat`, `annuler`) derrière le trait `Http`, `Ureq` en vrai
(rustls, 60 s), un faux en test ; `core::depot` pose les deux PDF sur un seau S3 par PUT
présigné (`rusty-s3`, URL en chemin), lisible sept jours, clés d'objet limitées à
`colophon/<référence>/album-print.pdf` et `album-cover.pdf`. Le moteur reste synchrone,
sans tokio ; les deux crates sont passés à l'audit de licence avant leur première ligne.
Seul `--devis` les appelle depuis la CLI ; le dépôt et la commande ne tournent qu'au banc.
`SECURITY.md` le dit : l'app fait un appel, la vérification de mise à jour, et la CLI
joint `api.cloudprinter.com` avec la clé de qui la lance.

Cinq choses à ne pas défaire. **Le secret ne fuit pas** : `Cle` et `Acces` rendent `***`
en `Debug` et `Display`, sans `Serialize`, la clé n'entre que dans le corps par
`commande::appeler`, et tout texte venu de dehors passe par un masque. **Le mode sandbox
ou réel est déclaré**, CloudCore ne le dit jamais (seul l'état 501 le trahit). **Le devis
et la commande nomment l'article pareil** (`commande::ARTICLE`), et `orders/info` met le
numéro d'état dans `state`, pas dans `state_code`. **Le papier est figé** sur celui du
dossier préparé : `passer` rend `PapierRefuse` avant tout dépôt. **Les objets se
retirent** à 100, 500, 501 ou trente jours (`depot::a_retirer`), et une erreur à
mi-chemin retire ce qui a été posé. Un produit par format (`commande::produit_pour`) :
tous sauf `paysage-28x21`, qui passe par Quick order. Banc : `banc_commande_sandbox`
(`#[ignore]`, `CLOUDPRINTER_SANDBOX_API_KEY` et `R2_*`).

**lopdf est en 0.42** (#41, RUSTSEC-2026-0187) : la seule lecture de PDF en production
est celle du prévol, sur des fichiers d'un dossier qui se partage, et un débordement de
pile n'est pas une erreur qu'on rattrape.

## L'alerte de qualité, la fiche et les dates

**Trois signes, un par cause, au coin de la case rognée** (B1, `photos.ts::badgesDe`,
`SignesAlerte.tsx`, `icons.tsx::SignePixels`/`SigneLune`/`SigneFlou`) : quatre carrés sur
pastille rouge (`--chip-danger`) pour `sous_resolution`, seule qui bloque l'impression ;
une lune et deux cercles sur pastille neutre (`--chip-fond`) pour `sombre` et `floue`. La
plus grave à gauche (`qualite.ts::GRAVITE`). Une bulle au survol et au focus de la case
(`SceneProxies.onFocusCase`) dit la phrase simple sans chiffre (`phraseDAlerte`), le ppi
en petit dessous (`detailDAlerte`) ; chaque signe porte `role="img"` et sa phrase. **La
légende du pied** (`LegendeQualite.tsx`, Livre et Planches) compte les cases par cause,
chaque compte mène à la première planche concernée puis à la suivante
(`qualite.ts::plancheSuivante`), et se tait quand tout est bon : Colophon ne note pas
les photos. `sous_resolution` sur la taille de l'**original**, pas de la vignette,
`sombre` sur la vignette au travers du réglage, `floue` dans le dernier décile de
netteté du dossier (`qualite::seuil_de_flou`, `PART_FLOUE`, rien sous `PETIT_DOSSIER`).
**Le décile reste un quota, et c'est mesuré** : un seuil absolu de netteté a été cherché
(B2, `qualite::tests::banc_seuil_de_flou`,
`docs/mesures/2026-10-17-seuil-de-flou.json`) et n'existe pas sur cette mesure — le haut
des vignettes floutées σ = 3 vaut 936, le 2ᵉ centile des originales 63 : la variance du
laplacien à 256 px dit la texture de la scène avant le flou. **Sans relevé, aucune
alerte.** La mesure vient de `qualite::releve_album` (le `releve.json`, sinon les
vignettes et les en-têtes, au bit de `analyze`), une fois par album. `SceneProxies`
reçoit les alertes en codes et les ajoute au nom : il reste une fonction de la scène. Le
Tri et Planches reprennent les signes et la bulle. Aucun seuil de l'audit n'a bougé.

**La fiche d'une photo** (F-s1, `core::fiche`, `FichePhoto.tsx`, ⌘I) lit l'EXIF étendu de
`meta.rs` et la mesure par `qualite::mesure_photo`. « Voir l'original » passe par
`fiche::a_reveler` (nom validé par `thumb::nom_propre`, chemins canonisés), puis
`open -R`, `explorer /select,` en `raw_arg` ou `xdg-open` : jamais un shell.

**Les dates des légendes** (H-s1) : `legende::date_de` est la seule formulation, la
proposition de case et le bouton l'appellent tous les deux ; `dates_fiables` en fait la
table, `edits.ts::daterLegendes` et `dedaterLegendes` sont idempotents et un seul pas
d'annulation.

**Dates et lieux sous les photos** (1.2-b) : `album.legendes` (contenu `date`, `lieu`,
`date_lieu`, forme `jour`/`court`/`mois`, langue retenue au choix), additif, absent = rien.
Les mots sont la **mention** de chaque case, `Slot::mention`, à part de `caption` : « Rien »
retire tout sans toucher une lettre tapée, et `reprise` ne la voit pas. Une seule source,
`legende::mention` (date si `taken_reliable`, ville si `places::nearest`, jamais rien
d'inventé) ; `poser_mentions` l'écrit à la composition et à la recomposition
(`BuildOptions.legendes`), la commande `mentions_photos` rend la table que `mentions.ts`
pose, et `apply` la repose après toute édition (une photo remplacée prend ses mots, jamais
ceux d'une autre ; une table d'un autre réglage ne s'applique pas). La scène imprime
`caption · mention` par `scene::legende_imprimee` / `scene.ts::legendeImprimee`, sans
répéter une date déjà posée par « Dater », et **sans mention sous une case dont la ligne de
base tomberait sous la bande sûre** (une pleine page). `zone_sure` mesure chaque légende de
photo à sa ligne de base. Le réglage : Planche → « Dates et lieux… » (`LegendesView.tsx`)
et une case décochée à la création ; CLI `--legendes`.

**Le glisser depuis la réserve** annonce une copie (`reserve.ts::effetDeDepot`), l'échange
de deux cases un déplacement, et `dragDropEnabled` est coupé sur la fenêtre. Le
portrait 20 × 25 mesure **203,2 mm** (`format.rs`), le vrai 8 × 10.

## La scène, source unique de ce que porte une planche

`Scene::of(planche, géométrie)` rend des objets : rectangle, **angle**, profondeur
(l'indice), rang de lecture, rôle codé (`photo`, `photo_caption`, `chapter_caption`,
`text`, `free_text`). L'émetteur, le tirage 300 dpi, le linter, le prévol **et l'écran**
la lisent tous. **Dérivée sauf les objets libres** : tout ce qu'un gabarit produit se
reconstruit de `album.json`, et les objets libres, que rien ne peut produire, y sont
stockés. `garde`, `texte` et
`colophon` sont un rôle `Text` aux lignes déjà mises en page. Un texte est placé par sa
ligne de base (`at`) et couvre son encre mesurée (`rect`), deux choses distinctes, et
`caption_box` reste le proxy de placement du dump. **Pour un gabarit-candidat, c'est
toujours `slots_for`** ; pour ce que porte une planche, c'est la scène.

`scene.ts::sceneOf` en est le miroir TypeScript, millimètres, origine en haut à gauche.
**Deux rendus consomment la même scène**, DOM et `SceneCanvas`, derrière `rendu.ts`,
défaut `dom`. Les quatre gestes passent par `scene.ts::hitTest` ; un recadrage en vol se
substitue **dans la scène** (`avecRecadrage`), donc aucun rendu ne sait ce qu'est un
brouillon. **`SceneProxies` pose une boîte focusable par objet**, dans l'ordre de lecture,
nommée depuis le rôle via `i18n.ts`, `pointer-events: none`, clippée au rognage. **Les
deux rendus donnent le même arbre d'accessibilité** (37 objets nommés, listes identiques
sur neuf planches) : ce qu'un rendu peint est du décor, marqué objet par objet, jamais un
`aria-hidden` sur le conteneur, deux champs de saisie y vivent.

**La géométrie a une seule source** (`gabarit.rs`, dump lu par le TS via `geometrie.ts`).
Restent portés et sous parité : fenêtres de recadrage, `coverSheet`, `gardeLayout`, et
l'assemblage de la scène, épinglé objet par objet sur neuf planches × six formats. Seule
l'encre mesurée échappe, faute de fonte sous Vitest, et court la mesure synthétique des
deux côtés. **Les deux fixtures se régénèrent** quand le dump ou la scène change
(`--dump-geometry`, `scripts/fixture-scene.sh`) : leur fraîcheur est un test.

### Les objets libres, et l'angle qu'ils ont apporté

**Le premier champ stocké au-delà d'un gabarit et de ses cases** : `Spread::objets`
(`model.rs`), additif, absent quand vide — **le schéma est monté à 3 en 6.4**, et pas
pour lui : un champ additif ne demande aucune conversion, et 3 n'achète que le refus
(voir « Le schéma, et sa migration étagée »). Un objet porte sa boîte, son angle et son
contenu (`Contenu::Texte` ou `Contenu::Ornement`, le tag étant dans le fichier
depuis le premier objet libre — un ornement n'a donc coûté aucune migration). **Leur ordre est leur profondeur**, et ils passent tous au-dessus de ce que le
gabarit a produit.

**La rotation est en V1** (décision d'Alexis du 04/09, contre la recommandation de la
note ; `plans/2026-08-28-note-decision-vague-6.md` hors dépôt). Elle est donc entrée dans
la géométrie **au premier objet stocké**, pas après : `distance_to_trim` prend l'angle,
`hitTest` aussi, l'émetteur pose une matrice. **Un angle et une origine, jamais une
matrice arbitraire** — tout ce qui mesure un objet doit pouvoir en retrouver les quatre
coins, et une matrice ferait entrer un cisaillement par la même porte.

Quatre choses à ne pas défaire :

- **`corners` rend le rectangle tel quel à l'angle zéro.** Ce n'est pas une optimisation :
  `(x + w/2) − w/2` n'est pas `x` en flottant binaire, et un objet droit doit mesurer
  exactement ce qu'il mesurait avant que l'angle existe. C'est là-dessus que repose
  l'égalité **au bit** de `distance_to_trim` sur les 10 000 cases du balayage de parité.
- **`Td` reste pour le texte droit, `Tm` n'arrive qu'au-dessus de zéro.** Mesuré : un
  album sans objet libre rend le PDF de `main` à l'octet (214 814 octets, banc
  `banc_octets_d_un_album_sans_objet_libre`, à lancer sur les deux arbres).
- **Un bloc revient à la ligne dans sa boîte**, aux mots, dans la face de l'album. Rien
  n'est replié en silence : ce qui dépasse dépasse **en hauteur** (`overflow`), et un mot
  plus large que la boîte est signalé (`trop_large`), jamais coupé — couper un mot est
  une décision sur une langue.
- **`decalage` et `replier` sont écrits une fois** et appelés par l'assembleur, le dump et
  le port. Une ligne que l'émetteur centrerait et qu'un rendu ne centrerait pas est
  précisément la classe de bug que ce module existe pour rendre impossible.

**Ce que la parité peut et ne peut pas dire.** La fixture de scène épingle ce qu'aucune
mesure ne bouge : la boîte, l'angle, l'index, l'alignement, l'ancre. Où les lignes se
coupent dépend de la face, et Vitest n'en a pas — donc c'est `libre_samples` du dump de
géométrie qui tient le repli, l'alignement et les décalages, rejoués des deux côtés sous
la mesure synthétique. **Et dans cette boucle-là, la taille se convertit avec le `PT_MM`
du harnais**, pas avec le 25,4 ÷ 72 du moteur : y verser la conversion exacte fait dériver
les deux côtés d'un millionième (7,9 contre 7,900008, mesuré). En production `scene.ts`
convertit bien avec `album.ts::PT_MM`, qui est celui du moteur.

**Un objet tourné garde son proxy d'accessibilité entier**, tourné avec lui, au lieu
d'être ramené au rognage : rogner un rectangle tourné donnerait un polygone, et ce qui
justifiait le rognage — le fond perdu — ne concerne que ce que les gabarits produisent,
qui est toujours droit.

### Les ornements (6.3 s1)

**Un ornement est un objet libre de plus, et rien d'autre n'a bougé** : il se pose, se
glisse, se tourne et se supprime avec le code de 6.2, sans une ligne de geste réécrite.
`Contenu::Ornement { pack, id }` **ne porte que l'identité** — pas de couleur, pas
d'échelle, pas de miroir : la boîte tient la taille, l'angle est déjà là, et un champ de
plus serait une seconde source de vérité pour une géométrie qu'`Objet` détient.
`Role::Ornement { index, pack, id }` fait pareil côté scène : **la scène ne connaît pas
les chemins**, le dessin est affaire de rendu, et la parité Rust/TypeScript porte donc
sur la boîte, l'angle et l'identité, pas sur des milliers de coordonnées que les deux
côtés recalculeraient chacun de son côté.

**Le mot « clipart » a quitté le projet.** `CONTRIBUTING.md` porte désormais la frontière
publiquement : stickers, masques, fonds décoratifs et cadres fantaisie restent refusés,
l'ornement typographique entre sous son nom. `CONTEXT.md` le définit, et l'interdit comme
synonyme.

**La boîte d'un ornement garde le rapport de son `viewBox`** (`Dessin::rapport`,
`ornement.ts::rapport`), et c'est la décision dont tout le reste découle : la boîte **est**
l'encre, donc « le rectangle d'un objet libre est sa boîte » reste vrai sans exception, et
le pli comme la coupe mesurent exactement ce qui s'imprime. Le redimensionnement est donc
proportionnel — `tailler` prend un rapport optionnel, et **le plancher entre dans le même
maximum**, sinon il écraserait la forme au moment précis où la boîte devient petite.
Mesuré à l'écran le 06/09 : 0,7058 après un glissement de coin, soit 12/17 à la
quatrième décimale.

**Le pack est de la donnée compilée, jamais un dossier de l'utilisateur** :
`assets/ornements/pack.toml` plus un `.svg` par actif. Contribuer, c'est une entrée et un
fichier, sans une ligne de Rust — **ni de TypeScript** : l'app lit le pack du moteur
(`--dump-ornements`, commande `ornements`, `/__dev/ornements`), donc une seconde liste ne
peut pas dériver puisqu'il n'y en a pas. Trois refus tenus par des tests : licence hors de
`{CC0-1.0, PD}`, champ manquant, `.svg` hors sous-ensemble. Le manifeste **refuse au lieu
d'ignorer** toute ligne qu'il ne comprend pas : un lecteur tolérant est celui qui laisse
passer une faute de frappe dans une licence.

**Le sous-ensemble SVG est `<svg viewBox>`, `<path d>`, `M m L l H h V v C c Z z`,
`fill-rule`, et rien d'autre** — le domaine où la traduction en opérateurs PDF est
littérale. Le refus se prononce **à la fabrication du pack**, jamais au chargement :
personne ne dépose de SVG dans un album, donc un message à l'écran serait un message que
personne ne devrait lire. `lire_svg` rend un dessin **normalisé** — absolu, droites et
cubiques —, si bien qu'aucun rendu n'interprète de SVG : le PDF émet les opérateurs, et
`ornement.ts::attributD` écrit le tracé **une fois** pour le `<path>` du DOM et le
`Path2D` du canvas. Trois pièges mesurés le 06/09 : une paire de coordonnées après un `M`
est un `L` (deux des trois actifs livrés en portent), `c0,0.931-33.845,3.722` porte six
nombres où le signe tient lieu de séparateur, et un `Z` ramène le point courant au départ
du sous-chemin — sans quoi le dessin dérive sans qu'aucune commande soit fausse.

**Un analyseur écrit à la main refuse ou casse là où personne ne regarde**, et quatre
défauts l'ont montré, tous trouvés en relecture et aucun par un actif du pack. Un fichier
qui s'arrête sur son `=` **paniquait** ; un nombre après un `Z` **bouclait à l'infini** ;
un commentaire sauté jusqu'au premier `>` venu faisait **revenir à l'encre un chemin
commenté** (`<!-- ancien <path/> <path d="…"/> -->` rendait deux chemins au lieu
d'aucun) ; et un `<path>` sans `d` entrait comme un chemin vide, l'émetteur posant alors
un `f` sans le moindre tracé devant. Aucun n'est atteignable depuis un album — le pack
est compilé — et tous les quatre frappaient la personne à qui ce module doit un message
clair : le contributeur. La leçon est dans le module : **refuser, jamais ignorer**.

**Une seule rotation dans le projet.** Un bloc tourne par `Tm`, un ornement par `cm`, et
les quatre coefficients sortent de `pdf::coefficients_rotation`, appelée par les deux ;
le test compare les deux flux au même angle. **`Td` reste `Td`** : un album sans ornement
rend le PDF de `main` à l'octet, revérifié à 214 814 octets. Un ornement est noir, un seul
remplissage, et il s'écrit `0 0 0 rg` et non `0 g` — ce document déclare un espace de
couleur et un OutputIntent, et faire entrer DeviceGray par un fleuron mettrait un second
espace devant veraPDF sans rien acheter. **PASS PDF/A-2b** avec neuf ornements dedans.
Un identifiant qu'aucun pack ne porte **ne dessine rien et ne fait rien échouer** : un
`album.json` se répare à la main, donc cet état est atteignable.

**L'encre tombe dans la boîte, et c'est mesuré au raster**
(`scripts/ornement-encre.py`, dans `check.sh` à côté de `pdf-png`, macOS seulement).
Trois poses, dix pixels par millimètre — à 72 ppp un demi-millimètre vaut un pixel et
demi et la mesure battrait au hasard. **La flèche est la sonde** : elle touche ses quatre
bords, là où un filet en losange se termine en pointe et absorbe un millimètre d'erreur
en silence. Vérifié en mutant la matrice de trois points : le losange passe à +0,20 mm et
laisse vert, la flèche à +1,00 mm et rougit. Chiffres :
`docs/mesures/2026-09-06-l-ornement.json`.

**6.3 s2 est faite**, et elle est ci-dessous, moins son compteur qui vit avec les autres
dans « Le linter, la reprise et le prévol devant un objet libre ». Le pack porte
**35 actifs** (#58, normalisés par `scripts/ornement-normaliser.py`). Les cartouches
n'ont aucun gisement libre sur Commons : trois familles, `fleuron`, `filet`,
`separateur`, et pas de quatrième.

### L'ornement se pose (6.3 s2)

**Le sélecteur n'est pas un sixième panneau.** Un second bouton dans la barre contextuelle,
juste à côté de « Bloc de texte », et le popover de `TemplatePicker` au motif près
(`OrnementPicker.tsx`) : bouton `.link`, fermeture au clic dehors et à Échap. Pas d'entrée
de menu, pas de raccourci — « Bloc de texte » n'en a pas non plus, et les cinq panneaux qui
s'excluent restent cinq. **Il montre des dessins, pas des noms** : une entrée *est* le
dessin, en SVG inline dans une case de hauteur fixe, le titre du pack la nommant pour
VoiceOver et au survol. Les groupes sont `fleuron`, `filet`, `separateur`, et cet ordre vit
dans `ornement.ts` et non dans le manifeste : une PR de données ne doit pas déplacer les
groupes sous la main de quelqu'un. Un pack vide — moteur plus vieux que la fenêtre — rend
**une phrase, jamais une grille vide**. Le nom du pack, lui, ne voyage pas dans le dump (un
ornement y porte son identifiant, le dessin ne dépendant pas d'où il sort) :
`ornement.ts::PACK_INTERNE` en est le miroir, et c'est la pose qui l'écrit.

**Un objet libre naît hors des cases photo, et les deux contenus passent par la même
porte.** `addObjet` et le nouvel `addOrnement` prennent **en plus** les rectangles des
cases, en repère moteur ; `App::poser` les rassemble (`slotsFor` puis `retournerBoite`,
comme il le fait déjà pour `boiteDePage`) et choisit l'objet posé. Le contrat, dans cet
ordre. **Liste vide ou absente : la position d'hier, au millimètre** — c'est ce qui garde
verts, sans y toucher, les tests de naissance d'avant. Sinon on part de cette position et,
si la boîte y recouvre une photo, on glisse à la position libre **la plus proche de
celle-là**, cherchée sur une grille de 24 × 24 dans la boîte de page, candidates ordonnées
par distance : c'est une aide à la pose, pas une mise en page, et il n'y a pas d'algorithme
de plus grand rectangle vide. **Aucune position libre : on pose quand même**, à la place
d'hier, et la ligne de statut le dit — refuser serait un cul-de-sac. « Recouvre » est un
chevauchement **strict** et une tangence ne recouvre pas. C'est `scene.ts::recouvre` qui
répond, le port de `scene.rs::recouvre` : la géométrie de l'app vit dans `scene.ts` avec
`horsMarge` et `retenirAuPli`, jamais dans le module des éditions, et une règle qui aurait
deux maisons dans deux langues en aurait bientôt quatre. La naissance l'appelle **aux deux
angles zéro** — un objet neuf n'est jamais tourné, une case ne l'est jamais — et il rend
alors ce qu'un chevauchement droit rendrait, au bit.

**Un ornement naît au tiers de la largeur de la page** (`ORNEMENT_PART`), **et sa boîte
garde le rapport de son dessin dès la première image** : si la hauteur qui en découle
dépassait la page, c'est la hauteur qui commande et la largeur qui suit. « La boîte **est**
l'encre » n'a pas d'exception, pas même à la naissance, comme `tailler` le tient ensuite.
Ni plancher ni plafond de plus : la main retaille.

**À propos crédite le pack, et seulement quand il est là** : créditer un pack absent serait
un mensonge dans le seul écran dont c'est tout le métier de dire ce qui voyage à
l'intérieur, et le cas est atteignable. Le détail par actif reste dans
`assets/ornements/LICENCES.md`, engendré et déjà testé pour sa fraîcheur. `notices.sh` et
`NOTICES.md` ne bougent pas : ils portent les obligations des bibliothèques tierces sous
copyleft, et un actif CC0 n'en est pas une.

### L'éditeur des objets libres (6.2 s2)

**La sélection est un second état, pas une union.** `App` tient `selected` (l'indice de
case) et `objet` (l'indice de l'objet libre), et deux aides-mémoire imposent qu'**au plus
un des deux est posé**. Une case choisie ouvre l'éditeur de recadrage, un objet choisi
celui de transformation : les fondre en une union churnerait trente sites d'appel pour ne
rien gagner. Le papier nu lâche les deux.

**`ObjetLibreCalque` porte le geste, en DOM dans les deux rendus** — le précédent est
celui des pastilles d'une case : une poignée a un curseur, un nom pour VoiceOver et un
état de survol, et un canvas n'a rien de tout ça. Au canvas, la *sélection* passe par
`hitTest` (un objet libre est au-dessus de tout, donc il gagne), le calque prend le relais
ensuite. Le calque ne décide de rien de géométrique : `retenirAuPli` et `horsMarge` vivent
dans `scene.ts`, et le clavier les appelle aussi.

**Le pli bute, la marge avertit.** La butée est une translation, pas un refus — le geste
continue de suivre la main. Pour tailler et tourner, le côté est pris **au début** du
geste, sinon l'objet sauterait d'une page à l'autre en touchant le pli. **Déplacer le relit
à chaque pas** (`scene.ts::coteDuGlisse`, retours du 28/10) : passé le milieu, l'objet
saute sur l'autre page, jamais à cheval ; retenu du côté de départ, un bloc né à gauche ne
gagnait jamais la droite. La page collée reste interdite. Vérifié à l'écran le 05/09 :
bloc tourné à −46° poussé 293 px au-delà, bord droit arrêté au pli au pixel. Un bloc neuf
naît à 26 pt (`OBJET_TAILLE_PT`, le titre de couverture), 12 pt se lisait à peine.

**Un geste ne pose qu'un pas d'annulation** : un brouillon local pendant, une édition à la
fin. Le brouillon entre par la **planche**, pas par la scène — la boîte décide où les
lignes se coupent, donc substituer la boîte après coup montrerait le texte de l'ancienne.
Et c'est la dernière pose gardée dans la ref qui est validée, jamais la prop : un dernier
déplacement et le relâchement peuvent tomber dans la même image.

**Le piège qui a coûté le plus, et qu'il ne faut pas défaire.** Partout ailleurs l'éditeur
dessine le texte **un tiers plus grand** qu'il ne s'imprime, pour la lisibilité. Un bloc
libre est **la seule exception** : sa boîte décide de ses coupures, donc grossir son texte
sans grossir sa boîte montre des coupures que le PDF n'a pas. Mesuré le 05/09 avant
correction : une ligne rendait 331 px dans une boîte de 255. Il se dessine à sa taille
d'impression, **sans plancher de lisibilité non plus** — un plancher est un grossissement
qui ne dit pas son nom. Les deux rendus le font, et le disent au même endroit.

**Le champ de saisie est droit même si le bloc est tourné** : un curseur et une sélection
dans un champ tourné sont le plus faible chemin d'un navigateur, et ce qu'on vient y faire
est taper. Échap referme sans écrire, ⌘⏎ écrit et referme, un retour simple reste un
retour à la ligne.

**`boiteDePage` est lue du dump**, pas recalculée : c'est `pdf.rs::page_box` qui décide
qu'une demi-gouttière reste du côté du pli, et un bloc neuf naît dans cette boîte — donc
jamais à cheval sur le pli, jamais hors marge, par construction. La parité la compare.

`tailler` (redimensionner par un coin) travaille **dans les axes de la boîte** : on ramène
le déplacement par une rotation inverse, on y lit largeur et hauteur, puis on replace le
centre. L'invariant est testé à tous les angles — **le coin opposé ne bouge pas dans le
monde** —, et c'est lui qui empêche la boîte de glisser sous la main pendant qu'on la
retaille.

### Le linter, la reprise et le prévol devant un objet libre (6.4)

**Le linter et le prévol lisent la scène, aucun des deux ne recalcule.** `hors_marge`,
`traverse_le_pli` et `recouvre` vivent dans `scene.rs`, avec le geste de l'éditeur et
l'émetteur ; les deux les appellent. `recouvre` est une séparation d'axes sur les quatre
normales des deux rectangles, et ses axes sont normalisés : à l'angle zéro la projection
d'un coin est sa propre coordonnée, donc le verdict est celui d'un chevauchement droit,
au bit. Une seconde implémentation de la doctrine est exactement ce que ce
module-là existe pour supprimer, et `hors_marge` n'était écrit qu'en TypeScript jusqu'ici
— un seuil dans une seule langue pour une règle que deux lisent. `--reprise`, lui, ne
lit pas la scène mais le fichier : il mesure un écart entre deux `album.json`, et un
objet libre y est un champ, pas un rectangle.

**Trois compteurs de linter, et ils avertissent au lieu de décider.**
`objet_hors_marge` (la boîte entre dans la bande sûre, `CAPTION_SAFE` × marge à
l'intérieur de la coupe), `objet_deborde` (le texte est plus haut que sa boîte,
`overflow`, ou un mot y est plus large, `trop_large` — deux façons de ne pas tenir dans la
boîte qu'on a dessinée, un seul compteur) et `ornement_sur_photo` (la boîte d'un ornement
recouvre une case, `scene::recouvre`, une ligne par ornement quel que soit le nombre de
cases mordues — ce qui se répare est l'ornement). Un objet posé volontairement à fond
perdu est un choix, l'éditeur a déjà averti au moment du geste, et le refus vit au prévol
sur la coupe et le pli.

**Le linter a donc trois régimes et non deux, et `seuil` les porte tous les trois**
(`Counter::seuil: Option<usize>`). Un compteur sans seuil compte, détaille, et ne décide
pas : `passes()` rend vrai, `--audit` ne rougit pas, et le champ est **absent** du rapport
JSON, ce que `check.sh` et `signaler.ts` lisent tous les deux pour ne pas afficher un
« 2 / 0 » qui veut dire rouge. Les dix compteurs du Composer gardent leurs seuils et leur
pouvoir au chiffre près. Nés à `seuil: 0`, les trois compteurs de la main étaient durs en
fait pendant qu'on les décrivait mous — `dur` n'est lu par personne, c'est `seuil` qui
décide —, et la seule sortie disponible aurait été de relever un chiffre pour faire taire
une règle, ce que la session C a justement refusé de faire au prévol. C'est « le pli bute,
la marge avertit » porté au linter.

**Les deux compteurs n'ont pas la même portée, et 6.3 s1 l'a tranché** :
`objet_hors_marge` mesure une boîte, donc il compte tout ce qu'une main a posé, ornement
compris ; `objet_deborde` mesure un texte contre la boîte où on l'a coupé, et un ornement
n'a rien à déborder. Le prévol, lui, mord sur les deux sans distinction. Ce qui rend cela
sûr est `Role::index_libre` : **un seul accesseur** au lieu d'un `match` par appelant, sans
quoi un troisième contenu compilerait pendant qu'il cesserait silencieusement d'être
mesuré.

**Deux bloquants au prévol, et deux seulement** : `objet_coupe`
(`distance_to_trim < 0`) et `objet_pli` (`traverse_le_pli`). La marge reste molle
partout ; ce que la guillotine traverse et ce que la reliure coupe en deux, non.

**`--reprise` gagne la classe `objet_libre`, sans compteur parent.** Poser un objet est
un geste de goût, pas un défaut que le Composer aurait pu attraper — il n'en propose aucun,
donc aucun compteur ne pouvait prévenir la main, comme pour `insertion` et `suppression`.
La détection est une **égalité exacte** sur `Spread::objets`, sans epsilon à la différence
du recadrage : un focal est un nombre que la machine a proposé, un objet libre n'existe
que parce qu'une main l'a posé.

### Le schéma, et sa migration étagée (6.4)

**`SCHEMA` vaut 3, et 3 veut dire « ce fichier peut porter des objets libres ».** Rien ne
s'y lit autrement — `objets` et `police` sont additifs, un album de schéma 2 les lit
déjà. La montée achète **une seule chose**, et elle la vaut : `migrate_album_folder`
**refuse** désormais toute version au-dessus de la sienne (`SchemaTropRecent`), au lieu
de laisser serde ouvrir le fichier en jetant ce qu'il ne connaît pas et la première
sauvegarde graver la perte. L'ancien `>= SCHEMA` laissait passer le dessus sans un mot.
C'est le seul échec de migration que les deux appelants ne ravalent pas.

**La migration est étagée, et il le fallait.** Chaque étape connaît la version d'où elle
part : 1 → 2 convertit les `focal` (`focal_du_schema_1`), 2 → 3 estampille et rien
d'autre. Faire tourner le tout pour « toute version sous `SCHEMA` » marchait tant qu'il
n'y avait qu'une étape et devenait un piège à la deuxième — un album de schéma 2 aurait
vu ses `focal` reconvertis, et **une double migration abîme ce qu'une simple réparait**.
Le test qui mord compare le fichier entier, à l'estampille près, avec les vignettes
présentes exprès : sans elles la conversion non étagée n'aurait rien eu à convertir et se
serait tue.

## L'app

Recadrage, tiroir, table lumineuse ⌘3, badge « éditée » et cadenas ⌘L, « rendre à
l'automatique », recomposition préservante, légendes, texte, couverture, **Envoi ⌘4**
(qui offre le verdict après un export ou une préparation), bilan de choix, revue
clavier, Stockage, À propos, **fiche d'une photo ⌘I**, **Préférences ⌘,** (langue
`i18n.ts` sans redémarrage, mises à jour, apparence), **aperçu
fidèle ⇧⌘P** (pdf.js). Ni clé ni dépôt dans les Préférences : Commander passe par le
relais (voir plus haut). L'interrupteur DOM/canvas a quitté les Préférences : il ne vit
plus que dans la clé `colophon.rendu` du `localStorage` (`rendu.ts`), pour la mesure.

**L'album s'écrit seul** (1.1-b, `autosave.ts`) : une seconde après la dernière
modification, et tout de suite avant Envoi, un rendu PDF, une recomposition. Une écriture
à la fois, la suivante part avec le dernier état. **Deux commandes** : `ecrire_album`
n'écrit que `album.json` ; `save_album` (⌘S, fermer, ouvrir un autre, quitter) élague en
plus les propositions et les faces de bloc orphelines — pendant la séance, un ⌘Z
ramènerait un bloc sans sa police. À l'ouverture (`open_album`, pas une recomposition),
`album.ouverture.json` copie l'album, hors d'`IDS_PROPOSITIONS` ; Fichier → « Revenir à
l'ouverture de la séance » l'applique comme une retouche. Aucun dialogue « non
enregistré » : un échec d'écriture tient la ligne d'état en terracotta, et seul Quitter
redemande tant qu'il dure.

**Le clavier garde sa place quand la page tourne** : la couche est démontée à chaque tour
(`key={index}` rejoue l'animation), donc la mémoire du rang vit hors du composant ; au
bout de l'ordre de lecture, la flèche tourne la planche. **Un champ ouvert depuis une
boîte rend le focus à cette boîte** : le rang survit au blur vers le champ, et Entrée
valide avec `preventDefault`, sans lui l'action par défaut rouvrait le champ. **Le canvas
avale son clic résiduel**, sans quoi le papier désélectionnait dans le même souffle. **La
ligne de statut est vivante** (`role="status"`, les quatre pieds) et **la table lumineuse
se parcourt au clavier** (un arrêt de tabulation, flèches, verticales d'une rangée).
**La légende proposée** : champ vide, fantôme gris (`legende::proposition`), Tab la pose
et marque `edited`. Les badges de case sont l'alerte de qualité (voir plus bas).

**L'écran Nouvel album n'offre que ce qui s'imprime, et ne laisse aucune colonne vide**
(C1-C2). Cinq formats, ceux de `commande::formats_imprimables` (déduite de
`produit_pour`, commande `formats_imprimables`) ; 28 × 21 et la taille libre restent à la
CLI. Plus de rythme : `equilibree` part toujours, C3 rouvre le choix dans Recomposer. À
droite, dès qu'un dossier est choisi : une couverture proposée (`nouvelAlbum.ts::couvertureProposee`,
dite « aperçu », pas celle du Composer), la **visionneuse de format** et trois lignes
(`estimerLivre`, la règle du Composer, « environ »). Tout vient du **survol**
(`core::survol`, commande `survol_dossier`, CLI cachée `--survol`) : le compte de `scan`
et huit **miniatures** lues dans l'en-tête des fichiers, jamais un original, budget
200 ms — 4 ms sur corse, 70 sur mauritanie, mesuré le 18/10. `VisionneuseFormat.tsx`
ne sait rien de l'écran qui l'appelle : une échelle commune à tous les formats
(`visionneuse.ts::BOITE_MM`, une constante, jamais calculée sur la liste offerte), les
cases du moteur (`slotsFor`), une main de 19 cm, la cote ouverte, des aplats sans
vignette, la taille animée en 250 ms.

**Le sélecteur de gabarits montre des dispositions** (`gabarit.ts`) : verso, bande de
légende et forme de cellule repliés, une entrée par disposition, groupées par nombre
de photos — les 171 gabarits qu'une planche de quatre photos peut prendre deviennent
au plus 23 entrées, et chacune porte un nom français au lieu du `g_1x2f_1x2f` que le
catalogue généré affichait. La variante réellement posée est celle de plus faible
trahison, **sans bande d'abord** (le Composer n'en pose aucune) : `gabarits_compatibles`
remonte donc `[nom, trahison]`, l'aptitude gardant sa définition unique côté moteur.
La grammaire des noms générés est relue côté app **pour le seul libellé**, et
`gabarit.test.ts` exige qu'aucun nom du dump n'y échappe, que la disposition déclare
autant de cases que le dump en pose, et qu'elle les répartisse du même côté du pli —
c'est ce test qui tient la table des familles historiques, seul endroit de l'app qui
redit une forme du moteur. G et ⇧G lisent la même liste.

**Recomposer ouvre un panneau, et c'est là que le format se change** (C3,
`RecomposerView.tsx`, `recomposer.ts`). Les cinq formats de Nouvel album (le format
d'un album en 28 × 21 ou en taille libre s'y ajoute, coché, « non imprimable par
Colophon »), la `VisionneuseFormat` garnie des vignettes de l'album, la case « Moins de
photos, plus grandes » (`aeree`), la phrase chiffrée des planches gardées, et le bouton.
Le panneau n'envoie **que ce qui a changé** (`choixDeRecomposition`) : rien de changé,
`recompose_album` reçoit `format: null, densite: null` et rend l'`album.json` d'avant au
bit près — c'est le test `sans_parametres_le_meme_album_json_qu_avant`, qui recopie
l'ancien assemblage de la coquille. Un format donné passe les planches gardées par
`bascule::bascule` **avant** de recomposer le reste au nouveau format
(`core::recomposition::recomposer`, appelé par la commande et par la CLI cachée
`--recomposer`, que le harnais lance par `POST /__dev/recomposer`). La phrase dit alors
que les planches gardées « gardent leurs photos ; leur mise en page s'adapte » : « telles
quelles » serait faux. ⌘Z repart de zéro après une recomposition, et le panneau le dit.
**`recomposeAlbum` recharge la géométrie** après la commande : avant C3 le format ne
changeait jamais là, et un album passé en A4 sur le dump du carré est une fenêtre
blanche. « Changer de format sans recomposer », lien du panneau, est le seul chemin vers
`BasculeView` ; l'ancien « Format et police » est devenu **Police** (`PoliceView.tsx`),
sans format, et la destination `format` d'Envoi mène à Recomposer.

Menu natif (`menu.ts`), barre latérale et barre du haut (voir **DA**), couverture = cellule zéro de Planches,
export uniquement par Envoi. **Toute commande passe par la table `raw` d'App et
`menu.ts`** ; les cinq panneaux s'excluent. Recomposer, le changement de format sans
recomposer et Police sont **une seule place** (l'état `bascule` d'App, une union de trois
valeurs) : ils s'excluent par construction et le compte ne bouge pas. Fichier porte
« Recomposer… » et « Police… ». **Aide puis Signaler** (`signaler.ts`) :
rapport entier visible avant envoi, jamais un chemin ni une légende.

**DA** : « La page calme » (`DESIGN.md`, plan `plans/2026-10-28-da-la-page-calme.md` côté gestion).
Chrome blanc, encre gris chaud, un seul filet, police du système pour tout le chrome (aucune
police chargée pour lui) ; `--font-book` et Source Sans 3 ne servent que le livre. Trois
rayons, un rôle chacun (`--r-tag` 4, `--r-ctl` 6, `--r-panel` 10), **le papier reste carré**.
Action presque noire, **une seule par écran** (`.cta`, secondaire `.btn-second`, discret
`.link`) ; le terracotta ne vit que dans le chapitre (étiquette, repère) et la marque ; le
bleu `--link` est le focus et la sélection, et **ce qui est dessiné sur le papier garde sa
paire fixe** (`--link-paper`, `--paper-warn`), sinon le sombre rend du pâle sur du crème.
Quatre étiquettes (`.tag` : `chap`, `qual`, `main`, `pret`, gris par défaut), aucune
décorative ; le rouge est l'erreur et la destruction, jamais une étiquette. **Jamais de
bordure colorée sur un côté** (un encart a un fond pâle). Deux ombres : `--shadow-float`
pour ce qui flotte, `--shadow-sheet` pour la feuille sur la table. Six tailles, rien sous
12 px (`--fs-12` à `--fs-28`). Le sombre suit le système. **`couleurs-chrome.test.ts` lit
`styles.css` et tient chaque paire** (texte 4,5:1, frontière de champ 3:1, clair et
sombre) : une paire neuve s'y ajoute avant de partir. Les bordures d'un champ prennent
`--field-border`, celles d'un bouton qui porte son libellé prennent `--line`. La salle
sombre de la revue clavier (`--salle-*`) et les `--chip-*` sur photo ne bougent pas. `--ink-rgb`
= encre du papier, `--ui-rgb` = encre du chrome.

**La structure** : barre latérale (`Lateral.tsx`, 240 px, `⌘\`, entrée « Barre latérale » du
menu Affichage, repliée d'elle-même sous 1180 px de fenêtre tant que personne n'a tranché),
barre du haut en fil d'Ariane (`Bar` dans `App.tsx` : annuler, rétablir, Recomposer discret,
Commander secondaire **seulement si un relais est compilé** — `useRelais`, la même sonde
qu'Envoi —, « Exporter le PDF » mène à Envoi, qui est le seul chemin d'export ; aucun des
deux sur Envoi), les vues et les chapitres à gauche (`chapitres.ts` : un chapitre est une
planche dont `caption` est posée, jusqu'à la suivante). Le Livre est la planche sur sa table
et **le panneau de propriétés** (`Proprietes.tsx`, 290 px) : chapitre, disposition (le
sélecteur de gabarit, qui s'ouvre vers la gauche), moment (la prise de vue de la première
photo, si le fichier la dit), état, qualité, puis la photo choisie, son encart qualité et
ses réglages. La ligne de contexte ne garde que « Feuilleter » et l'aide du geste.
Planches : vues en onglets (Toutes, À vérifier, Éditées) ; **le glisser et Alt + flèches
n'existent que sur la vue complète** (réordonner entre des voisines qu'on ne voit pas serait
un piège), le clavier marche sur les cellules visibles. « À vérifier » compte ce que les
vignettes décodées ont dit : son nombre peut croître au défilement (« sombre » lit les
pixels). Le menu contextuel porte raccourcis, filets et Supprimer seul en rouge
(`Entree.raccourci|filet|danger`). Pas de zone de dépôt à l'accueil : `dragDropEnabled`
est faux, la zone mentirait.

## Le PDF

**L'export se déclare PDF/X-4 et PDF/A-2b** (`core::pdfx`) : OutputIntent sRGB, XMP,
`/Trapped`, `/ID`, en-tête 1.6. Aucun validateur gratuit ne certifie X-4 : la mesure est
PDF/A-2b (veraPDF) plus cinq tests Rust, le verdict X-4 revient au prévol imprimeur.
**Le PDF est reproductible** : `SOURCE_DATE_EPOCH` honoré, deux exports du même album
sont identiques à l'octet.

**Le texte est écrit en composite** (`/Type0` sous Identity-H, descendant
`/CIDFontType2`, `/CIDToGIDMap /Identity`, plus `/ToUnicode`) : le code du flux **est**
le glyphe, écrit en chaîne hexadécimale, et rien ne dépend plus d'une table d'encodage.
Une légende porte donc **tout ce que la face sait dessiner**, et plus les 224 codes d'un
encodage à un octet. La face entre **entière** : aucun préfixe de sous-ensemble sur son
nom, aucun `/CIDSet` — les deux annonceraient un sous-ensemble qui n'existe pas. Seuls
`/W` et `/ToUnicode` sont restreints aux glyphes **dessinés**, accumulés pendant le
dessin dans `font::Utilises`. Trois conséquences à ne pas défaire : l'accumulateur est
**ordonné** (une carte de hachage rendrait un fichier différent d'un export à l'autre) ;
les objets de police sont **réservés à la construction et écrits à `save()`**, les
glyphes n'étant connus qu'à la dernière planche ; et l'écrivain **descend jusqu'aux
appelants**, `cover.rs` écrivant son flux hors du writer — un accumulateur logé dans le
writer perdrait tout le texte du dos sans que rien le voie.

**Le passe-plat JPEG vérifie le fichier entier avant de le recopier** (`print::jpeg_entier`) :
une marche des segments jusqu'à `FFD9`, en-têtes sautés par leur longueur (la vignette EXIF
a son propre EOI, qui ne compte pas), données de scan sautées au marqueur suivant. Mesuré le
26/09 (`docs/mesures/2026-09-26-le-passe-plat-jpeg.json`) : 1 ms par photo, zéro refus sur
les 572 de corse ; le décodage complet coûtait 29 ms et n'attrapait rien de plus, le
décodeur tolérant une soupe de marqueurs. Piège : un fichier réel du jeu finit par soixante
espaces après son EOI, une recherche en queue de fichier le refusait à tort.

**Trois PDF** : `album.pdf` = aperçu vignettes, jamais imprimé, mais c'est lui que lit
l'aperçu fidèle ; `--print` = 300 dpi, rien ne court-circuite `print_scale` ; `--cover` =
la feuille à plat, une par profil. **Ce qui doit survivre au massicot se mesure depuis la
coupe**, et il n'y a plus qu'une implémentation, `scene::distance_to_trim`.

### L'imposition, quand l'imprimeur relie page par page

**Le cadre change à l'export, jamais à la composition.** `core::imposition` prend une
scène déjà bâtie et la coupe en deux ; `pdf::geometry` ne bouge pas, `album.json` ne bouge
pas, l'éditeur ne bouge pas, **aucune fiche ne bouge**, et les deux fixtures se régénèrent
à l'octet. C'est cette immobilité qui sert de preuve, et elle est mesurée : le même
`--print` sous un profil qui relie des planches rend le fichier de `main` **à l'octet**,
95 Mo de photographies comprises (`docs/mesures/2026-09-08-les-pages-simples.json`).

Deux choses arrivent à un objet en chemin, et rien d'autre. **Une photo dont la case
atteint le pli** en donne `pli_mm` de plus vers le pli, comme elle en donne déjà sur les
trois autres bords — c'est tout le fond perdu intérieur. **Tout le reste est translaté**
dans le repère de sa page. Ni un texte ni un objet posé à la main ne saigne : le pli est
de la colle, et l'éditeur y bute exprès.

Le fond perdu du pli est le seul endroit où un rectangle change, donc **le prévol mesure la
résolution sur `rect_exporte`** et non sur la case composée : `s = max(w/iw, h/ih)`, donc
élargir la case ne coûte un pixel qu'aux photos dont le recadrage est piloté par la
largeur. Mesuré sur corse-2013 : 4 cases sur 17 perdent du ppi, 5 ppi au pire, aucune ne
franchit le plancher de 250.

**L'imposition est dans `faces`, et `ordre` en est le seul autre lecteur** : p1 = planche 1
recto, p2k = planche k+1 gauche, p2k+1 = sa droite, p2n = blanche. La moitié gauche de la
première planche ne s'imprime pas — c'est la page qui fait face à l'intérieur du plat — et
c'est ce qui met le faux-titre en page un et le colophon en page 95, deux rectos. En
échange, cette moitié doit être vide : le prévol la refuse (`imposition`) au lieu de la
tronquer en silence.

**Sans faux-titre, la page un est blanche (retours du 28/10, défait l'ouverture sur une
photo de 1.1-d).** `BuildOptions::garde` vaut faux par défaut (CLI : `--garde` le rend) ;
une recomposition reporte la garde d'un album qui en a une. Sans elle, `build.rs` pose une
planche `vide` en tête dès que la première porte une photo (`Spread::vide`) : rien juste
derrière la couverture, gauche collée comme droite. `Composer::ouverture_a_droite` et sa
recherche de gabarit sont retirés ; reste de 1.1-d **une bande dont toutes les photos sont
à droite pend à droite** (`caption_anchor_of`). `reserve::cases_vides` saute la planche 0,
et côté app `edits.ts::photoEnPageUne` est posée par `apply` sur l'avant et l'après de
toute édition : aucun chemin (repêcher, coller, glisser, déplacer ou supprimer une planche)
n'ajoute de photo à la page 1 ; ce qu'un album d'avant y porte reste. `rescuePhoto` cherche
à partir de la planche 1.
L'éditeur grise la page collée (`scene.ts::gaucheCollee`, le prédicat du prévol, profil
courant), refuse ce qu'on y lâche en le disant, pose bloc et ornement à droite
(`pageDePose`), et la table grise la même moitié.

**`Bleed::dos` a enfin un sens** : une planche n'a pas de bord au pli, une page simple en a
un, et il ne se lit que par `imposition::pli_mm`.

La sonde est `scripts/pages-simples.py` : elle rend les deux formes du même album, compare
chaque page à la moitié de planche dont elle sort, et **se vérifie mordante** avec
`--mutant`, qui décale l'imposition d'une page. `check.sh` l'appelle en `--avec-mutant`,
qui fait les deux passes sur **un seul rendu**, sur `mauritanie-2019` — le plus léger des
trois jeux, 37 s. Le bloc se saute là où il ne peut rien mesurer : sans photographies il
n'y a pas de tirage 300 dpi, donc jamais en CI ; et il demande `pypdfium2`, que le message
de saut nomme (`python3 -m pip install --user --break-system-packages pypdfium2`). Deux pièges
du raster y sont écrits, tous deux mesurés le 08/09 : pdfium étire la page sur un bitmap
arrondi vers le haut, et deux microns d'écart de coordonnée suffisent à faire glisser un
raster d'une colonne. **`ornement-encre.py` et `apercu-fidele.py` nomment désormais
`--profil lulu`** : ils ont besoin d'un intérieur de planches doubles sans couverture
dedans, et le défaut de la ligne de commande découpe.

`--audit` : quatorze compteurs, 18/18 verts (3 jeux × 6 formats), sur les trois propositions
de chaque jeu. `--reprise` : part des planches corrigées à la main contre
`album.origin.json` ; sous 10 % bon, jusqu'à 30 % à surveiller, au-delà rédhibitoire.
`--prevol --profil <id>` : bloquants et avertissements contre un `PrinterProfile`, et la
fiche que l'imprimeur demande au téléphone. **Bloque ce que la coupe traverse ou que la
reliure sépare** (`format_sans_produit`, `couverture_vide`,
`imposition`, `objet_coupe`, `objet_pli`, `fond_perdu`, `espace`, `pagination`,
`fichier_interieur`, `fichier_couverture`, `fichier_profil`, `fichier_perime`) ; **avertit de ce qu'un fournisseur
préfère** (`zone_sure`, `dos_nu`, un coefficient de dos provisoire) ; **laisse à l'auteur**
la résolution (`resolution`, `couverture_resolution`, 1.1-c). Un seuil réglé pour
faire taire une règle est le défaut que cette frontière existe pour empêcher.

## Pièges connus

**Ce que le moteur rédige reste en français, et l'app l'écrit en anglais depuis des
codes** (`moteur.ts`) : les défauts du prévol portent `phrase` (codes et valeurs déjà
formatées, table `prevol::PHRASES`), un format, un rythme et une proposition leur
identifiant (`VarianteResume.densite` pour « autre-rythme »), et les textes fixes —
notes du rapport, réserves des profils — se reconnaissent à leur français exact.
L'écran français affiche le français du moteur tel quel, aucune copie dans l'app. La
liste est `moteur.codes.json`, tenue fraîche par `codes.rs` (régénérer :
`cargo test -p colophon-core ecrire_la_liste_des_codes -- --ignored`) ; `moteur.test.ts`
exige une phrase anglaise par code et aucun trou sans valeur, et un mot changé dans une
réserve ou une note rougit jusqu'à ce que l'anglais suive. Un code inconnu rend le
français, jamais un trou. **Une nouvelle règle de prévol = son nom dans `REGLES`, un
code dans `PHRASES`, la phrase dans `moteur.ts`, sa destination dans `envoi.ts` et sa
phrase simple dans `i18n.ts`.** Restent tels quels : ce que la fiche d'une photo cite
(appareil, lieu), qui est de la donnée, et la raison brute d'un PDF illisible. **Vitest tourne sans `navigator`**, langue par défaut
anglaise : un test qui affirme du français pose `setLangue("fr")`. Sous
`pipeline::PETIT_DOSSIER` (25 photos), la curation se limite aux rejets certains. **Et
le filtre parasite ne s'arme que si l'empreinte d'appareil est majoritaire**
(`pipeline::empreinte_est_un_signal`, majorité stricte) : un dossier de scans ou un
Takeout sans sidecars n'a personne d'empreint, la règle y mesurerait l'origine du dossier
et non ses parasites, elle se coupe et le dit (`note:`), le livre suit les dates de fichier.

**Une migration de schéma est étagée, jamais « tout ce qui est sous `SCHEMA` ».** Le
jour où une deuxième étape arrive, la première se remettrait à tourner sur des albums
qu'elle a déjà convertis, et une double conversion abîme ce qu'une simple réparait. Une
étape = une version de départ, et `Migration::slots` ne compte que ce que 1 → 2 a
converti. Voir « Le schéma, et sa migration étagée ».

**`album.origin.json` ne se réécrit jamais**, c'est la référence de `--reprise` :
composer dans un dossier déjà utilisé mesurerait l'album du jour contre une vieille
proposition. `check.sh` efface avant. **Les seuils du linter sont co-réglés avec le
Composer** (250 ppi, écart 1,4, doublons 24 bits dHash / 8 pHash / 180 s, couleur ≤ 20)
dans `audit.rs`, importés par `layout.rs`.

**Les actifs sous licence gardent leur licence à côté** : `colophon-core/assets/` (OFL,
ICC sRGB, GeoNames **CC-BY, attribution obligatoire**) ; l'écran À propos porte les trois.
Le pack d'ornements a le sien, `assets/ornements/LICENCES.md`, **engendré** par
`ornement::licences_md` et dont la fraîcheur est un test — un inventaire de licences tenu
à la main est faux au premier ajout. À propos le crédite (6.3 s2). **Ce
test compare des contenus, pas des fins de ligne** : git rend un `.md` en CRLF sur
Windows là où le générateur écrit des `\n`, et le gate y accusait l'inventaire d'avoir
vieilli sans qu'un caractère diffère. Le dépôt n'a pas de `.gitattributes`, et en poser
un renormaliserait tous ses fichiers texte pour une seule comparaison ; tout autre
`include_str!` du projet est analysé, jamais comparé, donc aucun n'a ce piège.

**Un album ne s'écrit jamais où un album est** (`lib.rs::dossier_libre`) : composer deux
fois le même dossier pose le second à côté (`-2`, `-3`), jamais par-dessus. L'album déjà
composé se retrouve **par le `root` de son `album.json`** (`album_deja_compose`), jamais par
le hachage du chemin, qui est celui du projet (`export::empreinte`) et non `DefaultHasher`,
instable d'une version de Rust à l'autre. La question « le rouvrir ? » vit dans
`App::createAlbum`, avant tout travail.

**Le serveur de dev écrase le vrai `album.json`** (POST `/__dev/album`) : copies jetables.
**Recharger la page après toute édition de source**, le fast refresh Vite corrompt l'état
React. **Vignettes** : nom non devinable, tout lecteur passe par `thumbs.json` ; sous
1600 px une vignette EST l'original, le badge ppi s'y fie. **Mémoire** : pipeline sur
vignettes, l'original ne s'ouvre qu'au rendu.

**Une fenêtre en arrière-plan ne reçoit aucune trame d'animation** : pdf.js n'y résout
jamais sa promesse et `mesure.ts` n'y pose aucun chiffre, vérifier `document.visibilityState`
avant d'accuser le code. **Une fenêtre sans le focus système ne reçoit aucun événement de
focus** (`hasFocus()` faux, les deux harnais) : `focus()` déplace `activeElement`, mais
`focusin` ne part pas et `:focus` ne matche pas. Tout ce qui touche au clavier ou au
chronomètre se vérifie **au premier plan**, jamais au harnais. **Mais une capture d'écran
force une étape de rendu** : sans capture préalable, `ResizeObserver` n'a jamais livré ses
callbacks, l'échelle de la planche vaut 1 et tout relevé DOM est faussé. Capturer, puis
relever. La cure des fenêtres borgnes : instance Brave dédiée, drapeaux anti-occlusion,
focus émulé CDP, `scripts/mesure-cdp.mjs` et `scripts/mesure-rendu.md` § « La cure ».
**pdf.js** : jamais de lambda dans les dépendances de l'effet de rendu. Et **l'aperçu
fidèle ne s'affichera jamais dans un harnais borgne**, capture d'écran ou pas : la
promesse de rendu de pdf.js ne se résout que sur une trame d'animation, qu'une fenêtre
cachée ne reçoit pas. Tout ce qui lit le PDF à l'écran se vérifie sur l'instance Brave de
la cure — `scripts/feuille-cdp.mjs` en est le second pilote, 31 épreuves sur la page qui
tourne, et il se vérifie mordant en mutant le code, comme un test.

**Une bascule charge la géométrie de sa cible avant de s'appliquer.** `geometrie()`
jette pour un format que personne n'a chargé, et c'est le bon choix — mais aucune
frontière d'erreur ne couvre l'arbre React : la levée démonte tout et il ne reste
**qu'une fenêtre blanche**, sans message ni journal. Le dump du format visé se charge
donc pendant l'aperçu (`chargeGeometrieFormat`, **dans le fond perdu de l'album**, pas
celui de l'écran de création qui vaut zéro et n'a jamais la bonne clé), et
`adopterGeometrie` refuse plutôt que de laisser appliquer un album que personne ne
saurait dessiner. L'ancien dump reste en cache, et c'est ce qui fait tenir ⌘Z sur une
bascule. `geometrie.test.ts` tient les quatre cas.

**Une UX ne se valide jamais au seul harnais navigateur** (`.claude/launch.json`).
**Installer le bundle après chaque push** (TCC pour le pilotage à l'écran), 19 Mo.
**Les artefacts de mise à jour n'existent qu'en release** (`tauri.release.conf.json`).

**Les panneaux passent par `Dialogue.tsx`, et par rien d'autre** (Préférences, À propos,
Stockage, Signaler, Raccourcis, Format, la fiche d'une photo, la confirmation de
commande) : rôle, `aria-modal`,
nom par le titre, focus dedans à l'ouverture et rendu à la fermeture, Tab qui cycle, et
`inert` posé sur tous les frères de l'overlay le temps qu'il vit (un second panneau ne
réveille pas l'éditeur en se fermant : chacun ne retire que ce qu'il a posé). Échap reste
dans la table clavier d'App, où chaque panneau a sa ligne, `bascule` comprise. **La
frontière d'erreur est `Frontiere.tsx`**, montée dans `main.tsx` : elle dit, elle ne
sauve pas — un album dont le rendu vient de lever n'est pas un album à réécrire par-dessus
le fichier qui s'ouvre encore.

**Un canvas casse l'accessibilité par nature** : les proxies DOM sont posés, et
`SceneProxies` doit rester une fonction de la scène. Le jour où il lit le gabarit ou une
chaîne du moteur, il cesse de servir les deux rendus.

**`album.root` est le dossier des photos, la police est à côté d'`album.json`.** Les
confondre donne un album qui marche tant que les deux coïncident et casse au premier
album rangé ailleurs. Et **l'écran ne nomme jamais une police installée** : ça marche
sur la machine qui l'a, donc le défaut ne se voit qu'ailleurs. Le test qui mord est
`font.test.ts`, qui lit la chaîne posée sur le contexte, pas une constante à côté.

**Jamais un octet écrit sur un original**, la retouche vit dans `album.json`. **Jamais
d'échec silencieux à l'export.**
 **Tout ce qui touche le PDF remesure la conformité**
(cinq tests Rust plus veraPDF) dans la vague où c'est écrit.

**Trois manières de mesurer une photo, et deux qui mentent.** `image::image_dimensions`
perd toute photo HEIC : la seule répartition du projet est `heic::dimensions`. Un en-tête
brut n'est pas orienté alors que `Photo::orig` l'est : passer par `heic::oriente`, sans
quoi toute photo couchée se lit paysage. Et `Releve::lire` recompose ses chemins depuis la
racine, donc une fiche relue porte `jeu/photo.jpg` là où un slot porte `photo.jpg` :
la clé est `Releve::src`. Les trois se sont trouvées en confrontant la bascule depuis les
photos et depuis les fiches ; aucune n'aurait été vue autrement, la troisième rendant un
succès parfait sur zéro travail.

## Décisions à ne pas rouvrir, côté code

Tauri 2 et React. GPL-3.0. `album.json` état unique réparable à la main. Le PDF fait foi.
Aucune image ne traverse le pli, **ni aucun objet libre** : le pli est dur, l'éditeur y
bute et le prévol refuse — et c'est ce qui rend la découpe en pages simples possible ; la
marge de sécurité est molle, on avertit et le linter compte.
Un `album.json` d'un schéma inconnu se refuse, jamais ne s'ouvre en perdant des champs.
Heuristiques d'abord, IA jamais
décisionnaire. Jamais de résolution sous 250 ppi **sans que l'auteur l'ait vue et acceptée**
(levé le 19/10, 1.1-c : l'export la nomme, la commande demande la case). Jamais `imazen/heic` (AGPL). Un objet
de scène porte un angle et une origine, **jamais une matrice**.

## Commandes

```bash
./scripts/check.sh
```

```bash
./target/release/booklibre ~/Pictures/colophon-testsets/corse-2013 -o .albums/corse-2013 --format carre-21 && ./target/release/booklibre --audit -o .albums/corse-2013
```

Autres drapeaux : `--print`, `--cover`, `--prevol --profil <id>`, `--densite`,
`--variantes`, `--legendes`, `--reprise`, `--bascule <FORMAT> [--essai]`, `--dump-scene`,
`--dump-geometry`, `--dump-ornements`, `--profils`, `--preparer <DOSSIER>`. Cachés :
`--fiche`, `--releve-album`, `--dates`, `--reserve`, `--remplir`, `--cover --apercu`, `--recomposer [--vers <FORMAT>]
[--rythme <ID>]`, `--devis <album> --pays FR` (le
code de commande, clé lue dans `CLOUDPRINTER_SANDBOX_API_KEY`, jamais en argument).
Scripts :
`pdfx.sh full`, `install-app.sh`, `fixture-scene.sh`, `ornement-encre.py`, `notices.sh`,
`apercu-fidele.py`, `pages-simples.py`,
`banc-gabarits.sh`, `mesure-cdp.mjs`, `feuille-cdp.mjs`, `police-cdp.mjs`. App :
`npm run tauri dev`.

Les deux bancs à la main de la police, sur l'instance Brave de la cure : poser une vraie
face du Mac dans un album sans passer par la fenêtre, puis mesurer la parité écran/papier
contre la référence du moteur.

```bash
COLOPHON_ALBUM=.albums/corse-2013 COLOPHON_FACE="Helvetica Neue Regular" \
  cargo test -p colophon-core --release banc_poser_une_face_du_mac -- --ignored --nocapture
```

```bash
COLOPHON_POLICE=.albums/corse-2013 cargo test -p colophon-core --release \
  banc_parite_ecran_papier -- --ignored --nocapture > /tmp/ref.json
```


## Architecture

Workspace Cargo. **`colophon-core`** : `scan` → `meta` → `thumb` → `analyze` → `face` →
`heic` → `pipeline` (curation) → `layout` (Composer, `Densite`) → `scene` → `pdf` →
`print` → `cover` → `audit` ; `build.rs` enchaîne. À côté : `font`, `icc`, `places`,
`ornement` (le pack), `pdfx`, `reprise`, `log`, `printer`, `prevol`, `recomposition`
(la recomposition d'un album composé, au même format ou à un autre),
`imposition` (la découpe en pages simples), `export` (le manifeste `export.json`,
l'empreinte d'un album, le dossier préparé, MD5 et SHA-256), `colophon` (la page),
`fiche` (la fiche d'une photo), `qualite` (la mesure de l'alerte), `legende` (la
proposition et les dates), `reserve`, `commande` (CloudCore), `depot` (le seau S3), `relais` (le client du relais
marchand).
**`colophon-cli`** : clap. **`colophon-app`** : React et Vite (`bridge.ts` seule porte,
`album.ts` géométries, `scene.ts` la scène et `hitTest`, `SceneCanvas.tsx` le peintre,
`SceneProxies.tsx` le clavier, `rendu.ts` l'interrupteur, `feuille.ts` le modèle de la
feuille qui tourne, `raster.ts` le PDF en bitmaps, `Feuilletage.tsx` la scène du
feuilletage, `photos.ts` vignettes décodées et badges, `font.ts` la mesure de texte sur
les octets de l'album, `police.ts` les noms et les refus d'une face,
`ornement.ts` le pack et le seul tracé que les deux rendus partagent,
`menu.ts`, `signaler.ts`, `pdfview.tsx`, `reasons.ts`, `icons.tsx`, `recents.ts`,
`envoi.ts` la logique pure d'Envoi, `commande.ts` celle de Commander, `CommandeView.tsx`
son parcours, `Livre3D.tsx` et `volume.ts` le livre en volume, `fond.ts` et
`PaletteFond.tsx` le fond de la couverture) plus
la coquille Tauri, marques d'icône dans `design/marques` (posée depuis le 28/10 : `pages.svg`, les
deux pages de la DA « La page calme », par `scripts/icone.sh`).


Chaîne de distribution : `NOTICES.md` et `src-tauri/notices.md`, la copie embarquée,
régénérés ensemble par `notices.sh` et identiques (le gate ne les compare pas), CSP réelle, CHANGELOG, README,
modèles d'issue, `check.yml` et `release.yml` (binaires, SHA-256, `latest.json`), updater
branché, `audit.yml` (avis RustSec, hebdomadaire, hors chemin de fusion). **La version vit
à quatre endroits et `scripts/versions.py` est le seul à les comparer** : dans le gate sans
argument, dans `release.yml` avec `--attendue <tag>`, qui refuse aussi un CHANGELOG sans
section `## [<tag>]`, et qui ne construit rien avant un `check.sh` vert. Le gate construit
le bundle Vite et garde le petit dossier (`scripts/petit-dossier.py`, 24/25/30/2 photos
depuis la fiche de corse). Windows ne sort que le MSI (`bundle.targets`), pour une seule
forme d'artefact de mise à jour. **Le DMG est signé Developer ID et notarisé** quand les
cinq secrets Apple sont posés, tous ou aucun (`release.yml` refuse un jeu partiel) :
Tauri signe l'app en runtime durci, la notarise et l'agrafe ; `scripts/signature-macos.sh`
notarise le DMG, que Tauri laisse, puis vérifie au lieu de supposer. **Le runtime durci
refuse la photothèque** à qui ne la déclare pas : sans `entitlements.plist`
(`personal-information.photos-library`), seule une release signée rencontrerait ce refus,
jamais le bundle ad hoc du Mac de dev. Vérifié le 10/10 : un DMG Developer ID notarisé
par Apple, agrafé, accepté par `spctl`, installé, demande la photothèque et en affiche les
albums. La forme du refus sans la clé n'est pas mesurée. `signature-macos.sh verifier`
tourne aussi en local avec un certificat Apple Development.
**La version Mac App Store est une saveur, pas une branche** (MAS-s1) : feature Cargo
`mas` et `VITE_COLOPHON_MAS=1`, construite par `scripts/mas.sh build|pkg|valider|envoyer`
seulement, jamais par la CI ni le DMG. Elle n'enregistre pas l'updater et ne charge pas
`capabilities/maj.json` (`build.rs`), `tauri.appstore.conf.json` retire `plugins.updater`
par fusion (`null`), et `mas.sh` le **prouve sur le binaire** (ni `latest.json` ni la clé).
Sous bac à sable, `$HOME` est le conteneur : tout dossier de la personne passe par
`core::maison::vrai_dossier_personnel` (getpwuid), et `~/Library/Fonts` n'est pas listé
(`font::installed_avec`). `album.root` se rouvre par un **marque-page de sécurité**,
`root.bookmark` à côté d'`album.json` (`signet.rs`, `signet.m`), résolu dans `open_album`
avant toute lecture ; sans lui, `dossier_a_revoir` et le bouton qui rechoisit le dossier.
Un dossier déplacé, le marque-page le suit : `open_album` réécrit alors `album.root`
(`signet::suivre`, même nom et mêmes photos, sinon la personne le revoit). Un refus de
`revoir_dossier` est un code `refus-revoir:…` que `racine.ts` traduit en phrase.
**Une app signée « Apple Distribution » ne se lance pas sur ce Mac** (AMFI, « No matching
profile found ») : l'essai à l'écran se fait par `mas.sh build --essai`, ad hoc même quand le
`.env` porte les identités. Le `CFBundleVersion` est le nombre de commits de `HEAD`
(`COLOPHON_MAS_BUILD` le force) : unique et croissant si l'on envoie depuis `main` à jour.
Ouvrir, révéler, naviguer passent par NSWorkspace en `mas`, par `open` ailleurs. Les droits
sont exactement ceux de `entitlements.mas.plist`, un test les compte ; **`network.client` y est
pour WebKit**, pas pour l'app : sans lui, sous bac à sable, WebContent meurt au lancement et la
fenêtre reste blanche (mesuré le 15/10). **Le dialogue d'enregistrement n'accorde que le fichier
choisi**, jamais son dossier : un `.part` ou un second fichier d'à côté est refusé. D'où
`print::poser_pdf` (repli par le dossier temporaire) et le dialogue propre à la couverture en `mas`
(qu'aucun bouton n'atteint aujourd'hui : Cloudprinter passe par « Préparer », qui choisit un
dossier, et « PDF sans contrainte » n'écrit qu'un fichier). La GPL tient par la
permission additionnelle de `LICENSE-APPSTORE.md`.
**`thumb::chemin` est la seule porte vers une vignette
du cache**, six lecteurs y passent, un nom qui n'est pas un nom de fichier est refusé.
**`url_de_rapport` est un jeu fermé**, et Windows ouvre par `rundll32`, jamais `cmd`.
Reste le parcours de correction.

Le prompt de la session en cours, quand il y en a un : `docs/prompts/en-cours.md`.
Mesures : `docs/mesures/`. Glossaire : `CONTEXT.md`.
