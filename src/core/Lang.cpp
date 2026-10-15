#include "core/Lang.h"

#include <atomic>

namespace caruspex {
namespace {

// German stays the default so the CLI and the tests behave as before unless
// someone asks for English.
std::atomic<Language> current{Language::German};

}  // namespace

void setLanguage(Language language) { current.store(language); }

Language language() { return current.load(); }

}  // namespace caruspex
