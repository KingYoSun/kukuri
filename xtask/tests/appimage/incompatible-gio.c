#include <gio/gio.h>

/* ビルド環境のGIOにはない、新しいGVfsが要求する関数を再現する。 */
extern void g_task_set_static_name(GTask *, const gchar *);
/* 関数の初回呼出しではなく、実際のGVfs同様にdlopen時の解決を要求する。 */
G_MODULE_EXPORT void (*required_gio_symbol)(GTask *, const gchar *) = g_task_set_static_name;

G_MODULE_EXPORT void g_io_module_load(GIOModule *module) {
    (void)module;
}

G_MODULE_EXPORT void g_io_module_unload(GIOModule *module) {
    (void)module;
}
