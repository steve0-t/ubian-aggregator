export type Language = "sk" | "en" | "uk";

const sk = {
  unofficial:
    "Neoficiálny klient Ubian. Bez prepojenia na prevádzkovateľa Ubian.",
  journeys: "Cesty",
  timetables: "Cestovné poriadky",
  signIn: "Prihlásenie",
  account: "Účet",
  settings: "Nastavenia",
  journeyDescription:
    "Vyhľadávanie spojení a odchodov priamo v aplikácii sa pripravuje.",
  openJourneys: "Otvoriť cestovné poriadky",
  returnHint:
    "Tlačidlo Zavrieť vás vráti do aplikácie. Späť prechádza históriou webu.",
  accountTitle: "Účet Ubian",
  accountDescription:
    "Prihlásenie prebieha na stránke Ubian. Aplikácia zatiaľ nevie overiť stav prihlásenia.",
  openLogin: "Otvoriť prihlásenie Ubian",
  credentials:
    "Ukladanie a opätovné použitie hesla ešte nie sú dostupné. Heslo zadávajte iba na overenej stránke Ubian.",
  session:
    "Zapamätanie relácie je vypnuté. Aplikácia žiada vyčistenie webových údajov pred otvorením a po zatvorení. Úplné vymazanie ešte nie je overené. WebView môže zapisovať na disk.",
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
  guestShort: "Bez povinného účtu",
  privacyDetails: "Viac o súkromí",
  passwordTitle: "Prihlasovacie údaje",
  sessionTitle: "Relácia prehliadania",
  socialTitle: "Ďalšie spôsoby prihlásenia",
  appearance: "Vzhľad",
  theme: "Motív aplikácie",
  systemTheme: "Podľa systému",
  lightTheme: "Svetlý",
  darkTheme: "Tmavý",
  blackTheme: "Úplne čierny / AMOLED",
  accentColor: "Farba zvýraznenia",
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
  localStatus: "Dostupné v Android aplikácii",
  timetableIncomplete: "Cestovné poriadky v aplikácii zatiaľ nie sú dostupné",
  timetableBlocked:
    "Prístup k zdroju cestovných poriadkov pre túto aplikáciu ešte nie je potvrdený. Vyhľadávanie spojení, odchody a detaily sa zatiaľ nedajú použiť.",
  noJourneyHistory: "Vyhľadávania sa neukladajú. Účet nie je potrebný.",
  stopQuery: "Názov zastávky",
  lookupStop: "Vyhľadať zastávku",
  queryInvalid: "Zadajte 2 až 100 znakov bez riadiacich znakov.",
  cancel: "Zrušiť",
  cancelled: "Vyhľadávanie bolo zrušené.",
  offline: "Nie ste pripojení. Skontrolujte pripojenie a skúste to znova.",
  timeout: "Vyhľadávanie trvalo príliš dlho. Skúste to znova neskôr.",
  rateLimited: "Príliš veľa požiadaviek. Počkajte a skúste to znova.",
  providerChanged:
    "Formát údajov poskytovateľa sa zmenil. Vyhľadávanie je nedostupné.",
  noStops: "Nenašli sa žiadne zastávky.",
  searching: "Vyhľadávajú sa zastávky…",
  providerLanguage:
    "Jazyk webu Ubian sa riadi webom poskytovateľa. Pre ukrajinčinu používame slovenskú stránku; miestne ovládanie zostáva preložené.",
  rememberSession: "Zapamätať reláciu",
  saveCredentials: "Uložiť prihlasovacie údaje",
  storagePreparation:
    "Pripravuje sa úložisko chránené overením zariadenia. Používa iba syntetické údaje. Skutočné prihlasovacie údaje sa nedajú uložiť ani opätovne použiť.",
  unavailableOff: "Vypnuté · zatiaľ nedostupné",
  systemAppearanceError:
    "Vzhľad aplikácie je nastavený; farby systémových panelov Android sa nepodarilo zmeniť.",
};

type Messages = { [K in keyof typeof sk]: string };

