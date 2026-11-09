#include "data/PidPacks.h"

#include "core/Lang.h"
#include "data/PidPacksTable.h"

namespace caruspex {
namespace {

using pidpacks::detail::PackRow;
using pidpacks::detail::ValueRow;

std::vector<CustomPid> build(bool english) {
    std::size_t packCount = 0, valueCount = 0;
    const PackRow* packs = pidpacks::detail::packTable(packCount);
    const ValueRow* rows = pidpacks::detail::valueTable(valueCount);
    std::vector<CustomPid> out;
    out.reserve(valueCount);
    for (std::size_t i = 0; i < valueCount; ++i) {
        const ValueRow& r = rows[i];
        const PackRow& pack = packs[static_cast<std::size_t>(r.pack)];
        CustomPid pid;
        pid.name = english ? r.name.en : r.name.de;
        pid.shortName = english ? r.shortName.en : r.shortName.de;
        pid.request = r.request;
        pid.header = r.header;
        pid.response = r.response;
        pid.equation = r.equation;
        pid.minValue = r.minValue;
        pid.maxValue = r.maxValue;
        pid.unit = r.unit;
        pid.pack = pack.id;
        pid.source = r.source;
        pid.key = kPackKeyBase + static_cast<int>(i);
        pid.category = pack.category;
        out.push_back(std::move(pid));
    }
    return out;
}

const PackRow* findPack(const std::string& name) {
    std::size_t count = 0;
    const PackRow* packs = pidpacks::detail::packTable(count);
    for (std::size_t i = 0; i < count; ++i) {
        if (name == packs[i].id) return &packs[i];
    }
    return nullptr;
}

}  // namespace

const std::vector<CustomPid>& builtInPacks() {
    static const std::vector<CustomPid> german = build(false);
    static const std::vector<CustomPid> english = build(true);
    return language() == Language::English ? english : german;
}

std::vector<std::string> packNames() {
    std::size_t count = 0;
    const PackRow* packs = pidpacks::detail::packTable(count);
    std::vector<std::string> out;
    for (std::size_t i = 0; i < count; ++i) out.emplace_back(packs[i].id);
    return out;
}

std::string packTitle(const std::string& name) {
    const PackRow* pack = findPack(name);
    return pack ? pack->title.get() : std::string();
}

PidCategory packCategory(const std::string& name) {
    const PackRow* pack = findPack(name);
    return pack ? pack->category : PidCategory::Custom;
}

std::vector<CustomPid> packPids(const std::string& name) {
    std::vector<CustomPid> out;
    for (const CustomPid& pid : builtInPacks()) {
        if (pid.pack == name) out.push_back(pid);
    }
    return out;
}

}  // namespace caruspex
