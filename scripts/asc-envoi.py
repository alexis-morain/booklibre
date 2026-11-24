"""Le .pkg de la version Mac App Store, vérifié puis envoyé par l'API App Store
Connect (`buildUploads`), sans altool.

Usage : python3 scripts/asc-envoi.py valider <pkg>
        python3 scripts/asc-envoi.py envoyer <pkg>
        python3 scripts/asc-envoi.py --test

`xcrun altool` est cassé depuis Xcode 27.2 (« Defaults.properties »
introuvable, même pour --list-apps). L'API n'a pas d'équivalent de
`--validate-app` : `valider` ne fait plus passer le binaire par les contrôles
d'Apple. Il vérifie ce qui se vérifie sans rien déposer (voir `valider`), et
les refus d'Apple sur le binaire n'arrivent plus qu'après l'envoi, dans l'état
FAILED du buildUpload, que `envoyer` affiche.

La clé d'API se lit dans l'environnement, posé par `mas.sh` depuis le .env de
gestion : APPLE_MAS_API_KEY_ID, APPLE_MAS_API_ISSUER, APPLE_MAS_API_KEY_PATH
(le fichier .p8). Ni la clé ni le jeton ne s'affichent jamais. Le jeton est
signé par `openssl` : bibliothèque standard seulement, rien à installer.
"""
import base64
import glob
import hashlib
import json
import os
import plistlib
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request

API = "https://api.appstoreconnect.apple.com"
INSTALLEUR = "3rd Party Mac Developer Installer"


class Refus(Exception):
    pass


# ---------------------------------------------------------------------------
# Le jeton : JWT ES256, signé par openssl.
# ---------------------------------------------------------------------------

def b64url(octets):
    return base64.urlsafe_b64encode(octets).rstrip(b"=").decode()


def der_vers_brut(der):
    """Une signature ECDSA DER (ce que rend openssl) en r||s sur 64 octets
    (ce que demande JWS)."""
    def entier(buf, i):
        if buf[i] != 0x02:
            raise Refus("signature DER illisible")
        n = buf[i + 1]
        valeur = buf[i + 2:i + 2 + n].lstrip(b"\x00")
        if len(valeur) > 32:
            raise Refus("signature DER illisible")
        return valeur.rjust(32, b"\x00"), i + 2 + n

    if len(der) < 8 or der[0] != 0x30:
        raise Refus("signature DER illisible")
    i = 3 if der[1] & 0x80 else 2
    r, i = entier(der, i)
    s, _ = entier(der, i)
    return r + s


def jeton(cle_id, emetteur, signer, maintenant):
    entete = b64url(json.dumps({"alg": "ES256", "kid": cle_id, "typ": "JWT"}, separators=(",", ":")).encode())
    charge = b64url(json.dumps(
        {"iss": emetteur, "iat": maintenant, "exp": maintenant + 1100, "aud": "appstoreconnect-v1"},
        separators=(",", ":"),
    ).encode())
    a_signer = f"{entete}.{charge}".encode()
    return f"{entete}.{charge}.{b64url(der_vers_brut(signer(a_signer)))}"


def signer_openssl(chemin_cle):
    def signer(donnees):
        r = subprocess.run(
            ["openssl", "dgst", "-sha256", "-sign", chemin_cle],
            input=donnees, capture_output=True,
        )
        if r.returncode != 0:
            # Le message d'openssl ne porte pas la clé, seulement son chemin.
            raise Refus("openssl n'a pas signé le jeton : " + r.stderr.decode(errors="replace").strip()[:200])
        return r.stdout
    return signer


# ---------------------------------------------------------------------------
# Les corps de requête et leur lecture : pur, donc testé.
# ---------------------------------------------------------------------------

def corps_envoi(app_id, version, build):
    return {"data": {
        "type": "buildUploads",
        "attributes": {"cfBundleShortVersionString": version, "cfBundleVersion": build, "platform": "MAC_OS"},
        "relationships": {"app": {"data": {"type": "apps", "id": app_id}}},
    }}