const en: Messages = {
  unofficial: "An unofficial Ubian client. Not affiliated with Ubian.",
  journeys: "Journeys",
  timetables: "Timetables",
  signIn: "Sign in",
  account: "Account",
  settings: "Settings",
  journeyDescription:
    "In-app connection search and departure boards are in preparation.",
  openJourneys: "Open timetables",
  returnHint:
    "Close returns to the app. Back moves through the website history.",
  accountTitle: "Ubian account",
  accountDescription:
    "Sign in on the Ubian website. The app cannot verify your login status yet.",
  openLogin: "Open Ubian sign-in",
  credentials:
    "Saving and reusing passwords is not available yet. Enter your password only on the verified Ubian page.",
  session:
    "Remember session is off. The app requests website cleanup before opening and after closing. Complete deletion is not verified yet. WebView may write to disk.",
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
  guestShort: "No account needed",
  privacyDetails: "More about privacy",
  passwordTitle: "Sign-in details",
  sessionTitle: "Browsing session",
  socialTitle: "Other ways to sign in",
  appearance: "Appearance",
  theme: "App theme",
  systemTheme: "System",
  lightTheme: "Light",
  darkTheme: "Dark",
  blackTheme: "True black / AMOLED",
  accentColor: "Accent color",
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
  localStatus: "Available in the Android app",
  timetableIncomplete: "In-app timetables are not available yet",
  timetableBlocked:
    "Access to the timetable source for this app has not been confirmed. Connection search, departures and details are not available yet.",
  noJourneyHistory: "Searches are not saved. No account is needed.",
  stopQuery: "Stop name",
  lookupStop: "Find a stop",
  queryInvalid: "Enter 2–100 characters without control characters.",
  cancel: "Cancel",
  cancelled: "Search cancelled.",
  offline: "You are offline. Check your connection and try again.",
  timeout: "The search timed out. Try again later.",
  rateLimited: "Too many requests. Please wait and try again.",
  providerChanged:
    "The provider data format has changed. Search is unavailable.",
  noStops: "No stops found.",
  searching: "Finding stops…",
  providerLanguage:
    "The Ubian website controls its own language. Ukrainian uses the Slovak page; local controls stay translated.",
  rememberSession: "Remember session",
  saveCredentials: "Save login credentials",
  storagePreparation:
    "Storage protected by device authentication is in preparation, using synthetic data only. Saving and reusing real login credentials is unavailable.",
  unavailableOff: "Off · not available yet",
  systemAppearanceError:
    "The app appearance is set; Android system bar colors could not be updated.",
};

