#include <gio/gio.h>

int gio_probe(void) {
    GFile *file = g_file_new_for_path("/");
    GFileInfo *info = g_file_query_info(file, G_FILE_ATTRIBUTE_STANDARD_TYPE,
                                      G_FILE_QUERY_INFO_NONE, NULL, NULL);
    gboolean local_ok = info != NULL &&
        g_file_info_get_file_type(info) == G_FILE_TYPE_DIRECTORY;
    gboolean tls_ok = g_tls_backend_supports_tls(g_tls_backend_get_default());
    g_print("local=%d tls=%d\n", local_ok, tls_ok);
    g_clear_object(&info);
    g_object_unref(file);
    return local_ok && tls_ok ? 0 : 1;
}

#ifndef RUST_PROBE
int main(void) { return gio_probe(); }
#endif
