# HyperProcessor RASP - Real-time Monitoring Guide

## 🚀 Как запустить мониторинг в реальном времени

### 1. Быстрый старт - Защита отдельных приложений

```bash
# Защитить конкретное приложение
./protect-app.sh firefox

# В режиме аудита (только логирование)
./protect-app.sh -a chrome

# С дополнительным whitelist
./protect-app.sh -w "libcustom.so,libplugin.so" myapp

# С debug логами
./protect-app.sh -l debug /usr/bin/app
```

### 2. Простой мониторинг процессов

```bash
# Запустить мониторинг (показывает незащищенные процессы)
./monitor-realtime.sh

# С кастомными настройками
MONITOR_INTERVAL=2 LOG_FILE=/tmp/rasp.log ./monitor-realtime.sh
```

### 3. Системный демон (для постоянной защиты)

#### Установка как systemd сервис:

```bash
# Установка файлов
sudo mkdir -p /opt/hyper_processor
sudo cp target/release/libhyper_processor.so /opt/hyper_processor/
sudo cp rasp-daemon.sh /opt/hyper_processor/rasp-daemon
sudo chmod +x /opt/hyper_processor/rasp-daemon

# Установка сервиса
sudo cp hyper-rasp-monitor.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable hyper-rasp-monitor
sudo systemctl start hyper-rasp-monitor

# Проверка статуса
sudo systemctl status hyper-rasp-monitor
```

#### Просмотр логов демона:

```bash
# Системные логи
sudo journalctl -u hyper-rasp-monitor -f

# Логи безопасности
sudo tail -f /var/log/hyper_rasp/security.log

# Аудит логи
sudo tail -f /var/log/hyper_rasp/audit.log
```

### 4. Защита через /etc/ld.so.preload (глобально)

⚠️ **ВНИМАНИЕ**: Это защитит ВСЕ процессы в системе!

```bash
# Добавить в /etc/ld.so.preload (требует root)
echo "/opt/hyper_processor/libhyper_processor.so" | sudo tee -a /etc/ld.so.preload

# Включить режим аудита глобально
sudo sh -c 'echo "HYPER_RASP_AUDIT_MODE=true" >> /etc/environment'
```

### 5. Защита конкретных сервисов

#### Для systemd сервисов:

```bash
# Создать drop-in конфигурацию
sudo systemctl edit myservice.service

# Добавить в файл:
[Service]
Environment="LD_PRELOAD=/opt/hyper_processor/libhyper_processor.so"
Environment="HYPER_RASP_AUDIT_MODE=true"
Environment="RUST_LOG=hyper_processor=info"
```

#### Для Docker контейнеров:

```bash
# В docker-compose.yml
services:
  myapp:
    image: myapp:latest
    environment:
      - LD_PRELOAD=/opt/rasp/libhyper_processor.so
      - HYPER_RASP_AUDIT_MODE=true
    volumes:
      - /opt/hyper_processor:/opt/rasp:ro
```

## 📊 Мониторинг и алерты

### Настройка алертов в реальном времени:

1. **Через journalctl:**
```bash
# Следить за security алертами
journalctl -f | grep "SECURITY ALERT"
```

2. **Через скрипт мониторинга:**
```bash
#!/bin/bash
# watch-alerts.sh
journalctl -f -o json | jq -r 'select(.MESSAGE | contains("SECURITY")) | .MESSAGE' | while read alert; do
    # Отправить уведомление
    notify-send "RASP Security Alert" "$alert"
    
    # Или в Telegram
    # curl -X POST "https://api.telegram.org/bot$TOKEN/sendMessage" \
    #      -d "chat_id=$CHAT_ID&text=RASP Alert: $alert"
done
```

3. **Интеграция с SIEM:**
```bash
# Rsyslog конфигурация для отправки в SIEM
echo '*.* @@siem-server:514' | sudo tee /etc/rsyslog.d/50-rasp.conf
```

## 🛡️ Режимы работы

### 1. **Blocking Mode** (по умолчанию)
- Немедленно завершает процесс при обнаружении неавторизованной библиотеки
- Максимальная защита
- Используйте после тестирования в audit mode

### 2. **Audit Mode**
- Только логирует обнаружения
- Процесс продолжает работу
- Идеально для начального развертывания

### 3. **Learning Mode** (с CLI)
```bash
# Включить режим обучения
HYPER_RASP_LEARNING_MODE=true HYPER_RASP_LEARNING_OUTPUT=./learned.yaml ./protect-app.sh myapp

# Сгенерировать whitelist из логов
./hyper-processor learn generate --from-audit-logs --output whitelist.yaml
```

## 🔧 Troubleshooting

### Проблема: Процесс не запускается
```bash
# Проверить в audit mode
HYPER_RASP_AUDIT_MODE=true LD_PRELOAD=/path/to/libhyper_processor.so myapp

# Посмотреть детальные логи
RUST_LOG=hyper_processor=debug myapp 2>&1 | grep hyper_processor
```

### Проблема: False positives
```bash
# Добавить библиотеку в whitelist
echo "whitelisted_filenames:" >> ~/.config/hyper_processor/rasp_config.yaml
echo "  - problematic_lib.so" >> ~/.config/hyper_processor/rasp_config.yaml
```

### Проблема: Не видно логов
```bash
# Проверить, что RUST_LOG установлен
export RUST_LOG=hyper_processor=info

# Для JSON логов в файл
RUST_LOG=hyper_processor=info myapp 2>&1 | tee rasp.log
```

## 📈 Метрики и статистика

Если включена поддержка метрик:

```bash
# Запустить с метриками
./hyper-processor --metrics-port 9090 &

# Посмотреть метрики
curl http://localhost:9090/metrics | grep hyper_processor
```

## 🚨 Best Practices

1. **Всегда начинайте с Audit Mode**
2. **Тестируйте на staging окружении**
3. **Ведите whitelist для каждого приложения отдельно**
4. **Регулярно проверяйте логи**
5. **Автоматизируйте алерты для security событий**
6. **Используйте learning mode для создания baseline**

## 📝 Примеры использования

### Защита веб-сервера:
```bash
# Nginx
sudo -E ./protect-app.sh -a nginx

# Apache
sudo -E ./protect-app.sh -w "mod_*.so" apache2
```

### Защита базы данных:
```bash
# PostgreSQL
./protect-app.sh -c /etc/hyper_processor/postgres_whitelist.yaml postgres

# MySQL
./protect-app.sh -a -l info mysqld
```

### Защита критических сервисов:
```bash
# SSH daemon
sudo systemctl edit sshd
# Добавить Environment="LD_PRELOAD=/opt/hyper_processor/libhyper_processor.so"
``` 