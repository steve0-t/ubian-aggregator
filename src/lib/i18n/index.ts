export type Language = "sk" | "en";

const sk = {
  tagline: "Vaša cesta, na jednom mieste.",
  unofficial:
    "Neoficiálny klient Ubian. Bez prepojenia na prevádzkovateľa Ubian.",
  journeys: "Cesty",
  account: "Účet",
  settings: "Nastavenia",
  welcome: "Kam sa vydáte?",
  journeyDescription:
    "Vyhľadajte spojenie, odchody alebo zastávku na webe Ubian.",
  openJourneys: "Otvoriť cestovné poriadky",
  providerLabel: "Web Ubian · www.ubian.sk",
  returnHint:
    "Tlačidlo Zavrieť vás vráti do Cestou. Späť prechádza históriou webu.",
  guest: "Verejné cestovné poriadky môžete používať bez účtu.",
  accountTitle: "Váš účet Ubian",
  accountDescription:
    "Prihlásenie prebieha na stránke Ubian. Cestou zatiaľ nevie overiť stav prihlásenia.",
  openLogin: "Otvoriť prihlásenie Ubian",
  credentials:
    "Ukladanie a opätovné použitie hesla ešte nie sú dostupné. Heslo zadávajte iba na overenej stránke Ubian.",
  session:
    "Zapamätanie relácie je vypnuté. Cestou žiada vyčistenie webových údajov pred otvorením a po zatvorení. Úplné vymazanie ešte nie je overené. WebView môže zapisovať na disk.",
  social:
    "Sociálne prihlásenie a platby vyžadujú ďalšie overenie. Externý prehliadač nemusí zdieľať reláciu aplikácie.",
  privacy: "Súkromie",
  privacyDescription:
    "Cookies tretích strán sú vypnuté. Aplikácia nepridáva analytiku ani reklamy.",
  rulesPending:
    "Blokovanie voliteľného sledovania čaká na overenie sieťovej prevádzky. Zatiaľ nie sú aktívne žiadne pravidlá pre analytiku.",
  limits:
    "Ubian a potrební poskytovatelia prijímajú vaše požiadavky a IP adresu. Ochrana nepokrýva externý prehliadač.",
  language: "Jazyk aplikácie",
  status: "Stav aplikácie",
  spike: "Vývojová verzia · Android integrácia čaká na overenie",
  browserPreview:
    "Náhľad miestneho rozhrania. Otváranie webu a Keystore test vyžadujú Android aplikáciu.",
  nativeUnavailable: "Táto funkcia vyžaduje Android aplikáciu.",
  genericError:
    "Operácia sa nepodarila. Skontrolujte pripojenie alebo stav aplikácie.",
  busy: "Pracuje sa…",
  opened: "Web Ubian bol otvorený.",
  refresh: "Obnoviť stav",
  diagnostic: "Test Android Keystore",
  diagnosticDescription:
    "Použije iba skúšobné údaje, overí šifrovanie a odstráni skúšobný kľúč aj súbor. Neoveruje biometriku ani uložené prihlásenie.",
  runProbe: "Spustiť test šifrovania",
  probePassed:
    "Šifrovanie, odmietnutie zmenených údajov, rôzne nonce a odmietnutie chýbajúceho kľúča: úspešné.",
  probeFailed:
    "Test šifrovania zlyhal. Ukladanie prihlasovacích údajov nie je povolené.",
  hardware: "Kľúč chránený hardvérom",
  yes: "Áno",
  no: "Nie",
  help: "Pomoc Ubian",
  verifiedRules: "Overené pravidlá blokovania",
  state: "Stav webového zobrazenia",
  cleanupFailed:
    "Čistenie webových údajov zlyhalo. Ďalšie otvorenie je zablokované; ukončite a znovu spustite aplikáciu.",
  closed: "Zatvorené",
  opening: "Otváranie",
  open: "Otvorené",
  cleaning: "Čistenie",
  mainNavigation: "Hlavná navigácia",
  skipContent: "Prejsť na obsah",
  navigationHeading: "Cestovanie",
  browserLabel: "Náhľad v prehliadači",
  androidLabel: "Android aplikácia",
  developmentLabel: "Vývojový náhľad",
  personalize: "Prispôsobiť vzhľad",
  introEyebrow: "Cestovanie, jednoducho",
  journeySubtitle: "Každá dobrá cesta sa začína jednoduchým plánom.",
  journeyPanelLabel: "Vaša ďalšia cesta",
  journeyHeading: "Všetky spojenia.",
  journeyHeadingAccent: "Na jednom mieste.",
  guestShort: "Bez povinného účtu",
  accountCardDescription: "Prihlásenie a služby vášho účtu na webe Ubian.",
  appearanceCardTitle: "Trochu viac podľa vás",
  appearanceCardDescription:
    "Obľúbená farba. Svetlý či tmavý vzhľad. Vaša voľba.",
  privacyDetails: "Viac o súkromí",
  accountEyebrow: "Váš priestor",
  accountPanelTitle: "Všetko vybavíte na Ubian",
  accountPanelDescription:
    "Otvorte stránku Ubian a prihláste sa do svojho účtu.",
  passwordTitle: "Prihlasovacie údaje",
  sessionTitle: "Relácia prehliadania",
  socialTitle: "Ďalšie spôsoby prihlásenia",
  settingsDescription: "Malé detaily, vďaka ktorým je Cestou viac vaše.",
  appearance: "Vzhľad",
  appearanceDescription: "Vytvorte si prostredie, v ktorom sa cítite dobre.",
  theme: "Motív aplikácie",
  systemTheme: "Podľa systému",
  lightTheme: "Svetlý",
  darkTheme: "Tmavý",
  accentColor: "Farba zvýraznenia",
  accentDescription: "Jedna farba pre tlačidlá, navigáciu a malé detaily.",
  customColor: "Vlastná farba",
  hexColor: "Kód farby HEX",
  invalidColor: "Zadajte farbu vo formáte #RRGGBB, napríklad #008837.",
  resetAppearance: "Obnoviť predvolený vzhľad",
  preferencesSaved: "Zmeny sa automaticky ukladajú do tohto zariadenia.",
  preferencesUnsaved:
    "Vzhľad sa zmenil, ale nastavenia sa nepodarilo uložiť do zariadenia.",
  green: "Zelená",
  teal: "Tyrkysová",
  blue: "Modrá",
  purple: "Fialová",
  rose: "Ružová",
  orange: "Oranžová",
  languageDescription: "Vyberte si jazyk miestneho rozhrania.",
  previewTitle: "Vaša cesta. Vaša farba.",
  previewDescription: "Aj každodenné cestovanie môže mať váš štýl.",
  backToJourneys: "Späť na cesty",
  localStatus: "Dostupné v Android aplikácii",
};