const uk: Messages = {
  unofficial: "Неофіційний клієнт Ubian. Не пов’язаний з оператором Ubian.",
  journeys: "Поїздки",
  timetables: "Розклади руху",
  signIn: "Вхід",
  account: "Обліковий запис",
  settings: "Налаштування",
  journeyDescription:
    "Пошук сполучень і розклади відправлень у застосунку готуються.",
  openJourneys: "Відкрити розклади руху",
  returnHint:
    "Кнопка «Закрити» повертає до застосунку. Кнопка «Назад» переглядає історію сайту.",
  accountTitle: "Обліковий запис Ubian",
  accountDescription:
    "Вхід відбувається на сайті Ubian. Застосунок поки що не може перевірити стан входу.",
  openLogin: "Відкрити вхід до Ubian",
  credentials:
    "Збереження та повторне використання пароля ще недоступні. Вводьте пароль лише на перевіреній сторінці Ubian.",
  session:
    "Запам’ятовування сеансу вимкнено. Застосунок запитує очищення даних сайту перед відкриттям і після закриття. Повне видалення ще не перевірено. WebView може записувати дані на диск.",
  social:
    "Вхід через соціальні мережі й оплата потребують додаткової перевірки. Зовнішній браузер може не використовувати сеанс застосунку.",
  privacy: "Приватність",
  privacyDescription:
    "Сторонні файли cookie вимкнено. Застосунок не додає аналітику чи рекламу.",
  rulesPending:
    "Правила блокування необов’язкового стеження очікують перевірки мережевих запитів. Правил блокування аналітики поки що немає.",
  limits:
    "Ubian і необхідні постачальники отримують ваші запити та IP-адресу. Захист не поширюється на зовнішній браузер.",
  language: "Мова застосунку",
  status: "Стан застосунку",
  spike: "Версія для розробки · інтеграція з Android очікує перевірки",
  browserPreview:
    "Попередній перегляд локального інтерфейсу. Перегляд сайту й перевірка Keystore потребують застосунку Android.",
  nativeUnavailable: "Ця функція потребує застосунку Android.",
  genericError:
    "Не вдалося виконати дію. Перевірте з’єднання або стан застосунку.",
  busy: "Триває обробка…",
  opened: "Сайт Ubian відкрито.",
  refresh: "Оновити стан",
  diagnostic: "Перевірка Android Keystore",
  diagnosticDescription:
    "Використовує лише тестові дані, перевіряє шифрування, а потім видаляє тестовий ключ і файл. Не перевіряє біометрію чи збережений вхід.",
  runProbe: "Перевірити шифрування",
  probePassed:
    "Шифрування, відхилення змінених даних, унікальні nonce та відхилення відсутнього ключа перевірено успішно.",
  probeFailed:
    "Перевірка шифрування не вдалася. Збереження облікових даних недоступне.",
  hardware: "Ключ захищено апаратно",
  yes: "Так",
  no: "Ні",
  help: "Допомога Ubian",
  verifiedRules: "Перевірені правила блокування",
  state: "Стан перегляду сайту",
  cleanupFailed:
    "Не вдалося очистити дані сайту. Відкриття заблоковано; закрийте та знову запустіть застосунок.",
  closed: "Закрито",
  opening: "Відкривається",
  open: "Відкрито",
  cleaning: "Очищення",
  mainNavigation: "Головна навігація",
  skipContent: "Перейти до вмісту",
  navigationHeading: "Подорожі",
  browserLabel: "Перегляд у браузері",
  androidLabel: "Застосунок Android",
  developmentLabel: "Попередня версія",
  personalize: "Налаштувати вигляд",
  guestShort: "Без обов’язкового облікового запису",
  privacyDetails: "Докладніше про приватність",
  passwordTitle: "Дані для входу",
  sessionTitle: "Сеанс перегляду",
  socialTitle: "Інші способи входу",
  appearance: "Вигляд",
  theme: "Тема застосунку",
  systemTheme: "Системна",
  lightTheme: "Світла",
  darkTheme: "Темна",
  blackTheme: "Суто чорна / AMOLED",
  accentColor: "Колір акценту",
  customColor: "Власний колір",
  hexColor: "HEX-код кольору",
  invalidColor: "Введіть колір у форматі #RRGGBB, наприклад #008837.",
  resetAppearance: "Відновити стандартний вигляд",
  preferencesSaved: "Зміни автоматично зберігаються на цьому пристрої.",
  preferencesUnsaved:
    "Вигляд змінено, але налаштування не вдалося зберегти на пристрої.",
  green: "Зелений",
  teal: "Бірюзовий",
  blue: "Синій",
  purple: "Фіолетовий",
  rose: "Рожевий",
  orange: "Помаранчевий",
  localStatus: "Доступно в застосунку Android",
  timetableIncomplete: "Розклади руху в застосунку ще недоступні",
  timetableBlocked:
    "Доступ цього застосунку до джерела розкладів ще не підтверджено. Пошук сполучень, відправлення та докладні відомості поки що недоступні.",
  noJourneyHistory:
    "Пошукові запити не зберігаються. Обліковий запис не потрібен.",
  stopQuery: "Назва зупинки",
  lookupStop: "Знайти зупинку",
  queryInvalid: "Введіть від 2 до 100 символів без керівних символів.",
  cancel: "Скасувати",
  cancelled: "Пошук скасовано.",
  offline:
    "Немає з’єднання з мережею. Перевірте підключення та спробуйте ще раз.",
  timeout: "Час очікування пошуку минув. Спробуйте пізніше.",
  rateLimited: "Забагато запитів. Зачекайте та спробуйте ще раз.",
  providerChanged: "Формат даних постачальника змінився. Пошук недоступний.",
  noStops: "Зупинок не знайдено.",
  searching: "Пошук зупинок…",
  providerLanguage:
    "Мову сайту Ubian визначає постачальник. Для української використовується словацька сторінка; локальні елементи керування залишаються перекладеними.",
  rememberSession: "Запам’ятати сеанс",
  saveCredentials: "Зберегти дані для входу",
  storagePreparation:
    "Готується сховище із захистом через автентифікацію пристрою, лише на синтетичних даних. Збереження та повторне використання справжніх даних для входу недоступні.",
  unavailableOff: "Вимкнено · ще недоступно",
  systemAppearanceError:
    "Вигляд застосунку налаштовано; не вдалося оновити кольори системних панелей Android.",
};

export const messages: Record<Language, Messages> = { sk, en, uk };

export const locales: Record<Language, string> = {
  sk: "sk-SK",
  en: "en-GB",
  uk: "uk-UA",
};
export const TIMETABLE_TIME_ZONE = "Europe/Bratislava";

export function formatTimetableTime(instant: Date, language: Language): string {
  return new Intl.DateTimeFormat(locales[language], {
    timeZone: TIMETABLE_TIME_ZONE,
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
  }).format(instant);
}

export function formatTimetableDate(instant: Date, language: Language): string {
  return new Intl.DateTimeFormat(locales[language], {
    timeZone: TIMETABLE_TIME_ZONE,
    day: "numeric",
    month: "long",
    year: "numeric",
  }).format(instant);
}
