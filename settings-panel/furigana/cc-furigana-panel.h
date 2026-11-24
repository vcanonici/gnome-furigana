/* -*- mode: C; c-file-style: "gnu"; indent-tabs-mode: nil; -*- */
/*
 * gnome-furigana — "Furigana" panel for GNOME Settings 46.
 * SPDX-License-Identifier: GPL-2.0-or-later
 */

#pragma once

#include <shell/cc-panel.h>

G_BEGIN_DECLS

#define CC_TYPE_FURIGANA_PANEL (cc_furigana_panel_get_type ())
G_DECLARE_FINAL_TYPE (CcFuriganaPanel, cc_furigana_panel, CC, FURIGANA_PANEL, CcPanel)

G_END_DECLS
