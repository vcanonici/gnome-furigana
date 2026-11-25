/* -*- mode: C; c-file-style: "gnu"; indent-tabs-mode: nil; -*- */
/*
 * gnome-furigana — "Furigana" panel for GNOME Settings 46.
 *
 * Edits the same GSettings keys as the Shell extension
 * (org.gnome.shell.extensions.furigana) and talks to the linguistic service
 * (io.github.vcanonici.Furigana1) for status and vocabulary reloads.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later
 */

#include "cc-furigana-panel.h"
#include "cc-furigana-resources.h"

#include <adwaita.h>
#include <json-glib/json-glib.h>

#define FURIGANA_SCHEMA "org.gnome.shell.extensions.furigana"
#define SERVICE_NAME "io.github.vcanonici.Furigana"
#define SERVICE_PATH "/io/github/vcanonici/Furigana"
#define SERVICE_INTERFACE "io.github.vcanonici.Furigana1"
#define SERVICE_TIMEOUT_MS 2000
#define CATALOG_DIR "/usr/share/locale-langpack/ja@furigana"
#define CATALOG_LANGUAGE "ja@furigana"
#define OVERRIDES_HEADER \
  "# Meu vocabulário gnome-furigana: termo<TAB>leitura<TAB>significado;outro sentido<TAB>classe\n" \
  "# Exemplo:\n# 設定\tせってい\tajustes\tsubstantivo\n"

struct _CcFuriganaPanel
{
  CcPanel       parent_instance;

  GtkStack     *stack;
  AdwSwitchRow *annotate_row;
  AdwComboRow  *form_row;
  AdwSpinRow   *scale_row;
  AdwSwitchRow *hover_row;
  AdwSpinRow   *delay_row;
  AdwComboRow  *scope_row;
  AdwActionRow *service_row;
  AdwActionRow *apps_row;

  GSettings    *settings;
  GCancellable *cancellable;
};

CC_PANEL_REGISTER (CcFuriganaPanel, cc_furigana_panel)

/* Nicks of the enum keys, in the order of the combo rows' string lists. */
static const gchar *const FORM_NICKS[] = { "ruby", "inline", NULL };
static const gchar *const SCOPE_NICKS[] = { "word", "label", NULL };

/* --- GSettings ↔ widget mappings ------------------------------------------------ */

static gboolean
annotate_to_active (GValue *value, GVariant *variant, gpointer user_data)
{
  g_value_set_boolean (value, g_strcmp0 (g_variant_get_string (variant, NULL), "off") != 0);
  return TRUE;
}

static GVariant *
active_to_annotate (const GValue *value, const GVariantType *type, gpointer user_data)
{
  return g_variant_new_string (g_value_get_boolean (value) ? "all" : "off");
}

static gboolean
nick_to_index (GValue *value, GVariant *variant, gpointer user_data)
{
  const gchar *const *nicks = user_data;
  const gchar *nick = g_variant_get_string (variant, NULL);

  for (guint i = 0; nicks[i] != NULL; i++)
    {
      if (g_strcmp0 (nicks[i], nick) == 0)
        {
          g_value_set_uint (value, i);
          return TRUE;
        }
    }
  return FALSE;
}

static GVariant *
index_to_nick (const GValue *value, const GVariantType *type, gpointer user_data)
{
  const gchar *const *nicks = user_data;
  guint index = g_value_get_uint (value);

  return index < g_strv_length ((gchar **) nicks) ? g_variant_new_string (nicks[index]) : NULL;
}

/* hover-delay-ms is stored in milliseconds and shown in seconds. */
static gboolean
milliseconds_to_seconds (GValue *value, GVariant *variant, gpointer user_data)
{
  g_value_set_double (value, g_variant_get_uint32 (variant) / 1000.0);
  return TRUE;
}

static GVariant *
seconds_to_milliseconds (const GValue *value, const GVariantType *type, gpointer user_data)
{
  return g_variant_new_uint32 ((guint32) (g_value_get_double (value) * 1000.0 + 0.5));
}

static gboolean
settings_has_key (GSettings *settings, const gchar *key)
{
  g_autoptr(GSettingsSchema) schema = NULL;

  g_object_get (settings, "settings-schema", &schema, NULL);
  return schema != NULL && g_settings_schema_has_key (schema, key);
}

/* Binds a row property to a key. A key missing from the installed schema (extension and
 * panel of different versions) hides the row: GSettings aborts the whole app otherwise. */
static void
bind_row (CcFuriganaPanel *self, const gchar *key, gpointer row, const gchar *property, GSettingsBindFlags flags,
          GSettingsBindGetMapping get_mapping, GSettingsBindSetMapping set_mapping, gpointer user_data)
{
  if (!settings_has_key (self->settings, key))
    {
      g_warning ("furigana: schema has no key '%s'; hiding its row", key);
      gtk_widget_set_visible (GTK_WIDGET (row), FALSE);
      return;
    }
  g_settings_bind_with_mapping (self->settings, key, row, property, flags, get_mapping, set_mapping, user_data,
                                NULL);
}

