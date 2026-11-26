MOGGER AI — запуск на Windows и Android/Termux
================================================

ВАЖНО
- Один Telegram bot token должен обслуживаться только одним работающим экземпляром polling-бота.
- Не запускайте бота одновременно на ПК и телефоне. Иначе процессы будут конкурировать за getUpdates.
- users.json и memory.json хранятся локально рядом с программой. Между ПК и телефоном они автоматически НЕ синхронизируются.
- Для постоянной работы с одной общей памятью запускайте один экземпляр на постоянно включённом ПК/VPS либо отдельно настройте синхронизацию/общую БД.
- Никогда не публикуйте токен и файлы памяти.

WINDOWS 10/11
1. Установите Rust: https://rustup.rs/
2. Для сборки под Windows установите Microsoft C++ Build Tools:
   https://visualstudio.microsoft.com/visual-cpp-build-tools/
   Выберите workload "Desktop development with C++".
3. Откройте папку проекта в Проводнике.
4. Дважды нажмите windows_build.bat. Дождитесь успешной сборки.
5. Дважды нажмите windows_start.bat и вставьте токен Telegram-бота.
6. Окно консоли должно оставаться открытым. Ctrl+C останавливает бота.

ANDROID / TERMUX
1. Установите Termux из F-Droid или официального GitHub проекта Termux.
2. Распакуйте проект в доступную Termux папку, например ~/Mogger.
   Если архив в Downloads:
     termux-setup-storage
     mkdir -p ~/Mogger
     cp -r ~/storage/downloads/Mogger-crossplatform/* ~/Mogger/
   Если имя распакованной папки отличается, поправьте путь.
3. В Termux:
     cd ~/Mogger
     bash termux_setup.sh
4. Запуск:
     bash termux_start.sh
5. Для фоновой сессии:
     pkg install tmux -y
     tmux new -s mogger
     bash termux_start.sh
   Отсоединиться: Ctrl+B, затем D
   Вернуться: tmux attach -t mogger
   Остановить: tmux attach -t mogger, затем Ctrl+C

ПЕРЕКЛЮЧЕНИЕ МЕЖДУ ПК И ТЕЛЕФОНОМ
1. Остановите процесс на первом устройстве (Ctrl+C).
2. Скопируйте проект на другое устройство.
3. Скопируйте memory.json и users.json, если хотите перенести локальную память и список участников.
4. Запустите там бота.
Не переносите эти JSON-файлы в публичные репозитории или чаты.

TELEGRAM
Для чтения обычных сообщений группы отключите Privacy Mode через @BotFather:
 /setprivacy -> выбрать бота -> Disable
При необходимости удалите бота из группы и добавьте заново.
Убедитесь, что у бота есть разрешение читать сообщения.

СБОРКА
Cargo.toml уже содержит зависимости проекта. Команда:
  cargo build --release

Бинарные файлы разных ОС/архитектур несовместимы:
- Windows: target/release/tgbot.exe
- Android arm64 (Termux): target/release/tgbot
Собирайте отдельно на каждом устройстве.


MULTI-API FALLBACK
------------------
Провайдеры проверяются последовательно:
  1. Groq      (если задан GROQ_API_KEY)
  2. Gemini    (если задан GEMINI_API_KEY)
  3. OpenRouter (если задан OPENROUTER_API_KEY)
  4. Pollinations (последний резерв; POLLINATIONS_API_KEY необязателен)

Windows PowerShell, пример:
  $env:TG_TOKEN="ВАШ_TELEGRAM_BOT_TOKEN"
  $env:GROQ_API_KEY="ВАШ_GROQ_API_KEY"
  $env:GEMINI_API_KEY="ВАШ_GEMINI_API_KEY"
  $env:OPENROUTER_API_KEY="ВАШ_OPENROUTER_API_KEY"
  $env:POLLINATIONS_API_KEY="ВАШ_POLLINATIONS_API_KEY"
  .\target\release\tgbot.exe

Можно задать только один или несколько ключей. Незаданные провайдеры пропускаются,
Pollinations остаётся последним резервом. Для большинства провайдеров нужен собственный
API-ключ; наличие нескольких провайдеров не означает безлимитную бесплатную работу.
Бесплатные квоты, доступность моделей и условия могут меняться у самих провайдеров.

ОБРАБОТКА СООБЩЕНИЙ
-------------------
В группе бот теперь обрабатывает каждое обычное текстовое сообщение, которое Telegram
передал боту. Сообщения обрабатываются последовательно, чтобы ответы не перемешивались.
Команды (например /start, /battle, /mog) обрабатываются отдельно.
Для чтения обычных сообщений группы отключите Privacy Mode через @BotFather:
  /setprivacy -> выбрать бота -> Disable

Бот также реагирует на подписи к медиа и на распространённые типы вложений (стикер, фото, видео, голосовое, GIF, файл) текстовым комментарием; содержимое вложений он не распознаёт.