def corps_fichier(envoi_id, nom, taille):
    return {"data": {
        "type": "buildUploadFiles",
        "attributes": {"assetType": "ASSET", "fileName": nom, "fileSize": taille, "uti": "com.apple.pkg"},
        "relationships": {"buildUpload": {"data": {"type": "buildUploads", "id": envoi_id}}},
    }}


def corps_fin(fichier_id, md5):
    return {"data": {
        "type": "buildUploadFiles", "id": fichier_id,
        "attributes": {"uploaded": True, "sourceFileChecksums": {"file": {"hash": md5, "algorithm": "MD5"}}},
    }}


def morceaux(operations, taille):
    """Les morceaux qu'Apple demande, refusés s'ils ne couvrent pas le fichier
    exactement, une fois, dans l'ordre."""
    ops = sorted(operations, key=lambda o: o["offset"])
    attendu = 0
    for op in ops:
        if op["offset"] != attendu or op["length"] <= 0:
            raise Refus(f"morceaux incohérents : {op['offset']} au lieu de {attendu}")
        attendu += op["length"]
    if attendu != taille:
        raise Refus(f"les morceaux couvrent {attendu} octets sur {taille}")
    return [
        (op["method"], op["url"], {h["name"]: h["value"] for h in op.get("requestHeaders") or []},
         op["offset"], op["length"])
        for op in ops
    ]


def etat_envoi(attributs):
    """(état, messages) d'un buildUpload. L'état vaut None tant qu'Apple
    traite ; COMPLETE ou FAILED à la fin."""
    etat = attributs.get("state") or {}
    if isinstance(etat, str):
        etat = {"state": etat}
    messages = []
    for genre in ("errors", "warnings", "infos"):
        for m in etat.get(genre) or []:
            texte = m.get("description") or m.get("message") or json.dumps(m, ensure_ascii=False)
            code = m.get("code")
            messages.append(f"{genre[:-1]} : {code + ' : ' if code else ''}{texte}")
    fin = etat.get("state") if etat.get("state") in ("COMPLETE", "FAILED") else None
    return fin, messages


def numero_libre(versions_deja_envoyees, build):
    """Refuse un CFBundleVersion déjà envoyé ou plus petit qu'un envoi passé :
    Apple le refuserait après l'envoi, on le dit avant."""
    nombres = [int(v) for v in versions_deja_envoyees if str(v).isdigit()]
    if int(build) in nombres:
        raise Refus(f"le build {build} existe déjà chez Apple : refaire `mas.sh build` depuis `main` à jour")
    if nombres and int(build) < max(nombres):
        raise Refus(f"le build {build} est plus petit que le dernier envoyé ({max(nombres)})")


def verifier_app(info, signature_pkg, a_un_profil, bac_a_sable):
    """Ce que le .pkg doit porter pour partir. Rend (bundle, version, build)."""
    if INSTALLEUR not in signature_pkg:
        raise Refus(f"le .pkg n'est pas signé « {INSTALLEUR} »")
    if not a_un_profil:
        raise Refus("l'app du .pkg n'embarque pas de profil : signée ad hoc, elle ne part pas chez Apple")
    if not bac_a_sable:
        raise Refus("la signature de l'app du .pkg ne porte pas app-sandbox à vrai")
    bundle = info.get("CFBundleIdentifier")
    version = info.get("CFBundleShortVersionString")
    build = info.get("CFBundleVersion")
    if not bundle or not version or not build:
        raise Refus("Info.plist sans identifiant, version ou build")
    if not str(build).isdigit():
        raise Refus(f"CFBundleVersion vaut {build}, attendu un entier")
    return bundle, version, str(build)


# ---------------------------------------------------------------------------
# Le réseau et le Mac.
# ---------------------------------------------------------------------------