static void
bind_combo (CcFuriganaPanel *self, const gchar *key, AdwComboRow *row, const gchar *const *nicks)
{
  bind_row (self, key, row, "selected", G_SETTINGS_BIND_DEFAULT, nick_to_index, index_to_nick, (gpointer) nicks);
}

static void
bind_settings (CcFuriganaPanel *self)
{
  bind_row (self, "annotate", self->annotate_row, "active", G_SETTINGS_BIND_DEFAULT, annotate_to_active,
            active_to_annotate, NULL);
  bind_combo (self, "display-form", self->form_row, FORM_NICKS);
  bind_combo (self, "hover-scope", self->scope_row, SCOPE_NICKS);
  bind_row (self, "min-scale", self->scale_row, "value", G_SETTINGS_BIND_DEFAULT, NULL, NULL, NULL);
  bind_row (self, "hover-enabled", self->hover_row, "active", G_SETTINGS_BIND_DEFAULT, NULL, NULL, NULL);
  bind_row (self, "hover-delay-ms", self->delay_row, "value", G_SETTINGS_BIND_DEFAULT, milliseconds_to_seconds,
            seconds_to_milliseconds, NULL);
  bind_row (self, "hover-enabled", self->delay_row, "sensitive", G_SETTINGS_BIND_GET, NULL, NULL, NULL);
  bind_row (self, "hover-enabled", self->scope_row, "sensitive", G_SETTINGS_BIND_GET, NULL, NULL, NULL);
}

/* --- Linguistic service status ----------------------------------------------------- */

static gchar *
describe_health (const gchar *json)
{
  g_autoptr(JsonParser) parser = json_parser_new ();
  JsonObject *health;

  if (!json_parser_load_from_data (parser, json, -1, NULL) || !JSON_NODE_HOLDS_OBJECT (json_parser_get_root (parser)))
    return g_strdup ("Resposta inesperada do serviço");
  health = json_node_get_object (json_parser_get_root (parser));
  return g_strdup_printf ("Ativo · versão %s · %" G_GINT64_FORMAT " termos no glossário · %" G_GINT64_FORMAT
                          " no seu vocabulário · JMdict %s",
                          json_object_get_string_member_with_default (health, "version", "?"),
                          json_object_get_int_member_with_default (health, "glossary_terms", 0),
                          json_object_get_int_member_with_default (health, "override_terms", 0),
                          json_object_get_boolean_member_with_default (health, "jmdict", FALSE) ? "instalado"
                                                                                                : "ausente");
}

static void
on_service_reply (GObject *source, GAsyncResult *result, gpointer user_data)
{
  CcFuriganaPanel *self = user_data;
  g_autoptr(GError) error = NULL;
  g_autoptr(GVariant) reply = g_dbus_connection_call_finish (G_DBUS_CONNECTION (source), result, &error);
  g_autofree gchar *subtitle = NULL;
  const gchar *json = NULL;

  if (g_error_matches (error, G_IO_ERROR, G_IO_ERROR_CANCELLED))
    return;
  if (reply == NULL)
    {
      subtitle = g_strdup_printf ("Indisponível: %s", error->message);
    }
  else
    {
      g_variant_get (reply, "(&s)", &json);
      subtitle = describe_health (json);
    }
  adw_action_row_set_subtitle (self->service_row, subtitle);
}

static void
call_service (CcFuriganaPanel *self, const gchar *method)
{
  GDBusConnection *bus = g_bus_get_sync (G_BUS_TYPE_SESSION, self->cancellable, NULL);

  if (bus == NULL)
    {
      adw_action_row_set_subtitle (self->service_row, "Sessão D-Bus indisponível");
      return;
    }
  g_dbus_connection_call (bus, SERVICE_NAME, SERVICE_PATH, SERVICE_INTERFACE, method, NULL,
                          G_VARIANT_TYPE ("(s)"), G_DBUS_CALL_FLAGS_NONE, SERVICE_TIMEOUT_MS,
                          self->cancellable, on_service_reply, self);
  g_object_unref (bus);
}

/* --- GTK applications (ja@furigana gettext catalogs) ------------------------------- */

/* Catalogs are installed system-wide by install.sh; the session uses them when
 * LANGUAGE (from ~/.pam_environment at login) lists ja@furigana first. */
static void
describe_apps_status (CcFuriganaPanel *self)
{
  const gchar *language = g_getenv ("LANGUAGE");
  gboolean has_catalogs = g_file_test (CATALOG_DIR, G_FILE_TEST_IS_DIR);
  gboolean is_active = language != NULL && g_str_has_prefix (language, CATALOG_LANGUAGE);
  const gchar *subtitle;

  if (has_catalogs && is_active)
    subtitle = "Ativo: leituras 漢字(かんじ) nos apps GTK (Configurações, Arquivos…)";
  else if (has_catalogs)
    subtitle = "Catálogos instalados; ativam no próximo login";
  else
    subtitle = "Não instalado (reinstale sem --no-system)";
  adw_action_row_set_subtitle (self->apps_row, subtitle);
}

/* --- Actions ----------------------------------------------------------------------- */

