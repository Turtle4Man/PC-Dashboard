# PC Dashboard

Tableau de bord Windows compact, construit avec Tauri 2 et React. Il regroupe
sur une seule fenêtre les informations et les actions utilisées au quotidien,
sans passer par plusieurs applications.

## Fonctionnalités

- **Statistiques** : processeur, GPU, mémoire, latence réseau et temps de
  réponse des disques, rafraîchis chaque seconde.
- **Lecteur multimédia** : titre, artiste, pochette et commandes
  lecture/pause, piste suivante et précédente, avec ouverture de la source.
- **Boutons d'action** : micro Windows (synchronisé avec Discord quand il est
  lancé), verrouillage du clavier avec durée réglable ou illimitée, capture
  d'écran, réglages Bluetooth, raccourcis web et clavier. Les icônes peuvent
  être remplacées par une image locale.
- **Appareils** : niveau de batterie des périphériques Bluetooth, GATT,
  récepteurs Logitech Lightspeed et écouteurs Apple.
- **Réglages** : thème, format de l'heure, portée des captures, durée du
  verrouillage, lancement au démarrage de Windows et masquage des blocs de
  l'accueil.
- **Configuration matérielle** : processeur, carte graphique, mémoire, carte
  mère, stockage, système et périphériques détectés.

L'interface est en français et suit le thème clair ou sombre du système.

## Prérequis

- Windows 10 ou 11 avec WebView2 (installé par défaut sur Windows 11)
- [Node.js](https://nodejs.org/) 20 ou plus récent
- [Rust](https://www.rust-lang.org/tools/install) (édition 2024)

## Développement

```sh
npm install
npm run tauri dev
```

## Construction

```sh
npm run tauri build
```

Le binaire se trouve dans `src-tauri/target/release/pc-dashboard.exe`, avec les
installateurs MSI et NSIS dans `src-tauri/target/release/bundle/`.

## Tests

Les tests couvrent le comportement visible de l'application et les contrats
entre l'interface et le backend. Aucun test ne demande l'application lancée :
les commandes Tauri, les événements et `localStorage` sont simulés.

```sh
npm test           # interface (Vitest + Testing Library)
npm run test:watch # interface, en continu
```

```sh
cd src-tauri
cargo test --lib   # backend Windows
```

- `src/**/*.test.js(x)` : réglages et normalisation, boutons d'action (micro,
  verrou clavier, capture d'écran, raccourcis), tuile média, appareils,
  statistiques, horloge, IPC et stores.
- `src-tauri/src/**` : analyse des raccourcis, entrée de démarrage automatique,
  formatage matériel, contrat JSON des événements, batteries, captures d'écran.

## Structure

```text
src/                    interface React (écrans, composants, stores, IPC)
src-tauri/src/          backend Rust (actions système, capture, Discord)
src-tauri/src/actions/  micro, clavier, capture, raccourcis, médias
src-tauri/src/capture/  échantillonnage des statistiques et des batteries
```

## Licence

Distribué sous licence MIT. Voir le fichier [LICENSE](LICENSE).