class Client:
    def __init__(self, cle_id, emetteur, chemin_cle):
        self.cle_id, self.emetteur = cle_id, emetteur
        self.signer = signer_openssl(chemin_cle)

    def appel(self, methode, chemin, corps=None):
        url = chemin if chemin.startswith("http") else API + chemin
        jwt = jeton(self.cle_id, self.emetteur, self.signer, int(time.time()))
        donnees = json.dumps(corps).encode() if corps is not None else None
        req = urllib.request.Request(url, data=donnees, method=methode, headers={
            "Authorization": "Bearer " + jwt, "Content-Type": "application/json"})
        try:
            with urllib.request.urlopen(req, timeout=120) as r:
                texte = r.read()
        except urllib.error.HTTPError as e:
            raise Refus(f"{methode} {chemin.split('?')[0]} : {e.code} {e.read().decode(errors='replace')[:500]}")
        return json.loads(texte) if texte else {}

    def tout(self, chemin):
        donnees, suivant = [], chemin
        while suivant:
            page = self.appel("GET", suivant)
            donnees += page.get("data", [])
            suivant = (page.get("links") or {}).get("next")
        return donnees


def client_depuis_env():
    cle_id = os.environ.get("APPLE_MAS_API_KEY_ID", "")
    emetteur = os.environ.get("APPLE_MAS_API_ISSUER", "")
    chemin = os.path.expanduser(os.environ.get("APPLE_MAS_API_KEY_PATH", ""))
    if not cle_id or not emetteur:
        raise Refus("APPLE_MAS_API_KEY_ID et APPLE_MAS_API_ISSUER manquent")
    if not chemin or not os.path.isfile(chemin):
        raise Refus("APPLE_MAS_API_KEY_PATH ne désigne aucun fichier .p8")
    return Client(cle_id, emetteur, chemin)


def lire_pkg(pkg):
    """Ouvre le .pkg et lit l'app qu'il installe, sans rien supposer."""
    sig = subprocess.run(["pkgutil", "--check-signature", pkg], capture_output=True, text=True).stdout
    tmp = tempfile.mkdtemp(prefix="mas-pkg-")
    try:
        sortie = os.path.join(tmp, "x")
        subprocess.run(["pkgutil", "--expand-full", pkg, sortie], check=True, capture_output=True)
        apps = glob.glob(os.path.join(sortie, "**", "Payload", "*.app"), recursive=True)
        if len(apps) != 1:
            raise Refus(f"le .pkg devrait porter une app, il en porte {len(apps)}")
        app = apps[0]
        with open(os.path.join(app, "Contents", "Info.plist"), "rb") as f:
            info = plistlib.load(f)
        if subprocess.run(["codesign", "--verify", "--strict", app], capture_output=True).returncode != 0:
            raise Refus("la signature de l'app du .pkg est invalide")
        droits = subprocess.run(["codesign", "-d", "--entitlements", "-", "--xml", app], capture_output=True).stdout
        bac = bool(droits) and plistlib.loads(droits).get("com.apple.security.app-sandbox") is True
        profil = os.path.isfile(os.path.join(app, "Contents", "embedded.provisionprofile"))
        return verifier_app(info, sig, profil, bac)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def trouver_app(client, bundle):
    apps = client.appel("GET", "/v1/apps?" + urllib.parse.urlencode({"filter[bundleId]": bundle}))["data"]
    if len(apps) != 1:
        raise Refus(f"aucune app App Store Connect pour {bundle}")
    return apps[0]["id"]


def builds_envoyes(client, app_id):
    q = urllib.parse.urlencode({"filter[app]": app_id, "limit": 200, "fields[builds]": "version"})
    return [b["attributes"]["version"] for b in client.tout("/v1/builds?" + q)]


def valider(pkg, client):
    bundle, version, build = lire_pkg(pkg)
    print(f"asc-envoi : {bundle} {version} ({build}), .pkg signé « {INSTALLEUR} », profil embarqué, bac à sable à vrai")
    app_id = trouver_app(client, bundle)
    print(f"asc-envoi : clé d'API acceptée, app {app_id}")
    numero_libre(builds_envoyes(client, app_id), build)
    print(f"asc-envoi : build {build} libre et plus grand que les envois passés")
    print("asc-envoi : les contrôles d'Apple sur le binaire ne tournent qu'à l'envoi (pas d'équivalent API à --validate-app)")
    return app_id, version, build