type Messages = { [K in keyof typeof sk]: string };

const en: Messages = {
  tagline: "Your journey, in one place.",
  unofficial: "An unofficial Ubian client. Not affiliated with Ubian.",
  journeys: "Journeys",
  account: "Account",
  settings: "Settings",
  welcome: "Where are you headed?",
  journeyDescription:
    "Find a connection, departures or a stop on the Ubian website.",
  openJourneys: "Open timetables",
  providerLabel: "Ubian website · www.ubian.sk",
  returnHint:
    "Close returns to Cestou. Back moves through the website history.",
  guest: "Public timetables are available without an account.",
  accountTitle: "Your Ubian account",
  accountDescription:
    "Sign in on the Ubian website. Cestou cannot verify your login status yet.",
  openLogin: "Open Ubian sign-in",
  credentials:
    "Saving and reusing passwords is not available yet. Enter your password only on the verified Ubian page.",
  session:
    "Remember session is off. Cestou requests website cleanup before opening and after closing. Complete deletion is not verified yet. WebView may write to disk.",
  social:
    "Social sign-in and payments require further verification. The external browser may not share the app session.",
  privacy: "Privacy",
  privacyDescription:
    "Third-party cookies are disabled. The app adds no analytics or advertisements.",
  rulesPending:
    "Optional tracking rules await network verification. No analytics blocking rules are active yet.",
  limits:
    "Ubian and necessary providers receive your requests and IP address. Protection does not cover external browsers.",
  language: "App language",
  status: "App status",
  spike: "Development build · Android integration awaits verification",
  browserPreview:
    "Local interface preview. Website browsing and the Keystore test require the Android app.",
  nativeUnavailable: "This feature requires the Android app.",
  genericError:
    "The operation failed. Check your connection or the app status.",
  busy: "Working…",
  opened: "The Ubian website was opened.",
  refresh: "Refresh status",
  diagnostic: "Android Keystore test",
  diagnosticDescription:
    "Uses test data only, checks encryption, then deletes the test key and file. Does not verify biometrics or saved sign-in.",
  runProbe: "Run encryption test",
  probePassed:
    "Encryption, tamper rejection, unique nonces and missing-key rejection passed.",
  probeFailed: "The encryption test failed. Credential storage is unavailable.",
  hardware: "Hardware-backed key",
  yes: "Yes",
  no: "No",
  help: "Ubian help",
  verifiedRules: "Verified blocking rules",
  state: "Website view state",
  cleanupFailed:
    "Website cleanup failed. Opening is blocked; close and restart the app.",
  closed: "Closed",
  opening: "Opening",
  open: "Open",
  cleaning: "Cleaning",
  mainNavigation: "Main navigation",
  skipContent: "Skip to content",
  navigationHeading: "Travel",
  browserLabel: "Browser preview",
  androidLabel: "Android app",
  developmentLabel: "Development preview",
  personalize: "Customize appearance",
  introEyebrow: "Travel, made simple",
  journeySubtitle: "Every good journey starts with a simple plan.",
  journeyPanelLabel: "Your next journey",
  journeyHeading: "Every connection.",
  journeyHeadingAccent: "All in one place.",
  guestShort: "No account needed",
  accountCardDescription:
    "Sign in and access your account on the Ubian website.",
  appearanceCardTitle: "A little more you",
  appearanceCardDescription:
    "Your favorite color. Light or dark. Make it yours.",
  privacyDetails: "More about privacy",
  accountEyebrow: "Your space",
  accountPanelTitle: "Take care of it on Ubian",
  accountPanelDescription:
    "Open the Ubian website and sign in to your account.",
  passwordTitle: "Sign-in details",
  sessionTitle: "Browsing session",
  socialTitle: "Other ways to sign in",
  settingsDescription: "The little details that make Cestou feel like yours.",
  appearance: "Appearance",
  appearanceDescription: "Make yourself at home.",
  theme: "App theme",
  systemTheme: "System",
  lightTheme: "Light",
  darkTheme: "Dark",
  accentColor: "Accent color",
  accentDescription:
    "One color for buttons, navigation, and the little details.",
  customColor: "Custom color",
  hexColor: "HEX color code",
  invalidColor: "Use the #RRGGBB format, for example #008837.",
  resetAppearance: "Reset appearance",
  preferencesSaved: "Changes are automatically saved on this device.",
  preferencesUnsaved:
    "Appearance changed, but your settings could not be saved on this device.",
  green: "Green",
  teal: "Teal",
  blue: "Blue",
  purple: "Purple",
  rose: "Rose",
  orange: "Orange",
  languageDescription: "Choose the language for the local interface.",
  previewTitle: "Your journey. Your color.",
  previewDescription: "Bring a little of your own style to the everyday.",
  backToJourneys: "Back to journeys",
  localStatus: "Available in the Android app",
};

export const messages: Record<Language, Messages> = { sk, en };
