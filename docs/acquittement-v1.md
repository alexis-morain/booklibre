# L'acquittement, version 1

Le libellé de la case que l'auteur coche pour faire imprimer un livre dont des
pages sont sous 250 ppi (session 1.1-c). Le relais garde, avec l'intention,
`version: 1`, les pages et leur ppi, et l'heure : ce fichier est le texte que
`version: 1` désigne, mot pour mot. Changer un mot de l'un de ces quatre
libellés, c'est passer à la version 2, et garder celui-ci.

`{pages}` est remplacé par les pages en clair : « la couverture », « la page
5 », « les pages 5, 13 et 21 », « la couverture et la page 5 » ; en anglais
« the cover », « page 5 », « pages 5, 13 and 21 », « the cover and page 5 ». Les
pages sont celles du livre : la planche k porte les pages 2k − 2 (gauche) et
2k − 1 (droite), la couverture est la page 1 de son fichier.

`i18n.test.ts` exige que les chaînes de l'app soient celles-ci.

## Français

Une page :

> Je sais que {pages} risque d’imprimer floue (sous 250 ppi), et j’accepte que sa netteté s’écarte de celle d’un livre photo.

Plusieurs pages :

> Je sais que {pages} risquent d’imprimer floues (sous 250 ppi), et j’accepte que leur netteté s’écarte de celle d’un livre photo.

## English

One page:

> I know {pages} may print blurry (under 250 ppi), and I accept that its sharpness falls short of a photo book’s.

Several pages:

> I know {pages} may print blurry (under 250 ppi), and I accept that their sharpness falls short of a photo book’s.
