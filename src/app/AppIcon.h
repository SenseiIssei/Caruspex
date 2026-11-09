// The application icon: the title bar mark (gradient square with a car) drawn
// at any size. The window icon is rendered at runtime; the .ico, .icns and
// .png files for installers come from `caruspex --render-icon DIR` and
// tools/gen_icons.py.
#pragma once

#include <QIcon>
#include <QImage>

namespace caruspex {

// One square image, transparent around the rounded square.
QImage renderAppIcon(int size);

// All the usual sizes in one QIcon, for the windows of the app.
QIcon appIcon();

}  // namespace caruspex
