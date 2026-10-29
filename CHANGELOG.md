# Journal des versions

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), et
la numérotation le [versionnage sémantique](https://semver.org/lang/fr/).

Une release par semaine le premier mois qui suit le lancement. Les binaires
macOS et Windows, leurs empreintes SHA-256 et ces notes sont publiés ensemble
sur la page des releases.

## [Non publié]

### Changed

- A new album opens on a blank page 1: no photo sits right behind the cover
  any more. Bringing a photo back from the sorting view, pasting, dropping or
  moving a spread never puts one there either.
- A new text block starts at 26 pt instead of 12, readable on screen.
- The cover photo picker marks the photos too small for the cover sheet of
  your printer, with the resolution they would print at.

### Fixed

- A text block or an ornament can be dragged across the fold to the right
  page; it used to stop at the fold of the page it was born on.

## [1.1.0] - 2026-10-28

### Added

- Order the printed album from Booklibre, for delivery in France. Send →
  Order quotes the book, uploads the two PDFs, and opens the payment on
  Stripe's page in your browser; the album is printed and shipped by
  Cloudprinter. Your card, name and address are typed on Stripe's page, never
  in the app. An order can be cancelled from the app until production starts,
  and is then refunded. The PDF export stays free, offline and without an
  account. The Mac App Store version has no ordering.

- While you build an album, the foot of the book and of the spreads shows what
  it would cost printed and delivered by Booklibre ("From €38 delivered"),
  following the number of pages. Only if you answer "Through Booklibre" to "How
  will you print it?" (welcome screen, then Preferences): the app then reads a
  price table once per launch, with nothing of your album. "Myself" turns it
  off. The Mac App Store version neither asks nor calls.

### Changed

- A new album opens on a photograph: the half-title page is no longer added
  (its days and towns stay on the colophon page, and you can still tick it in
  Send). The first left page, pasted inside the cover at Cloudprinter, is
  shown greyed out in the editor and in the spread table, and anything dropped
  on it is refused with a sentence saying why. Albums composed before keep
  their half-title. On the command line, `--garde` brings it back.

- A photo under 250 ppi no longer blocks the export. Send names the pages it
  is on and lets you decide: "Download the PDF" asks once, naming them, then
  exports; ordering through Booklibre asks you to tick a box that names them.
  What the printer refuses (size, page count, the cut, the fold, the files)
  still blocks.

- Prices no longer say "incl. VAT": the price shown is the price paid.

- Dates and places under the photos: Spread → Dates and places under the
  photos sets what is written under each photo (nothing, the date, the place,
  or both, in three date forms), and the New album screen offers it as a box,
  unticked. Only what the photo knows is written: a reliable capture date, a
  town within 30 km. A full-page photo has no room underneath, and gets
  nothing.

- The album saves itself, a second after each change: no more "unsaved
  changes" questions when closing, opening another album or quitting. ⌘Z still
  undoes, and File → Revert to the start of the session brings back the album
  as it was when you opened it.

- Colophon is now called Booklibre: the window, the menus, the installers, the
  command line (`booklibre`), the last page of the book ("Composed with
  Booklibre") and the PDF metadata. Your albums, preferences and imported
  photos stay where they were, and the app updates in place on macOS and
  Windows. On a Mac updated from 1.0.1, the application keeps its file name,
  `Colophon.app`, in the Applications folder. An album composed before keeps
  its last page as it was written, with the old name, until you tick that page
  again or recompose the album.
- The website is now [booklibre.app](https://booklibre.app), and the contact
  address hello@booklibre.app. The update check asks booklibre.app first, then
  GitHub.

## [1.0.1] - 2026-10-18

First public release. Colophon takes a folder of photos and turns it into a
composed album, spread by spread, in under a minute. You can rework any of it
by hand, then export a 300 dpi PDF for whichever printer you choose. Free,
offline, no account, GPL-3.0.

Requires macOS 11 or later. The app is signed with an Apple Developer ID and
notarized by Apple, so it opens on first launch like any other app. Windows has
an unsigned installer, but
nobody has run it on a real Windows machine yet. Unless you send a file
somewhere yourself, only one request ever leaves your computer: the update
check sent to GitHub at launch, which you can turn off in Preferences.

### Fixed

- Making an album from Apple Photos works. The app used to reject the list of
  photos it had just read from the library, so no import from Photos ever finished.
- A Photos library whose albums the app can read is no longer reported as
  unreachable. The app trusted the library location macOS reports, which can still
  point at a folder that was deleted.
- When an import from Photos fails, the window now says why, instead of staying
  silent.
- Windows: the next versions will reach you through the in-app update. The signed
  update package was built but left out of the release, so a Windows install would
  never have been offered a newer version.

Everything Colophon does is listed under 1.0.0 in
[CHANGELOG.md](https://github.com/alexis-morain/colophon/blob/main/CHANGELOG.md),
and all of it is part of this release.

## [1.0.0] - 2026-10-15

Tagged but never published: its import from Apple Photos was broken. 1.0.1 is the
first public release and contains everything below.

### Added

#### Compose

- A folder goes in, with no import step and no catalogue. JPEG and PNG work
  everywhere; HEIC and RAW work on macOS through the system decoder. On a Mac,
  an album from Apple Photos is copied into a visible folder, and the photo
  library is never modified.
- Curation drops near-duplicates, bursts and repeated shots of a scene,
  keeping the sharpest frame or the one you starred, plus panoramas too wide
  for the page, photos too small for the format, and whatever a full book has
  no room for. The Sort view shows every dropped photo, the reason, and the
  frame it lost to; a double-click rescues it.
- Lightroom star ratings and rejects (XMP) count, a Google Takeout export
  keeps its dates, and a folder of scans with no EXIF still makes an album.
- Three albums from a single analysis, at different paces and lengths, with a
  summary: photos read, kept, dropped, and why.
- Six page formats. Switching format keeps the spreads, the photos and the
  crops.
- A half-title (title, dates, towns) at the front and a colophon page at the
  back, each removed with one click.
- The words Colophon writes into the book (half-title, colophon page,
  chapter lines, cover subtitle, caption dates) are in the language of the
  interface at the time they are written: compose in English, get an English
  book. Switching language later leaves an album as it is.
- A new album comes with its cover: one of the book's best-rated photos,
  landscape or square, the album's title, and the dates of the half-title.
  Change it in Cover; a recomposition keeps the one you chose.

#### Edit

- Crop inside a cell, replace a photo or add one from the reserve with a
  right-click, reorder and duplicate spreads, pick from at most 23 named
  dispositions, write captions, rename chapters, lay out the cover.
- Exposure, contrast and black and white, without a byte of the original
  changing.
- Text blocks and 35 ornaments, public domain or CC0, that go anywhere on a
  spread and can be turned, coloured, copied and pasted.
- The book's typeface: a shortlist of ten families, and every typeface on the
  computer one click behind it. It is copied into the album's folder, so the
  album prints the same on another computer.
- A photo that will print badly carries a triangle in the corner of its cell,
  "N ppi, under 250", "dark" or "blurry", which VoiceOver reads too. The same
  sign marks it in Sort, and Spreads puts an alert dot on its spread.
- "Date every caption" writes the shooting date at the end of each caption,
  "Date it" does it for one photo, and "Remove the dates from the captions"
  takes them off again.
- Photo info (⌘I), and "Show the original" in the Finder or Explorer.
- ⌘Z undoes everything. Quitting, closing or opening another album asks
  before discarding anything.

#### Export

- A 300 dpi PDF with its typeface embedded, PDF/A-2b checked with veraPDF,
  PDF/X-4 declared, plus a light preview PDF for the screen.
- The Faithful preview (⇧⌘P) shows the PDF itself, not a drawing of the
  spread.
- The preflight blocks what the guillotine would cut or the binding would
  swallow, warns about the safe zone, and for every problem names the spread
  and the gesture that fixes it. No photo ever prints below 250 ppi.
- In Send, every problem reads as one plain sentence, in French or English,
  with the button that leads to the fix: the spread, the cover with its photo
  picker open, the Format panel, or Prepare again. The figures (ppi,
  millimetres, file names) wait under Details.
- The preflight also blocks a file exported for another printer, or one older
  than the album's last change. It judges the files in the folder Send
  prepares, and no longer the PDFs an earlier export left next to the album.

#### Print

- Any printer that accepts a PDF: no watermark, no logo, no partner you have
  to use. In Send, "Another printer? A PDF without constraints" writes one
  file, cover included.
- "Prepare for Cloudprinter" writes a folder with the interior, the cover as
  one flat sheet with its spine, `export.json`, and `fiche.txt`, which lists
  what the order form asks for (format, pages, paper, file sizes, SHA-256 and
  MD5). The preflight then runs again on the files written.
- The command line ships four profiles: Cloudprinter, Prodigi, Lulu and a
  generic one. Binders that work page by page get the interior as single
  pages.

#### Reporting and updates

- Help, Report a problem: the app builds the report, you read it before it
  goes, and it carries no photo, no path and no GPS coordinate. After an
  export, Send asks for your verdict on the first draft.
- Updates arrive through the app, signed, and nothing installs without a
  click.
- French and English, a dark mode that follows the system, a Storage panel to
  see and clear what the app has written, and an About screen with the
  licences.

### Changed

Compared with 0.9.0, described below, which was built but never distributed:

- *Install.* macOS 11 at minimum, and the macOS app is now signed with a
  Developer ID and notarized. Windows now ships only as an MSI installer.
- *Compose.* A photo with no date, no GPS and no stars is no longer dropped
  for lacking a camera fingerprint when most of the folder lacks one too: the
  filter switches itself off and says so.
- *Edit.* The picker shows at most 23 named dispositions instead of 171
  template names. A block's settings open right below the block. Preferences
  no longer carry the Elements or Canvas switch, which changed nothing in the
  book.
- *Print.* Send no longer offers four profiles: it prepares for Cloudprinter,
  or writes a plain PDF for any other printer. Prodigi and Lulu stay on the
  command line (`--profil`). A new album comes with a cover, and a cover with
  no photo blocks a printer that takes it as a separate file. Cloudprinter
  takes 24 to 800 pages. A format it does not print, such as landscape
  28 × 21 or a custom size, blocks the preparation and points to Format.
- *Export.* A photo too small for its cell still blocks the export, but its
  line in Send carries three gestures: Show, Replace, Remove from spread.
  Remove always works.
- *Formats.* Portrait 20 × 25 is 203.2 mm wide, a true 8 × 10 inches,
  instead of 203.0.
- *Updates.* The update check is documented in the README and in the security
  policy, and Preferences turns it off.

### Fixed

- *Compose.* Composing a folder that already has an album no longer
  overwrites it: the new album is written next to the old one, and the app
  offers to reopen the old one.
- *Edit.* Saving no longer deletes a copy of `album.json` you made by hand. A
  photo moved onto a text page no longer wipes out its text. Spreads reorder
  with the mouse as well as with the keyboard. A photo dragged from the
  reserve drawer onto a cell lands there. An error no longer leaves a blank
  window, and the panels work with the keyboard and with a screen reader.
- *Export.* A title like "Zażółć" prints as written, not as question marks. A
  character the typeface cannot draw shows as "?" on screen, exactly as on
  paper, and Send names it. A truncated JPEG no longer goes to print with a
  grey band where the image stops: the export halts and names the file.
- *Reporting.* Bug reports no longer go out empty on Windows.
- *Security.* The interface no longer handles any path on disk, and a
  thumbnail name that points outside the cache is refused.
- *Security.* The update check and the CLI's network calls use rustls
  0.23.45, which fixes RUSTSEC-2026-0285 (TLS 1.3 handshake messages accepted
  across encryption levels).

## [0.9.0] - 2026-08-17

Première version candidate publique. Le moteur, l'éditeur et l'export sont
là ; il manque la signature des binaires, la mise à jour automatique et
l'icône définitive.

### Ajouté

- **Trois propositions au lieu d'une.** Le même dossier donne trois albums
  qui diffèrent par le rythme et la longueur, composés d'une seule analyse.
  L'écran de fin de composition devient un écran de choix, et les deux
  propositions écartées restent récupérables jusqu'à la première retouche.
- **Page de colophon.** Une dernière page discrète : photos retenues sur
  photos lues, période couverte, villes traversées, appareils utilisés,
  format et papier. Activée par défaut, retirable d'un clic depuis Envoi.
  Elle ne porte jamais un chemin, une coordonnée ni une légende.
- **Aperçu fidèle (⇧⌘P).** La vue Livre lit le PDF plutôt que de le
  redessiner : ce qui est à l'écran est le fichier, glyphes et rognages
  compris. Rendu par pdf.js, sans réseau.
- **Panneau Stockage** (Fichier → Stockage…) : ce que l'application a écrit
  sur le disque, album par album, avec la suppression et la purge des
  caches de vignettes. Les photos d'origine ne sont jamais touchées.
- **Rendre une planche à l'automatique.** Le cadenas avait une porte
  d'entrée sans sortie ; la planche reprend la composition proposée au
  départ, et la mesure de reprise cesse de la compter.
- **Titre d'album modifiable** depuis la barre, la couverture suivant tant
  qu'elle n'a pas de titre à elle.
- **Écran À propos** : version, licence GPL-3.0, notices des licences
  tierces embarquées, et l'attribution GeoNames qu'exige la CC BY 4.0.

### Modifié

- Politique de sécurité de contenu réelle à la place de l'absence de
  politique : plus rien ne peut être chargé depuis le réseau.
- Le linter passe désormais les trois propositions, sur les trois jeux de
  référence et les six formats.

### Sécurité

- La commande qui supprime un album ne peut atteindre qu'un enfant direct du
  dossier de données, liens symboliques résolus des deux côtés. Un dossier
  de photos n'est jamais atteignable depuis l'application.