def envoyer(pkg, client, attendre=time.sleep, delai=30, essais=60):
    app_id, version, build = valider(pkg, client)
    with open(pkg, "rb") as f:
        octets = f.read()
    envoi = client.appel("POST", "/v1/buildUploads", corps_envoi(app_id, version, build))["data"]
    fichier = client.appel("POST", "/v1/buildUploadFiles", corps_fichier(envoi["id"], os.path.basename(pkg), len(octets)))["data"]
    parts = morceaux(fichier["attributes"]["uploadOperations"], len(octets))
    print(f"asc-envoi : envoi {envoi['id']}, {len(parts)} morceaux")
    for methode, url, entetes, debut, longueur in parts:
        req = urllib.request.Request(url, data=octets[debut:debut + longueur], method=methode, headers=entetes)
        try:
            urllib.request.urlopen(req, timeout=600).close()
        except urllib.error.HTTPError as e:
            raise Refus(f"morceau {debut} : {e.code}")
    client.appel("PATCH", f"/v1/buildUploadFiles/{fichier['id']}", corps_fin(fichier["id"], hashlib.md5(octets).hexdigest()))
    print("asc-envoi : déposé, traitement chez Apple")

    for _ in range(essais):
        fin, messages = etat_envoi(client.appel("GET", f"/v1/buildUploads/{envoi['id']}")["data"]["attributes"])
        if fin:
            for m in messages:
                print("asc-envoi :", m)
            if fin == "FAILED":
                raise Refus("Apple a refusé le binaire (messages ci-dessus)")
            break
        attendre(delai)
    else:
        print("asc-envoi : toujours en traitement, à suivre dans App Store Connect")
        return 3

    q = urllib.parse.urlencode({"filter[app]": app_id, "filter[version]": build,
                                "filter[preReleaseVersion.version]": version, "fields[builds]": "processingState"})
    for _ in range(essais):
        builds = client.appel("GET", "/v1/builds?" + q)["data"]
        etat = builds[0]["attributes"]["processingState"] if builds else None
        if etat in ("VALID", "FAILED", "INVALID"):
            print(f"asc-envoi : build {build} {etat}")
            return 0 if etat == "VALID" else 1
        attendre(delai)
    print("asc-envoi : envoi COMPLETE, build pas encore VALID, à suivre dans App Store Connect")
    return 3


# ---------------------------------------------------------------------------
# Les tests : `--test`, bibliothèque standard, donc partout.
# ---------------------------------------------------------------------------