static void
on_reload_clicked (CcFuriganaPanel *self)
{
  adw_action_row_set_subtitle (self->service_row, "Recarregando…");
  call_service (self, "Reload");
}

static GFile *
ensure_overrides_file (GError **error)
{
  g_autofree gchar *directory = g_build_filename (g_get_user_config_dir (), "gnome-furigana", NULL);
  g_autofree gchar *path = g_build_filename (directory, "overrides.tsv", NULL);

  if (g_mkdir_with_parents (directory, 0700) != 0)
    {
      g_set_error (error, G_IO_ERROR, g_io_error_from_errno (errno), "Não foi possível criar %s", directory);
      return NULL;
    }
  if (!g_file_test (path, G_FILE_TEST_EXISTS) && !g_file_set_contents (path, OVERRIDES_HEADER, -1, error))
    return NULL;
  return g_file_new_for_path (path);
}

static void
on_edit_vocabulary_activated (CcFuriganaPanel *self)
{
  g_autoptr(GError) error = NULL;
  g_autoptr(GFile) file = ensure_overrides_file (&error);
  g_autoptr(GtkFileLauncher) launcher = NULL;

  if (file == NULL)
    {
      g_warning ("gnome-furigana: %s", error->message);
      return;
    }
  launcher = gtk_file_launcher_new (file);
  gtk_file_launcher_launch (launcher, GTK_WINDOW (gtk_widget_get_root (GTK_WIDGET (self))), NULL, NULL, NULL);
}

static void
on_reset_activated (CcFuriganaPanel *self)
{
  g_autoptr(GSettingsSchema) schema = NULL;
  g_auto(GStrv) keys = NULL;

  g_object_get (self->settings, "settings-schema", &schema, NULL);
  keys = g_settings_schema_list_keys (schema);
  for (guint i = 0; keys[i] != NULL; i++)
    g_settings_reset (self->settings, keys[i]);
}

/* --- GObject ----------------------------------------------------------------------- */

static GSettings *
lookup_settings (void)
{
  GSettingsSchemaSource *source = g_settings_schema_source_get_default ();
  g_autoptr(GSettingsSchema) schema = source ? g_settings_schema_source_lookup (source, FURIGANA_SCHEMA, TRUE) : NULL;

  /* g_settings_new() aborts on a missing schema; show a status page instead. */
  return schema ? g_settings_new_full (schema, NULL, NULL) : NULL;
}

static void
cc_furigana_panel_dispose (GObject *object)
{
  CcFuriganaPanel *self = CC_FURIGANA_PANEL (object);

  g_cancellable_cancel (self->cancellable);
  g_clear_object (&self->cancellable);
  g_clear_object (&self->settings);
  G_OBJECT_CLASS (cc_furigana_panel_parent_class)->dispose (object);
}

static const char *
cc_furigana_panel_get_help_uri (CcPanel *panel)
{
  return "https://github.com/vcanonici/gnome-furigana#readme";
}

static void
cc_furigana_panel_class_init (CcFuriganaPanelClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);
  GtkWidgetClass *widget_class = GTK_WIDGET_CLASS (klass);
  CcPanelClass *panel_class = CC_PANEL_CLASS (klass);

  object_class->dispose = cc_furigana_panel_dispose;
  panel_class->get_help_uri = cc_furigana_panel_get_help_uri;

  gtk_widget_class_set_template_from_resource (widget_class, "/org/gnome/control-center/furigana/cc-furigana-panel.ui");
  gtk_widget_class_bind_template_child (widget_class, CcFuriganaPanel, stack);
  gtk_widget_class_bind_template_child (widget_class, CcFuriganaPanel, annotate_row);
  gtk_widget_class_bind_template_child (widget_class, CcFuriganaPanel, form_row);
  gtk_widget_class_bind_template_child (widget_class, CcFuriganaPanel, scale_row);
  gtk_widget_class_bind_template_child (widget_class, CcFuriganaPanel, hover_row);
  gtk_widget_class_bind_template_child (widget_class, CcFuriganaPanel, delay_row);
  gtk_widget_class_bind_template_child (widget_class, CcFuriganaPanel, scope_row);
  gtk_widget_class_bind_template_child (widget_class, CcFuriganaPanel, service_row);
  gtk_widget_class_bind_template_child (widget_class, CcFuriganaPanel, apps_row);
  gtk_widget_class_bind_template_callback (widget_class, on_reload_clicked);
  gtk_widget_class_bind_template_callback (widget_class, on_edit_vocabulary_activated);
  gtk_widget_class_bind_template_callback (widget_class, on_reset_activated);
}

static void
cc_furigana_panel_init (CcFuriganaPanel *self)
{
  g_resources_register (cc_furigana_get_resource ());
  gtk_widget_init_template (GTK_WIDGET (self));

  self->cancellable = g_cancellable_new ();
  self->settings = lookup_settings ();
  if (self->settings == NULL)
    {
      gtk_stack_set_visible_child_name (self->stack, "missing");
      return;
    }
  gtk_stack_set_visible_child_name (self->stack, "settings");
  bind_settings (self);
  describe_apps_status (self);
  call_service (self, "Health");
}
