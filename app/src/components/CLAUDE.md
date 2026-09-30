# app/src/components — UI-примітиви, що не знають домену

- Можна: `leptos`, `wasm-bindgen`, `web-sys`. Не можна: `types`, `domain`, `services`, `backend` —
  якщо компонент бере доменний тип у пропси, він домену "не знає" лише формально; тримай пропси
  максимально плоскими (рядки/числа/callback'и), щоб компонент лишався переюзним де завгодно.
- Перший реальний споживач цієї теки (07-code-structure.md описував її наперед) — `Select`, бо
  нативний `<select>`-список не можна стилізувати CSS (завжди системна панель поза темою застосунку).
- Класи — атоми `style`-крейту (skill `styling`), не інлайн-стилі.

| Елемент | Що це | Хто використовує |
|---|---|---|
| `accordion.rs` | `Accordion`/`AccordionItem` — розгортання розміткою, CSS grid-rows-анімація | — |
| `checkbox.rs` | `Checkbox` — стилізований чекбокс (справжній `<input>`, приховано лише візуально) | — |
| `combobox.rs` | `Combobox`/`ComboboxItem`/`ComboboxVariant`/`ComboboxMode` — переробка `Select` з нуля (`docs/spec/components/select.md`, `docs/spec/components/grid.md` §2): портал (`leptos::portal::Portal`) + `hooks::use_popover_position`, варіанти `Field`/`InCell`, режими `Trigger` (кнопка, пошук усередині панелі, компонент сам фільтрує) / `Input` (сам `<input>` — тригер, список фільтрується ЗЗОВНІ через `on_query_change`, домен-agnostic — заміна `OrgAutocomplete`), двострічкові пункти (назва+пояснення), групи | `/styleguide`, `widgets::group_grid::autocomplete` (`mode=Input`) |
| `date_picker.rs` | `DatePicker` — текстове поле з маскою "тільки цифри" (`ддммрррр`→`дд.мм.рррр` на льоту) + календар-панель, `chrono` напряму (не `domain::dates` — не знає домену); `hooks::use_floating_position` (feedback користувача — панель вилазила за екран) | `pages::import`, `pages::training_form`, `pages::home`, `pages::documents` |
| `download.rs` | `download_bytes` — браузерне скачування готових байтів без URL-ендпоінта (`Blob`+`<a download>`) | `pages::documents` |
| `drawer.rs` | `Drawer` — бічна панель на всю висоту (той самий контракт, що `Modal`, `hooks::use_escape_close`), для "розгорнути рядок у форму" | `widgets::group_grid` (RowEditor) |
| `file_dropzone.rs` | `FileDropzone` — перетягнути файл або клікнути (прихований `<input type="file">` — одне джерело `web_sys::File` для обох шляхів); `read_file_bytes` — спільне читання файлу в байти (`File::array_buffer()`) | `pages::training_form`, `pages::import` |
| `modal.rs` | `Modal`/`Dialog` — оверлей+панель, Escape закриває, клік-поза НЕ закриває (навмисно, `.modal__*` — той самий клас, що `training_form::CheatSheet`) | — |
| `select.rs` | `Select`/`SelectOption` — стилізована випадайка (клавіатура: стрілки/Enter/Escape/друк-до-літери, клік поза — закриває, `hooks::use_floating_position` — перевертається/вирівнюється, коли впирається в край в'юпорту), заміна нативного `<select>`; опційні `id`/`on_keydown` — для вбудовування в `Grid` (стрілки/Enter лишаються за сіткою, відкриття — `Alt+↓`); усередині `.grid__row` тригер заповнює свою колонку (`main.css`), не диктує власний `min-width` | `layout::ActorSwitcher`, `pages::training_form`/`pages::import` (FileKind), `widgets::group_grid::Grid` (TrainingKindCell/SiteCell) |
