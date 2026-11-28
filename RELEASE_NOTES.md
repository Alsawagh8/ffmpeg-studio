## v0.4.2

### 🇬🇧 What's New
- **Complete Console Suppression on Windows:** Added `windows_subsystem = "windows"` and enforced `CREATE_NO_WINDOW` across all `std::process::Command` calls so zero terminal windows flash on startup or encoding.

---

### 🇸🇦 أبرز التحديثات
- **إخفاء الطرفية بالكامل على ويندوز:** ضبط واجهة النظام وإخفاء جميع العمليات الخلفية لمنع ظهور أي نافذة سوداء عند تشغيل البرنامج أو أثناء المعالجة.
