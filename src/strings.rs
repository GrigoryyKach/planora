//! All interface text, in English, Russian and Ukrainian. One row per string: `key: en, ru, uk;`.

use crate::i18n::Table;

macro_rules! strings {
    (
        text { $( $name:ident : $en:expr, $ru:expr, $uk:expr; )* }
        plural { $( $pname:ident : $pen:expr, $pru:expr, $puk:expr; )* }
    ) => {
        pub struct S {
            $( pub $name: &'static str, )*
            $( pub $pname: [&'static str; 3], )*
        }
        const EN: S = S { $( $name: $en, )* $( $pname: $pen, )* };
        const RU: S = S { $( $name: $ru, )* $( $pname: $pru, )* };
        const UK: S = S { $( $name: $uk, )* $( $pname: $puk, )* };
        pub static TABLE: Table<S> = Table { en: &EN, ru: &RU, uk: &UK };

        impl S {
            /// Every string as `(key, [en, ru, uk])`; plural strings contribute their three forms. For the tests.
            #[cfg(test)]
            pub fn catalog() -> Vec<(&'static str, [&'static str; 3])> {
                let mut v: Vec<(&'static str, [&'static str; 3])> = Vec::new();
                $( v.push((stringify!($name), [$en, $ru, $uk])); )*
                $( for i in 0..3 { v.push((stringify!($pname), [$pen[i], $pru[i], $puk[i]])); } )*
                v
            }
        }
    };
}

strings! {
    text {
        // Navigation and general
        today: "Today", "Сегодня", "Сьогодні";
        calendar: "Calendar", "Календарь", "Календар";
        settings: "Settings", "Настройки", "Налаштування";
        inbox: "Inbox", "Входящие", "Вхідні";
        personal: "Personal", "Личное", "Особисте";
        work: "Work", "Работа", "Робота";
        all: "All", "Все", "Усі";
        new_task: "New task", "Новая задача", "Нове завдання";
        search: "Search", "Поиск", "Пошук";
        add: "Add", "Добавить", "Додати";
        cancel: "Cancel", "Отмена", "Скасувати";
        save: "Save", "Сохранить", "Зберегти";
        delete: "Delete", "Удалить", "Видалити";
        close: "Close", "Закрыть", "Закрити";
        ok: "OK", "Понятно", "Зрозуміло";
        clear: "Clear", "Очистить", "Очистити";
        open: "Open", "Открыть", "Відкрити";
        copy: "Copy", "Копировать", "Копіювати";
        undo: "Undo", "Отменить", "Скасувати";
        more: "+{n} more", "ещё {n}", "ще {n}";
        hide_done: "Hide done", "Скрыть завершённые", "Сховати завершені";
        today_btn: "Today", "Сегодня", "Сьогодні";
        prev_month: "Previous month", "Предыдущий месяц", "Попередній місяць";
        next_month: "Next month", "Следующий месяц", "Наступний місяць";
        tomorrow: "Tomorrow", "Завтра", "Завтра";
        yesterday: "Yesterday", "Вчера", "Вчора";
        next_week: "Next week", "На следующей неделе", "Наступного тижня";
        pick_date: "Pick a date", "Выбрать дату", "Обрати дату";
        set_date: "Set date", "Задать дату", "Задати дату";
        date_label: "Date", "Дата", "Дата";
        coming_up: "Coming up", "Ближайшие дни", "Найближчі дні";
        overdue: "Overdue", "Просрочено", "Прострочено";
        nothing_today: "Nothing planned for today", "На сегодня ничего не запланировано", "На сьогодні нічого не заплановано";
        nothing_today_hint: "Press Ctrl+N to add a task", "Нажмите Ctrl+N, чтобы добавить задачу", "Натисніть Ctrl+N, щоб додати завдання";
        nothing_on_day: "Nothing on this day yet", "На этот день пока ничего нет", "На цей день поки нічого немає";
        add_on_day: "Add task on this day", "Добавить задачу на этот день", "Додати завдання на цей день";
        added_to: "Task added to {where}", "Задача добавлена: {where}", "Завдання додано: {where}";
        task_trashed: "Task moved to Trash", "Задача перемещена в Корзину", "Завдання переміщено до Кошика";
        copy_suffix: "(copy)", "(копия)", "(копія)";
        save_failed: "Could not save your data. Check the disk and try again.", "Не удалось сохранить данные. Проверьте диск и повторите.", "Не вдалося зберегти дані. Перевірте диск і повторіть.";

        collapse_sidebar: "Collapse sidebar", "Свернуть панель", "Згорнути панель";
        expand_sidebar: "Expand sidebar", "Развернуть панель", "Розгорнути панель";

        // Lists
        no_tasks: "No tasks here", "Здесь пока нет задач", "Тут поки немає завдань";
        no_tasks_hint: "Type a title above and press Enter", "Введите название выше и нажмите Enter", "Введіть назву вище й натисніть Enter";
        add_task_hint: "Add a task", "Добавить задачу", "Додати завдання";
        filter_hint: "Search…", "Поиск…", "Пошук…";
        clear_filters: "Clear filters", "Сбросить фильтры", "Скинути фільтри";
        sort_by: "Sort", "Порядок", "Порядок";
        sort_manual: "Manual", "Вручную", "Вручну";
        sort_deadline: "Deadline", "По дедлайну", "За дедлайном";
        sort_priority: "Priority", "По приоритету", "За пріоритетом";
        sort_created: "Newest first", "Сначала новые", "Спочатку нові";
        col_title: "Task", "Задача", "Завдання";
        col_priority: "Priority", "Приоритет", "Пріоритет";
        col_deadline: "Deadline", "Дедлайн", "Дедлайн";
        col_scheduled: "Planned", "План", "План";
        click_status: "Click to change the status", "Нажмите, чтобы сменить статус", "Натисніть, щоб змінити статус";
        status_label: "Status", "Статус", "Статус";
        status_none: "No status", "Без статуса", "Без статусу";
        status_todo: "To do", "К выполнению", "До виконання";
        status_progress: "In progress", "В работе", "У роботі";
        status_done: "Done", "Готово", "Готово";
        status_cancelled: "Cancelled", "Отменена", "Скасовано";
        priority_label: "Priority", "Приоритет", "Пріоритет";
        priority_none: "No priority", "Без приоритета", "Без пріоритету";
        prio_low: "Low", "Низкий", "Низький";
        prio_medium: "Medium", "Средний", "Середній";
        prio_high: "High", "Высокий", "Високий";
        prio_urgent: "Urgent", "Срочный", "Терміновий";
        deadline_label: "Deadline", "Дедлайн", "Дедлайн";
        dl_overdue: "Overdue", "Просрочен", "Прострочений";
        dl_today: "Today", "Сегодня", "Сьогодні";
        dl_week: "This week", "На неделе", "Цього тижня";
        dl_none: "No deadline", "Без дедлайна", "Без дедлайну";
        menu_status: "Status", "Статус", "Статус";
        menu_priority: "Priority", "Приоритет", "Пріоритет";
        menu_deadline: "Deadline", "Дедлайн", "Дедлайн";
        menu_schedule: "Plan for", "Запланировать на", "Запланувати на";
        move_to: "Move to", "Переместить в", "Перемістити до";
        move_up: "Move up", "Выше", "Вище";
        move_down: "Move down", "Ниже", "Нижче";
        duplicate: "Duplicate", "Дублировать", "Дублювати";

        // Projects
        new_project: "New project", "Новый проект", "Новий проєкт";
        edit_project: "Edit project", "Изменить проект", "Змінити проєкт";
        delete_project: "Delete project", "Удалить проект", "Видалити проєкт";
        delete_project_q: "Delete project “{name}”?", "Удалить проект «{name}»?", "Видалити проєкт «{name}»?";
        move_to_inbox: "Move its tasks to the Inbox", "Перенести задачи во «Входящие»", "Перенести завдання до «Вхідних»";
        delete_tasks_too: "Move its tasks to the Trash", "Переместить и его задачи в Корзину", "Перемістити й його завдання до Кошика";
        project_name_hint: "Project name", "Название проекта", "Назва проєкту";
        project_desc_hint: "Description (optional)", "Описание (необязательно)", "Опис (необовʼязково)";
        color: "Colour", "Цвет", "Колір";
        archive: "Archive", "Архивировать", "Архівувати";
        unarchive: "Restore from archive", "Вернуть из архива", "Повернути з архіву";
        archived_label: "Archived", "В архиве", "В архіві";
        archived_short: "archived", "архив", "архів";
        show_archived_label: "Show archived", "Показать архив", "Показати архів";
        hide_archived: "Hide archived", "Скрыть архив", "Сховати архів";
        project_progress: "{done} of {total} done", "Готово {done} из {total}", "Готово {done} з {total}";
        next_deadline: "Next: {date}", "Дедлайн: {date}", "Дедлайн: {date}";

        // Editor
        task: "Task", "Задача", "Завдання";
        title_hint: "What needs to be done?", "Что нужно сделать?", "Що потрібно зробити?";
        when: "When", "Когда", "Коли";
        scheduled_label: "Planned", "План", "План";
        repeat: "Repeat", "Повтор", "Повтор";
        rec_none: "None", "Нет", "Ні";
        rec_daily: "Daily", "Каждый день", "Щодня";
        rec_weekdays: "Weekdays", "По будням", "У будні";
        rec_weekly: "Weekly", "Каждую неделю", "Щотижня";
        rec_monthly: "Monthly", "Каждый месяц", "Щомісяця";
        rec_days: "Every N days", "Каждые N дней", "Кожні N днів";
        every: "Every", "Каждые", "Кожні";
        days_word: "days", "дн.", "дн.";
        until: "Until", "До", "До";
        rec_hint: "Completing one occurrence does not complete the series", "Выполнение одного повтора не закрывает всю серию", "Виконання одного повтору не закриває всю серію";
        reminder: "Reminder", "Напоминание", "Нагадування";
        remind_default: "Default ({n} min)", "По умолчанию ({n} мин)", "За замовчуванням ({n} хв)";
        remind_off: "Off", "Выкл.", "Вимк.";
        remind_at: "At the time", "В момент события", "У момент події";
        remind_min: "{n} min before", "За {n} мин", "За {n} хв";
        description: "Description", "Описание", "Опис";
        description_hint: "Notes", "Заметки", "Нотатки";
        links: "Links", "Ссылки", "Посилання";
        link_title_hint: "Title (optional)", "Название (необязательно)", "Назва (необовʼязково)";
        add_link_hint: "Paste a link and press Enter", "Вставьте ссылку и нажмите Enter", "Вставте посилання й натисніть Enter";
        bad_url: "This is not a valid web address", "Это не похоже на веб-адрес", "Це не схоже на вебадресу";
        checklist: "Checklist", "Чек-лист", "Чекліст";
        add_item_hint: "Add a checklist item", "Добавить пункт", "Додати пункт";
        created_updated: "Created {c} · Updated {u}", "Создана {c} · Изменена {u}", "Створено {c} · Змінено {u}";

        // Search
        search_hint: "Search tasks by title or description", "Поиск по названию и описанию", "Пошук за назвою й описом";
        search_prompt: "Start typing to search all tasks", "Начните вводить, чтобы искать по всем задачам", "Почніть вводити, щоб шукати серед усіх завдань";
        search_none: "No tasks found", "Ничего не найдено", "Нічого не знайдено";

        // PDF export
        export_pdf: "Export PDF", "Экспорт в PDF", "Експорт у PDF";
        pdf_saved: "PDF saved", "PDF сохранён", "PDF збережено";
        show_in_folder: "Show in folder", "Показать в папке", "Показати в теці";
        pdf_failed: "Could not create the PDF", "Не удалось создать PDF", "Не вдалося створити PDF";
        pdf_page: "Page {n} of {total}", "Страница {n} из {total}", "Сторінка {n} з {total}";
        pdf_profile: "Profile: {p}", "Профиль: {p}", "Профіль: {p}";
        pdf_deadline: "deadline", "дедлайн", "дедлайн";

        // Settings
        group_interface: "Interface", "Интерфейс", "Інтерфейс";
        group_behaviour: "Behaviour", "Поведение", "Поведінка";
        group_profiles: "Profiles", "Профили", "Профілі";
        group_data: "Data", "Данные", "Дані";
        week_start: "First day of the week", "Первый день недели", "Перший день тижня";
        monday: "Monday", "Понедельник", "Понеділок";
        sunday: "Sunday", "Воскресенье", "Неділя";
        time_format: "Time format", "Формат времени", "Формат часу";
        week_fixed_hint: "The week starts on Monday and time is shown in 24-hour format", "Неделя начинается с понедельника, время показывается в 24-часовом формате", "Тиждень починається з понеділка, час показується в 24-годинному форматі";
        autostart: "Start with Windows", "Запускать вместе с Windows", "Запускати разом із Windows";
        autostart_os: "Start with the system", "Запускать вместе с системой", "Запускати разом із системою";
        autostart_hint: "Planora starts minimised in the tray so reminders keep working", "Planora запускается свёрнутой в трей, чтобы напоминания работали", "Planora запускається згорнутою в трей, щоб нагадування працювали";
        autostart_failed: "Could not change the startup setting", "Не удалось изменить автозапуск", "Не вдалося змінити автозапуск";
        close_to_tray: "Close to tray", "Сворачивать в трей при закрытии", "Згортати в трей при закритті";
        close_to_tray_hint: "The close button hides Planora instead of quitting, so reminders keep working", "Кнопка закрытия прячет Planora в трей, а не завершает её, поэтому напоминания работают", "Кнопка закриття ховає Planora в трей, а не завершує її, тому нагадування працюють";
        notifications: "Notifications", "Уведомления", "Сповіщення";
        notifications_hint: "Show a Windows notification when a reminder is due", "Показывать уведомление Windows, когда подходит время напоминания", "Показувати сповіщення Windows, коли настає час нагадування";
        notifications_hint_os: "Show a desktop notification when a reminder is due", "Показывать уведомление на рабочем столе, когда подходит время напоминания", "Показувати сповіщення на робочому столі, коли настає час нагадування";
        default_reminder: "Default reminder", "Напоминание по умолчанию", "Нагадування за замовчуванням";
        open_data_folder: "Open data folder", "Открыть папку данных", "Відкрити теку даних";
        export_json: "Export JSON", "Экспорт в JSON", "Експорт у JSON";
        import_json: "Import JSON", "Импорт из JSON", "Імпорт із JSON";
        exported: "Data exported", "Данные экспортированы", "Дані експортовано";
        export_failed: "Export failed", "Не удалось экспортировать", "Не вдалося експортувати";
        import_failed: "That file is not a valid Planora export", "Этот файл не похож на экспорт Planora", "Цей файл не схожий на експорт Planora";
        backups: "Backups", "Резервные копии", "Резервні копії";
        backups_hint: "One automatic backup per day; the last 14 are kept", "Одна автокопия в день, хранятся последние 14", "Одна автокопія на день, зберігаються останні 14";
        no_backups: "No backups yet", "Копий пока нет", "Копій поки немає";
        restore: "Restore", "Восстановить", "Відновити";
        restore_failed: "This backup could not be read", "Не удалось прочитать эту копию", "Не вдалося прочитати цю копію";
        replace_q: "Replace all current data?", "Заменить все текущие данные?", "Замінити всі поточні дані?";
        replace_hint: "Your current data is kept as a safety copy in the backups folder", "Текущие данные сохранятся как страховочная копия в папке backups", "Поточні дані збережуться як страховинна копія в теці backups";
        replace: "Replace", "Заменить", "Замінити";
        data_replaced: "Data replaced", "Данные заменены", "Дані замінено";
        about: "About", "О программе", "Про програму";
        version: "Version", "Версия", "Версія";

        // Dialogs
        dismiss: "Dismiss", "Закрыть", "Закрити";
        missed_title: "Missed reminders", "Пропущенные напоминания", "Пропущені нагадування";
        missed_hint: "These came due while Planora was not running", "Их время наступило, пока Planora не была запущена", "Їхній час настав, поки Planora не була запущена";
        corrupt_title: "Your data file could not be read", "Не удалось прочитать файл данных", "Не вдалося прочитати файл даних";
        corrupt_text: "The file was renamed to {file} and Planora started empty. Daily backups are in the data folder under backups: open Settings and restore one.", "Файл переименован в {file}, Planora запущена с пустыми данными. Ежедневные копии лежат в папке данных, в подпапке backups: откройте «Настройки» и восстановите одну из них.", "Файл перейменовано на {file}, Planora запущена з порожніми даними. Щоденні копії лежать у теці даних, у підтеці backups: відкрийте «Налаштування» й відновіть одну з них.";

        // Delete confirmation and the Trash
        delete_task_q: "Delete task?", "Удалить задачу?", "Видалити завдання?";
        delete_subtask_q: "Delete subtask?", "Удалить подзадачу?", "Видалити підзавдання?";
        trash_note: "It will be moved to the Trash and can be restored for {n} days.", "Она попадёт в Корзину, и её можно будет восстановить в течение {n} дней.", "Воно потрапить до Кошика, і його можна буде відновити протягом {n} днів.";
        trash_note_many: "They will be moved to the Trash and can be restored for {n} days.", "Они попадут в Корзину, и их можно будет восстановить в течение {n} дней.", "Вони потраплять до Кошика, і їх можна буде відновити протягом {n} днів.";
        trash: "Trash", "Корзина", "Кошик";
        trash_hint: "Deleted tasks stay here for {n} days, then they are removed for good.", "Удалённые задачи лежат здесь {n} дней, потом удаляются навсегда.", "Видалені завдання лежать тут {n} днів, потім вилучаються назавжди.";
        trash_empty: "The Trash is empty", "Корзина пуста", "Кошик порожній";
        trash_empty_hint: "Deleted tasks will appear here", "Здесь появятся удалённые задачи", "Тут зʼявляться видалені завдання";
        empty_trash: "Empty trash", "Очистить корзину", "Очистити кошик";
        empty_trash_q: "Empty the Trash?", "Очистить корзину?", "Очистити кошик?";
        delete_forever: "Delete permanently", "Удалить навсегда", "Видалити назавжди";
        delete_forever_q: "Delete permanently?", "Удалить навсегда?", "Видалити назавжди?";
        cannot_undo: "This cannot be undone.", "Это нельзя отменить.", "Це неможливо скасувати.";
        deleted_when: "Deleted {when}", "Удалено {when}", "Видалено {when}";
        deletes_soon: "Deletes at the next cleanup", "Удалится при следующей очистке", "Видалиться під час наступного очищення";
        trash_subtask_of: "Subtask of {parent}", "Подзадача «{parent}»", "Підзавдання «{parent}»";
        restored: "Task restored", "Задача восстановлена", "Завдання відновлено";
        restored_inbox: "The project no longer exists: restored to the Inbox", "Проекта больше нет: задача восстановлена во «Входящие»", "Проєкту вже немає: завдання відновлено до «Вхідних»";
        restored_detached: "The parent task is gone: restored as a regular task", "Родительской задачи нет: восстановлена как обычная задача", "Батьківського завдання немає: відновлено як звичайне завдання";
        keep_deleted: "Keep deleted tasks for", "Хранить удалённые задачи", "Зберігати видалені завдання";
        days_n: "{n} days", "{n} дней", "{n} днів";
        keep_deleted_hint: "Old items are removed when Planora starts and once a day. If you shorten the period, items older than the new limit go at the next cleanup.", "Старые элементы удаляются при запуске Planora и раз в сутки. Если сократить срок, элементы старше нового срока исчезнут при ближайшей очистке.", "Старі елементи видаляються під час запуску Planora й раз на добу. Якщо скоротити строк, елементи старші за новий строк зникнуть під час найближчого очищення.";

        // Subtasks
        subtasks: "Subtasks", "Подзадачи", "Підзавдання";
        add_subtask: "Add subtask", "Добавить подзадачу", "Додати підзавдання";
        add_subtask_hint: "Add a subtask and press Enter", "Новая подзадача, затем Enter", "Нове підзавдання, потім Enter";
        subtask_of: "Subtask of", "Подзадача для", "Підзавдання для";
        convert_to_task: "Convert to task", "Сделать обычной задачей", "Зробити звичайним завданням";
        make_subtask: "Make subtask of…", "Сделать подзадачей…", "Зробити підзавданням…";
        pick_parent_hint: "Search for the parent task", "Найдите родительскую задачу", "Знайдіть батьківське завдання";
        no_parent_candidates: "No tasks to choose from", "Нет подходящих задач", "Немає завдань для вибору";
        no_subtask_recurring: "A repeating task cannot become a subtask", "Повторяющаяся задача не может стать подзадачей", "Завдання, що повторюється, не може стати підзавданням";
        no_subtask_has_subtasks: "A task with subtasks cannot become a subtask", "Задача с подзадачами не может стать подзадачей", "Завдання з підзавданнями не може стати підзавданням";
        subtasks_all_done: "All subtasks done, mark the task done?", "Все подзадачи выполнены, отметить задачу выполненной?", "Усі підзавдання виконано, позначити завдання виконаним?";
        mark_done: "Mark done", "Отметить", "Позначити";
        complete_parent_q: "Complete the subtasks too?", "Завершить и подзадачи?", "Завершити й підзавдання?";
        complete_all: "Complete all", "Завершить все", "Завершити все";
        only_the_task: "Only the task", "Только задачу", "Лише завдання";
        sub_location_hint: "Project and profile come from the parent task", "Проект и профиль берутся из родительской задачи", "Проєкт і профіль беруться з батьківського завдання";
        show_subtasks: "Show subtasks", "Показать подзадачи", "Показати підзавдання";
        hide_subtasks: "Hide subtasks", "Скрыть подзадачи", "Сховати підзавдання";
        drag_to_reorder: "Drag to reorder", "Перетащите, чтобы изменить порядок", "Перетягніть, щоб змінити порядок";

        // Tray and notifications
        tray_open: "Open Planora", "Открыть Planora", "Відкрити Planora";
        tray_new: "New task", "Новая задача", "Нове завдання";
        tray_today: "Today", "Сегодня", "Сьогодні";
        tray_exit: "Exit", "Выход", "Вихід";
        toast_now: "Now", "Сейчас", "Зараз";
        toast_in_min: "In {n} min", "Через {n} мин", "Через {n} хв";
    }
    plural {
        tasks_plural: ["{n} task", "{n} tasks", "{n} tasks"], ["{n} задача", "{n} задачи", "{n} задач"], ["{n} завдання", "{n} завдання", "{n} завдань"];
        items_plural: ["{n} item", "{n} items", "{n} items"], ["{n} элемент", "{n} элемента", "{n} элементов"], ["{n} елемент", "{n} елементи", "{n} елементів"];
        subtasks_plural: ["and {n} subtask", "and {n} subtasks", "and {n} subtasks"], ["и {n} подзадача", "и {n} подзадачи", "и {n} подзадач"], ["і {n} підзавдання", "і {n} підзавдання", "і {n} підзавдань"];
        open_subtasks_plural: ["{n} subtask is still open", "{n} subtasks are still open", "{n} subtasks are still open"], ["{n} подзадача ещё не выполнена", "{n} подзадачи ещё не выполнены", "{n} подзадач ещё не выполнены"], ["{n} підзавдання ще не виконано", "{n} підзавдання ще не виконано", "{n} підзавдань ще не виконано"];
        deletes_in_plural: ["Deletes in {n} day", "Deletes in {n} days", "Deletes in {n} days"], ["Удалится через {n} день", "Удалится через {n} дня", "Удалится через {n} дней"], ["Видалиться через {n} день", "Видалиться через {n} дні", "Видалиться через {n} днів"];
        projects_plural: ["{n} project", "{n} projects", "{n} projects"], ["{n} проект", "{n} проекта", "{n} проектов"], ["{n} проєкт", "{n} проєкти", "{n} проєктів"];
    }
}

#[cfg(test)]
mod tests {
    use super::S;

    /// The `{name}` placeholders of a string, sorted.
    fn placeholders(s: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = s;
        while let Some(a) = rest.find('{') {
            let Some(b) = rest[a..].find('}') else { break };
            out.push(rest[a..a + b + 1].to_string());
            rest = &rest[a + b + 1..];
        }
        out.sort();
        out
    }

    #[test]
    fn every_string_is_filled_in_all_three_languages() {
        for (key, [en, ru, uk]) in S::catalog() {
            for (lang, text) in [("en", en), ("ru", ru), ("uk", uk)] {
                assert!(!text.trim().is_empty(), "{key} is empty in {lang}");
                assert!(!text.contains("TODO"), "{key} has a TODO in {lang}");
            }
        }
    }

    #[test]
    fn placeholders_match_in_every_language() {
        for (key, [en, ru, uk]) in S::catalog() {
            assert_eq!(placeholders(en), placeholders(ru), "{key}: en and ru placeholders differ");
            assert_eq!(placeholders(en), placeholders(uk), "{key}: en and uk placeholders differ");
        }
    }

    #[test]
    fn russian_and_ukrainian_are_not_left_in_english() {
        let cyr = |s: &str| s.chars().any(|c| ('\u{400}'..='\u{4ff}').contains(&c));
        for (key, [en, ru, uk]) in S::catalog() {
            if en.chars().any(|c| c.is_alphabetic()) {
                assert!(cyr(ru), "{key} (ru) looks untranslated: {ru}");
                assert!(cyr(uk), "{key} (uk) looks untranslated: {uk}");
            }
        }
    }

    #[test]
    fn new_trash_and_subtask_strings_exist() {
        let keys: Vec<&str> = S::catalog().into_iter().map(|(k, _)| k).collect();
        for k in [
            "delete_task_q", "trash_note", "trash", "empty_trash", "delete_forever", "restored_inbox", "keep_deleted", "keep_deleted_hint",
            "subtasks", "add_subtask", "make_subtask", "convert_to_task", "subtasks_all_done", "complete_parent_q", "subtasks_plural", "deletes_in_plural",
        ] {
            assert!(keys.contains(&k), "missing string {k}");
        }
    }
}
