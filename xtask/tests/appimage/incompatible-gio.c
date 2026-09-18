#include <gio/gio.h>

/*
 * ホスト側の新しいGVfsが、同梱GIOにない関数を要求する状況を再現する（#889）。
 * 実在するGIOの関数名は同梱GIOの版（build環境のUbuntu）によって存在してしまうため、
 * どの版のGIOにもない名前を使う（#1148）。
 */
extern void kukuri_fixture_missing_gio_symbol(GTask *, const gchar *);
/* 関数の初回呼出しではなく、実際のGVfs同様にdlopen時の解決を要求する。 */
G_MODULE_EXPORT void (*required_gio_symbol)(GTask *, const gchar *) = kukuri_fixture_missing_gio_symbol;

G_MODULE_EXPORT void g_io_module_load(GIOModule *module) {
    (void)module;
}

G_MODULE_EXPORT void g_io_module_unload(GIOModule *module) {
    (void)module;
}