def tests():
    def refuse(f, mot):
        try:
            f()
        except Refus as e:
            assert mot in str(e), (mot, str(e))
        else:
            raise AssertionError(f"« {mot} » aurait dû être refusé")

    # DER → r||s : r avec l'octet 0x00 de signe, s court qu'on complète.
    r, s = b"\x80" + b"\x11" * 31, b"\x22" * 30
    der = b"\x30" + bytes([4 + 33 + 30]) + b"\x02\x21\x00" + r + b"\x02" + bytes([30]) + s
    brut = der_vers_brut(der)
    assert brut == r + b"\x00\x00" + s, brut.hex()
    assert len(brut) == 64
    refuse(lambda: der_vers_brut(b"\x31\x06\x02\x01\x01\x02\x01\x01"), "DER")

    # Le jeton : trois parties, l'en-tête et la charge qu'Apple attend, la
    # signature qui vient du signataire, convertie.
    vu = []
    def signataire(d):
        vu.append(d)
        return b"\x30\x06\x02\x01\x07\x02\x01\x09"
    t = jeton("CLE", "EMETTEUR", signataire, 1000)
    a, b, c = t.split(".")
    pad = lambda x: x + "=" * (-len(x) % 4)
    assert json.loads(base64.urlsafe_b64decode(pad(a))) == {"alg": "ES256", "kid": "CLE", "typ": "JWT"}
    charge = json.loads(base64.urlsafe_b64decode(pad(b)))
    assert charge == {"iss": "EMETTEUR", "iat": 1000, "exp": 2100, "aud": "appstoreconnect-v1"}, charge
    assert vu == [f"{a}.{b}".encode()]
    assert base64.urlsafe_b64decode(pad(c)) == b"\x00" * 31 + b"\x07" + b"\x00" * 31 + b"\x09"

    # Les corps : ceux qu'a acceptés Apple le 23/11 (build 228).
    e = corps_envoi("1", "1.2.0", "228")["data"]
    assert e["attributes"] == {"cfBundleShortVersionString": "1.2.0", "cfBundleVersion": "228", "platform": "MAC_OS"}
    assert e["relationships"]["app"]["data"] == {"type": "apps", "id": "1"}
    f = corps_fichier("E", "Booklibre.pkg", 42)["data"]
    assert f["attributes"]["uti"] == "com.apple.pkg" and f["attributes"]["fileSize"] == 42
    assert f["relationships"]["buildUpload"]["data"]["id"] == "E"
    fin = corps_fin("F", "abc")["data"]
    assert fin["id"] == "F" and fin["attributes"]["uploaded"] is True
    assert fin["attributes"]["sourceFileChecksums"]["file"] == {"hash": "abc", "algorithm": "MD5"}

    # Les morceaux : remis dans l'ordre, en-têtes à plat ; trou, recouvrement
    # ou fichier mal couvert refusés.
    ops = [
        {"method": "PUT", "url": "u2", "offset": 5, "length": 5, "requestHeaders": None},
        {"method": "PUT", "url": "u1", "offset": 0, "length": 5, "requestHeaders": [{"name": "Content-Type", "value": "x"}]},
    ]
    m = morceaux(ops, 10)
    assert [x[1] for x in m] == ["u1", "u2"] and m[0][2] == {"Content-Type": "x"} and m[1][2] == {}
    refuse(lambda: morceaux(ops, 11), "couvrent 10")
    refuse(lambda: morceaux([dict(ops[1]), dict(ops[0], offset=4)], 10), "incohérents")

    # L'état : en cours, fini, refusé avec ses messages.
    assert etat_envoi({}) == (None, [])
    assert etat_envoi({"state": {"state": "PROCESSING"}}) == (None, [])
    assert etat_envoi({"state": {"state": "COMPLETE", "errors": [], "warnings": []}}) == ("COMPLETE", [])
    fin, msgs = etat_envoi({"state": {"state": "FAILED", "errors": [{"code": "90296", "description": "sandbox"}]}})
    assert fin == "FAILED" and msgs == ["error : 90296 : sandbox"], msgs

    # Le numéro : déjà pris ou en recul, refusé ; neuf et croissant, accepté.
    numero_libre(["198", "228"], "229")
    numero_libre([], "1")
    refuse(lambda: numero_libre(["198", "228"], "228"), "existe déjà")
    refuse(lambda: numero_libre(["198", "228"], "200"), "plus petit")

    # L'app du .pkg.
    info = {"CFBundleIdentifier": "fr.morain.colophon", "CFBundleShortVersionString": "1.2.0", "CFBundleVersion": "229"}
    sig = f"Status: signed by a certificate trusted by macOS\n   1. {INSTALLEUR}: X (ABC)\n"
    assert verifier_app(info, sig, True, True) == ("fr.morain.colophon", "1.2.0", "229")
    refuse(lambda: verifier_app(info, "Status: no signature", True, True), INSTALLEUR)
    refuse(lambda: verifier_app(info, sig, False, True), "profil")
    refuse(lambda: verifier_app(info, sig, True, False), "app-sandbox")
    refuse(lambda: verifier_app(dict(info, CFBundleVersion="1.2"), sig, True, True), "entier")
    refuse(lambda: verifier_app({}, sig, True, True), "Info.plist")
    print("asc-envoi : tests ok")


def main(argv):
    sys.stdout.reconfigure(line_buffering=True)
    if argv[1:] == ["--test"]:
        tests()
        return 0
    if len(argv) != 3 or argv[1] not in ("valider", "envoyer"):
        print("usage : asc-envoi.py valider|envoyer <pkg>  |  --test", file=sys.stderr)
        return 2
    action, pkg = argv[1], argv[2]
    try:
        if not os.path.isfile(pkg):
            raise Refus(f"pas de .pkg à {pkg}")
        client = client_depuis_env()
        if action == "valider":
            valider(pkg, client)
            return 0
        return envoyer(pkg, client)
    except Refus as e:
        print(f"asc-envoi : {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
