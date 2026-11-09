#include "data/VehicleCatalog.h"

#include <algorithm>
#include <cstring>
#include <set>

namespace caruspex {

namespace catalog::detail {
// Defined in VehicleCatalog.gen.cpp.
const BrandInfo* brandTable(std::size_t& count);
const VehicleGeneration* generationTable(std::size_t& count);
}  // namespace catalog::detail

namespace {

// The newest model year a "still built" generation can have.
constexpr int kOpenEnd = 2027;

int endYear(const VehicleGeneration& g) { return g.yearTo ? g.yearTo : kOpenEnd; }

bool covers(const VehicleGeneration& g, int year) {
    return year >= g.yearFrom && year <= endYear(g);
}

// Lower case without accents, so "Škoda", "skoda" and "SKODA" are equal and
// "Citroen" finds "Citroën". Only the letters that occur in car names are
// mapped; everything else passes through.
std::string fold(std::string_view text) {
    std::string out;
    out.reserve(text.size());
    for (std::size_t i = 0; i < text.size(); ++i) {
        const unsigned char c = static_cast<unsigned char>(text[i]);
        if (c < 0x80) {
            out += static_cast<char>(c >= 'A' && c <= 'Z' ? c + 32 : c);
            continue;
        }
        if (i + 1 >= text.size()) break;
        const unsigned char d = static_cast<unsigned char>(text[i + 1]);
        const char* mapped = nullptr;
        if (c == 0xC3) {
            switch (d) {
                case 0x80: case 0x81: case 0x82: case 0x84: case 0x85:
                case 0xA0: case 0xA1: case 0xA2: case 0xA4: case 0xA5: mapped = "a"; break;
                case 0x87: case 0xA7: mapped = "c"; break;
                case 0x88: case 0x89: case 0x8A: case 0x8B:
                case 0xA8: case 0xA9: case 0xAA: case 0xAB: mapped = "e"; break;
                case 0x8D: case 0xAD: mapped = "i"; break;
                case 0x91: case 0xB1: mapped = "n"; break;
                case 0x93: case 0x94: case 0x96: case 0x98:
                case 0xB3: case 0xB4: case 0xB6: case 0xB8: mapped = "o"; break;
                case 0x9A: case 0x9C: case 0xBA: case 0xBC: mapped = "u"; break;
                case 0x9F: mapped = "ss"; break;
                default: break;
            }
        } else if (c == 0xC4 && (d == 0x8C || d == 0x8D)) {
            mapped = "c";
        } else if (c == 0xC5 && (d == 0xA0 || d == 0xA1)) {
            mapped = "s";
        } else if (c == 0xC5 && (d == 0xBD || d == 0xBE)) {
            mapped = "z";
        }
        if (mapped) {
            out += mapped;
            ++i;
        } else {
            out += static_cast<char>(c);
        }
    }
    return out;
}

bool same(std::string_view a, std::string_view b) { return fold(a) == fold(b); }

bool isAlnum(char c) {
    return (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9');
}

// Splits folded text into words at everything that is not a letter or digit.
std::vector<std::string> words(std::string_view folded) {
    std::vector<std::string> out;
    std::string cur;
    for (char c : folded) {
        if (isAlnum(c)) {
            cur += c;
        } else if (!cur.empty()) {
            out.push_back(cur);
            cur.clear();
        }
    }
    if (!cur.empty()) out.push_back(cur);
    return out;
}

std::string joined(std::string_view folded) {
    std::string out;
    for (char c : folded) {
        if (isAlnum(c)) out += c;
    }
    return out;
}

bool isDigits(const std::string& s) {
    return !s.empty() && std::all_of(s.begin(), s.end(), [](char c) { return c >= '0' && c <= '9'; });
}

int romanValue(const std::string& s) {
    static const char* const kRoman[] = {"i", "ii", "iii", "iv", "v", "vi", "vii",
                                         "viii", "ix", "x", "xi", "xii", "xiii", "xiv"};
    for (int i = 0; i < 14; ++i) {
        if (s == kRoman[i]) return i + 1;
    }
    return 0;
}

const char* romanText(int n) {
    static const char* const kRoman[] = {"i", "ii", "iii", "iv", "v", "vi", "vii",
                                         "viii", "ix", "x", "xi", "xii", "xiii", "xiv"};
    return n >= 1 && n <= 14 ? kRoman[n - 1] : "";
}

// Ordinal of a generation name: "IV" -> 4, "Mk4" -> 4, otherwise 0.
int ordinal(const std::string& foldedGeneration) {
    if (int r = romanValue(foldedGeneration)) return r;
    if (foldedGeneration.size() > 2 && foldedGeneration.compare(0, 2, "mk") == 0 &&
        isDigits(foldedGeneration.substr(2))) {
        return std::stoi(foldedGeneration.substr(2));
    }
    return 0;
}

// Search words of one generation, grouped by how much a hit counts.
struct SearchKeys {
    std::vector<std::string> brand;
    std::vector<std::string> model;        // words and the joined name
    std::vector<std::string> modelWords;   // only the words, to check coverage
    std::vector<std::string> generation;   // name, ordinal, VIN codes
    std::vector<std::string> alias;
};

void addUnique(std::vector<std::string>& v, std::string s) {
    if (!s.empty() && std::find(v.begin(), v.end(), s) == v.end()) v.push_back(std::move(s));
}

SearchKeys buildKeys(const VehicleGeneration& g) {
    SearchKeys k;
    const std::string brand = fold(g.brand);
    for (auto& w : words(brand)) addUnique(k.brand, w);
    addUnique(k.brand, joined(brand));

    const std::string model = fold(g.model);
    k.modelWords = words(model);
    for (auto& w : k.modelWords) addUnique(k.model, w);
    addUnique(k.model, joined(model));

    const std::string gen = fold(g.generation);
    for (auto& w : words(gen)) addUnique(k.generation, w);
    addUnique(k.generation, joined(gen));
    if (int n = ordinal(joined(gen))) {
        addUnique(k.generation, std::to_string(n));
        addUnique(k.generation, romanText(n));
        addUnique(k.generation, "mk" + std::to_string(n));
    }
    for (auto& w : words(fold(g.vinModelCodes))) addUnique(k.generation, w);

    std::string alias = fold(g.aliases);
    std::size_t start = 0;
    while (start <= alias.size()) {
        std::size_t bar = alias.find('|', start);
        if (bar == std::string::npos) bar = alias.size();
        const std::string phrase = alias.substr(start, bar - start);
        for (auto& w : words(phrase)) addUnique(k.alias, w);
        addUnique(k.alias, joined(phrase));
        start = bar + 1;
    }
    return k;
}

bool contains(const std::vector<std::string>& v, const std::string& s) {
    return std::find(v.begin(), v.end(), s) != v.end();
}

bool prefixOf(const std::vector<std::string>& v, const std::string& s) {
    if (s.size() < 3) return false;
    for (const auto& w : v) {
        if (w.size() > s.size() && w.compare(0, s.size(), s) == 0) return true;
    }
    return false;
}

// Points for one query word; 0 = no hit.
int scoreWord(const SearchKeys& k, const std::string& w) {
    if (contains(k.generation, w)) return 6;
    if (contains(k.model, w)) return 5;
    if (contains(k.alias, w)) return 4;
    if (contains(k.brand, w)) return 3;
    if (prefixOf(k.model, w) || prefixOf(k.brand, w) || prefixOf(k.alias, w)) return 2;
    return 0;
}

// "golf4" -> "golf", "4"; used when a word as a whole finds nothing.
std::vector<std::string> splitLettersDigits(const std::string& w) {
    std::vector<std::string> out;
    std::string cur;
    for (char c : w) {
        const bool digit = c >= '0' && c <= '9';
        if (!cur.empty() && digit != (cur.back() >= '0' && cur.back() <= '9')) {
            out.push_back(cur);
            cur.clear();
        }
        cur += c;
    }
    if (!cur.empty()) out.push_back(cur);
    return out;
}

const std::vector<SearchKeys>& allKeys() {
    static const std::vector<SearchKeys> keys = [] {
        std::size_t n = 0;
        const VehicleGeneration* t = catalog::detail::generationTable(n);
        std::vector<SearchKeys> v;
        v.reserve(n);
        for (std::size_t i = 0; i < n; ++i) v.push_back(buildKeys(t[i]));
        return v;
    }();
    return keys;
}

bool hasCode(const VehicleGeneration& g, const std::string& code) {
    if (code.empty()) return false;
    const std::string codes = std::string(" ") + g.vinModelCodes + " ";
    return codes.find(" " + code + " ") != std::string::npos;
}

const char* groupOf(const VehicleGeneration& g) {
    const BrandInfo* b = catalog::brandInfo(g.brand);
    return b ? b->group : "";
}

bool onlyElectric(unsigned pt) { return pt != 0 && (pt & ~static_cast<unsigned>(PtElectric)) == 0; }

std::string withYear(const char* pattern, int value) {
    std::string out = pattern;
    const std::size_t at = out.find("%d");
    if (at != std::string::npos) out.replace(at, 2, std::to_string(value));
    return out;
}

}  // namespace

namespace catalog {

const VehicleGeneration* all(std::size_t& count) { return detail::generationTable(count); }

std::size_t size() {
    std::size_t n = 0;
    detail::generationTable(n);
    return n;
}

std::vector<std::string> brands() {
    std::size_t n = 0;
    const BrandInfo* b = detail::brandTable(n);
    std::vector<std::string> out;
    for (std::size_t i = 0; i < n; ++i) out.emplace_back(b[i].name);
    std::sort(out.begin(), out.end(),
              [](const std::string& x, const std::string& y) { return fold(x) < fold(y); });
    return out;
}

const BrandInfo* brandInfo(std::string_view brand) {
    std::size_t n = 0;
    const BrandInfo* b = detail::brandTable(n);
    for (std::size_t i = 0; i < n; ++i) {
        if (same(b[i].name, brand)) return &b[i];
    }
    return nullptr;
}

std::vector<std::string> models(std::string_view brand) {
    std::size_t n = 0;
    const VehicleGeneration* t = all(n);
    std::vector<std::string> out;
    std::set<std::string> seen;
    for (std::size_t i = 0; i < n; ++i) {
        if (same(t[i].brand, brand) && seen.insert(fold(t[i].model)).second) out.emplace_back(t[i].model);
    }
    std::sort(out.begin(), out.end(),
              [](const std::string& x, const std::string& y) { return fold(x) < fold(y); });
    return out;
}

std::vector<const VehicleGeneration*> generations(std::string_view brand, std::string_view model) {
    std::size_t n = 0;
    const VehicleGeneration* t = all(n);
    std::vector<const VehicleGeneration*> out;
    for (std::size_t i = 0; i < n; ++i) {
        if (same(t[i].brand, brand) && same(t[i].model, model)) out.push_back(&t[i]);
    }
    std::stable_sort(out.begin(), out.end(), [](const VehicleGeneration* a, const VehicleGeneration* b) {
        return a->yearFrom < b->yearFrom;
    });
    return out;
}

const VehicleGeneration* findGeneration(std::string_view brand, std::string_view model, int year) {
    const auto list = generations(brand, model);
    if (list.empty()) return nullptr;
    if (year <= 0) return list.back();
    // Generations overlap in their change-over years; the newer one wins.
    const VehicleGeneration* best = nullptr;
    for (const VehicleGeneration* g : list) {
        if (covers(*g, year)) best = g;
    }
    if (best) return best;
    // Outside every range: the closest one.
    int bestDistance = 1 << 30;
    for (const VehicleGeneration* g : list) {
        const int d = year < g->yearFrom ? g->yearFrom - year : year - endYear(*g);
        if (d < bestDistance) {
            bestDistance = d;
            best = g;
        }
    }
    return best;
}

std::vector<const VehicleGeneration*> search(std::string_view query, std::size_t limit) {
    std::vector<std::string> tokens = words(fold(query));
    std::vector<const VehicleGeneration*> out;
    if (tokens.empty()) return out;

    std::size_t n = 0;
    const VehicleGeneration* t = all(n);
    const auto& keys = allKeys();

    bool yearQuery = false;
    int queryYear = 0;
    struct Hit {
        std::size_t index;
        int score;
    };
    std::vector<Hit> hits;

    for (std::size_t i = 0; i < n; ++i) {
        const SearchKeys& k = keys[i];
        int score = 0;
        bool ok = true;
        std::vector<std::string> matched;
        for (const std::string& w : tokens) {
            int s = scoreWord(k, w);
            if (s > 0) {
                score += s;
                matched.push_back(w);
                continue;
            }
            // A year: filters the generations instead of matching text.
            if (w.size() == 4 && isDigits(w)) {
                const int year = std::stoi(w);
                if (year >= 1950 && year <= 2049) {
                    yearQuery = true;
                    queryYear = year;
                    if (covers(t[i], year)) {
                        score += 2;
                        continue;
                    }
                    ok = false;
                    break;
                }
            }
            // "golf4" or "e46i": try the parts on their own.
            const auto parts = splitLettersDigits(w);
            if (parts.size() > 1) {
                int partScore = 0;
                bool allParts = true;
                for (const auto& p : parts) {
                    const int ps = scoreWord(k, p);
                    if (ps == 0) {
                        allParts = false;
                        break;
                    }
                    partScore += ps;
                    matched.push_back(p);
                }
                if (allParts) {
                    score += partScore;
                    continue;
                }
            }
            ok = false;
            break;
        }
        if (!ok) continue;
        // Bonus when the query names the whole model, so "golf 4" prefers
        // the Golf over the Golf Cabriolet.
        bool covered = !k.modelWords.empty();
        for (const auto& mw : k.modelWords) {
            if (!contains(matched, mw)) covered = false;
        }
        if (covered) score += 3;
        hits.push_back({i, score});
    }

    std::stable_sort(hits.begin(), hits.end(), [&](const Hit& a, const Hit& b) {
        if (a.score != b.score) return a.score > b.score;
        if (yearQuery) {
            // Among equal hits the generation that had just started that year
            // is more likely than one at the end of its life.
            const bool ca = covers(t[a.index], queryYear);
            const bool cb = covers(t[b.index], queryYear);
            if (ca != cb) return ca;
            return t[a.index].yearFrom > t[b.index].yearFrom;
        }
        return false;
    });
    for (const Hit& h : hits) {
        if (out.size() >= limit) break;
        out.push_back(&t[h.index]);
    }
    return out;
}

CatalogMatch matchVin(const VinInfo& vin) {
    CatalogMatch m;
    std::vector<std::string> brandList = vin.brandCandidates;
    if (brandList.empty() && !vin.brand.empty()) brandList.push_back(vin.brand);
    m.year = vin.modelYear;

    if (brandList.empty()) {
        m.reason = L("Der Hersteller ist unbekannt, daher gibt es keinen Treffer im Katalog.",
                     "The manufacturer is unknown, so there is no match in the catalog.");
        return m;
    }
    if (brandList.size() == 1) m.brand = brandList.front();

    std::size_t n = 0;
    const VehicleGeneration* t = all(n);
    auto brandFits = [&](const VehicleGeneration& g) {
        for (const auto& b : brandList) {
            if (same(g.brand, b)) return true;
        }
        return false;
    };

    // 1. The model code from the VIN, checked against the year candidates.
    if (!vin.modelCode.empty()) {
        std::vector<const VehicleGeneration*> byCode;
        for (std::size_t i = 0; i < n; ++i) {
            if (brandFits(t[i]) && hasCode(t[i], vin.modelCode)) byCode.push_back(&t[i]);
        }
        if (!byCode.empty()) {
            // Try the most likely year first, then the alternatives.
            for (std::size_t yi = 0; yi < vin.modelYears.size(); ++yi) {
                const int year = vin.modelYears[yi];
                std::vector<const VehicleGeneration*> fit;
                for (const VehicleGeneration* g : byCode) {
                    if (covers(*g, year)) fit.push_back(g);
                }
                if (fit.empty()) continue;
                // Several models can share a code (Golf IV and Bora are both
                // 1J); they keep the catalog order. Within one model the
                // newer generation wins in the change-over years.
                std::vector<const VehicleGeneration*> ordered;
                for (const VehicleGeneration* g : fit) {
                    auto same_model = std::find_if(ordered.begin(), ordered.end(),
                                                   [g](const VehicleGeneration* o) {
                                                       return std::strcmp(o->model, g->model) == 0 &&
                                                              std::strcmp(o->brand, g->brand) == 0;
                                                   });
                    if (same_model == ordered.end()) {
                        ordered.push_back(g);
                    } else if (g->yearFrom > (*same_model)->yearFrom) {
                        ordered.insert(same_model, g);
                    } else {
                        ordered.insert(same_model + 1, g);
                    }
                }
                fit = ordered;
                m.generation = fit.front();
                m.alternatives.assign(fit.begin() + 1, fit.end());
                m.year = year;
                m.brand = m.generation->brand;
                m.confidence = (yi == 0 && vin.modelYearCertain) ? 95 : 85;
                if (fit.size() > 1) m.confidence -= 15;
                m.reason = yi == 0
                    ? L("Modellcode und Baujahr aus der Fahrgestellnummer passen zu dieser Generation.",
                        "Model code and model year from the VIN match this generation.")
                    : L("Der Modellcode passt; das Baujahr ergibt sich erst über die Bauzeit dieser Generation.",
                        "The model code matches; the model year follows from the build period of this generation.");
                return m;
            }
            // No usable year: the code alone.
            m.generation = byCode.front();
            m.alternatives.assign(byCode.begin() + 1, byCode.end());
            m.brand = m.generation->brand;
            m.confidence = byCode.size() == 1 ? 75 : 55;
            if (vin.modelYears.empty()) {
                m.year = 0;
                m.reason = L("Der Modellcode passt, ein Baujahr steht nicht in der Fahrgestellnummer.",
                             "The model code matches, the VIN carries no model year.");
            } else {
                m.confidence -= 20;
                m.reason = L("Der Modellcode passt, aber das Baujahr liegt außerhalb der bekannten Bauzeit.",
                             "The model code matches, but the model year lies outside the known build period.");
            }
            return m;
        }
    }

    // 2. Brand and year only: everything of that brand built in that year.
    for (int year : vin.modelYears) {
        for (std::size_t i = 0; i < n; ++i) {
            if (brandFits(t[i]) && covers(t[i], year)) m.alternatives.push_back(&t[i]);
        }
        if (!m.alternatives.empty()) {
            m.year = year;
            break;
        }
    }
    if (m.alternatives.size() == 1) {
        m.generation = m.alternatives.front();
        m.alternatives.clear();
        m.brand = m.generation->brand;
        m.confidence = vin.modelYearCertain ? 70 : 55;
        m.reason = L("Von diesem Hersteller gab es in diesem Baujahr nur dieses Modell.",
                     "This manufacturer only built this model in that year.");
        return m;
    }
    m.confidence = m.brand.empty() ? 15 : 30;
    m.reason = L("Der Hersteller ist klar, das Modell steht nicht in der Fahrgestellnummer. "
                 "Bitte das Modell aus der Liste wählen.",
                 "The manufacturer is clear, the model is not in the VIN. "
                 "Please pick the model from the list.");
    return m;
}

std::string protocolOrderFor(const VehicleGeneration& g) { return g.protocolOrder; }

Connector connectorFor(const VehicleGeneration& g, int year) {
    if (year <= 0) year = g.yearFrom;
    return (g.connectorChange && year >= g.connectorChange) ? g.lateConnector : g.connector;
}

bool kklPossible(const VehicleGeneration& g, int year) {
    if (year <= 0) year = g.yearFrom;
    if (std::strcmp(groupOf(g), "vag") != 0 || year < 1990) return false;
    const Connector c = connectorFor(g, year);
    if (c == Connector::Vag2x2) return true;
    return c == Connector::Obd2 && (g.bus == BusKind::KLine || g.bus == BusKind::Mixed) && year <= 2008;
}

ObdSupport supportFor(const VehicleGeneration& g, int year, unsigned powertrain) {
    if (year <= 0) year = g.yearFrom;
    const unsigned pt = powertrain ? powertrain : g.powertrains;
    switch (connectorFor(g, year)) {
        case Connector::None: return ObdSupport::None;
        case Connector::Vag2x2: return ObdSupport::Kkl;
        case Connector::Bmw20:
        case Connector::Mb38:
        case Connector::Other: return ObdSupport::Manufacturer;
        case Connector::Obd2: break;
    }
    if (onlyElectric(pt)) return ObdSupport::Partial;
    if (g.usMarket) return year >= 1996 ? ObdSupport::Full : ObdSupport::Partial;
    // EU: petrol cars (and hybrids, gas cars) from 2001, diesels from 2004.
    const bool petrolLike = (pt & (PtPetrol | PtHybrid | PtPlugIn | PtGas)) != 0;
    const int threshold = petrolLike ? 2001 : 2004;
    return year >= threshold ? ObdSupport::Full : ObdSupport::Partial;
}

std::string yearsText(const VehicleGeneration& g) {
    if (g.yearTo == 0) return std::string(L("seit ", "since ")) + std::to_string(g.yearFrom);
    if (g.yearTo == g.yearFrom) return std::to_string(g.yearFrom);
    return std::to_string(g.yearFrom) + L(" bis ", " to ") + std::to_string(g.yearTo);
}

std::string displayName(const VehicleGeneration& g) {
    std::string out = std::string(g.brand) + " " + g.model;
    if (g.generation[0] && fold(g.generation) != fold(g.model)) out += std::string(" ") + g.generation;
    return out + " (" + yearsText(g) + ")";
}

std::string diagnosisText(const VehicleGeneration& g, int year) {
    if (year <= 0) year = g.yearFrom;
    const Connector c = connectorFor(g, year);
    const bool kkl = kklPossible(g, year);
    std::string out;
    switch (supportFor(g, year)) {
        case ObdSupport::None:
            out = L("Dieses Auto hat noch keine Diagnose, die ein Adapter auslesen kann. Fehler zeigen sich "
                    "höchstens als Blinkcode.",
                    "This car has no diagnosis yet that an adapter could read. Faults show up as blink "
                    "codes at most.");
            break;
        case ObdSupport::Kkl:
            out = L("VAG-Rundstecker (2x2 Pole). Mit einem KKL-Kabel liest Caruspex Fehlerspeicher und "
                    "Messwerte über KWP1281.",
                    "VAG round plugs (2x2 pins). With a KKL cable Caruspex reads fault memory and live "
                    "values over KWP1281.");
            break;
        case ObdSupport::Manufacturer:
            if (c == Connector::Bmw20) {
                out = L("BMW-Rundstecker mit 20 Polen im Motorraum. Ein ELM327 versteht ihn nicht, dafür "
                        "braucht es ein BMW-Diagnosegerät.",
                        "BMW round 20-pin plug in the engine bay. An ELM327 does not understand it, that "
                        "takes a BMW diagnostic tool.");
            } else if (c == Connector::Mb38) {
                out = L("Mercedes-Rundstecker mit 38 Polen im Motorraum. Ein ELM327 versteht ihn nicht, "
                        "dafür braucht es ein Mercedes-Diagnosegerät.",
                        "Mercedes round 38-pin plug in the engine bay. An ELM327 does not understand it, "
                        "that takes a Mercedes diagnostic tool.");
            } else if (year >= 2008) {
                out = L("Herstellereigener Diagnoseanschluss statt der genormten OBD-II-Buchse. Ein "
                        "ELM327 kann damit nicht ohne Weiteres sprechen.",
                        "Manufacturer diagnostic connector instead of the standard OBD-II socket. An "
                        "ELM327 cannot simply talk to it.");
            } else {
                out = L("Herstellereigener Diagnosestecker, falls vorhanden. Ein ELM327 kann damit nicht "
                        "sprechen, oft lassen sich aber Blinkcodes auslesen.",
                        "Manufacturer diagnostic plug, if fitted. An ELM327 cannot talk to it, but blink "
                        "codes can often be read.");
            }
            break;
        case ObdSupport::Partial:
            if (onlyElectric(g.powertrains)) {
                out = L("Reines Elektroauto: Abgaswerte gibt es nicht, daher antworten oft nur wenige "
                        "OBD-Messwerte und kaum Fehlercodes.",
                        "Pure electric car: there are no emissions to report, so often only a few OBD "
                        "values and hardly any trouble codes answer.");
            } else {
                out = L("16-polige OBD-Buchse vorhanden, aber OBD war für dieses Baujahr noch keine "
                        "Pflicht (EU: Benziner ab 2001, Diesel ab 2004; USA ab 1996). Viele "
                        "Motorsteuergeräte antworten trotzdem, ein Versuch lohnt sich.",
                        "16-pin OBD socket present, but OBD was not yet mandatory for this model year "
                        "(EU: petrol from 2001, diesel from 2004; USA from 1996). Many engine control "
                        "units answer anyway, it is worth a try.");
            }
            if (kkl) {
                out += L(" Sicher klappt es mit einem KKL-Kabel über KWP1281.",
                         " A KKL cable with KWP1281 works for sure.");
            }
            break;
        case ObdSupport::Full:
            out = L("Volle OBD-II-Diagnose über die 16-polige Buchse: Fehlercodes, Messwerte und "
                    "Bereitschaftstests.",
                    "Full OBD-II diagnosis through the 16-pin socket: trouble codes, live values and "
                    "readiness tests.");
            if (!g.usMarket && (g.powertrains & PtDiesel) && year < 2004) {
                out += L(" Bei Dieselmotoren dieses Baujahrs war OBD noch keine Pflicht.",
                         " For diesel engines of this model year OBD was not yet mandatory.");
            }
            if (kkl) {
                out += L(" Die übrigen Steuergeräte erreicht Caruspex mit einem KKL-Kabel.",
                         " Caruspex reaches the other control units with a KKL cable.");
            }
            break;
    }
    if (g.connectorChange && year < g.connectorChange && g.lateConnector != g.connector) {
        out += " ";
        out += withYear(L("Ab etwa %d: ", "From about %d: "), g.connectorChange);
        out += connectorName(g.lateConnector);
        out += ".";
    }
    const char* note = g.note.get();
    if (note && *note) {
        out += " ";
        out += note;
    }
    return out;
}

}  // namespace catalog

const char* bodyName(BodyType body) {
    switch (body) {
        case BodyType::Hatchback: return L("Schrägheck", "Hatchback");
        case BodyType::Sedan: return L("Limousine", "Sedan");
        case BodyType::Estate: return L("Kombi", "Estate");
        case BodyType::Suv: return L("SUV", "SUV");
        case BodyType::Van: return L("Transporter", "Van");
        case BodyType::Coupe: return L("Coupé", "Coupe");
        case BodyType::Convertible: return L("Cabrio", "Convertible");
        case BodyType::Pickup: return L("Pick-up", "Pickup");
        case BodyType::Mpv: return L("Van (MPV)", "MPV");
        case BodyType::Microcar: return L("Kleinstwagen", "Microcar");
        case BodyType::Roadster: return L("Roadster", "Roadster");
    }
    return "";
}

const char* connectorName(Connector connector) {
    switch (connector) {
        case Connector::None: return L("kein Diagnosestecker", "no diagnostic connector");
        case Connector::Obd2: return L("16-polige OBD-II-Buchse", "16-pin OBD-II socket");
        case Connector::Vag2x2: return L("VAG-Rundstecker (2x2), KKL-Kabel", "VAG round plugs (2x2), KKL cable");
        case Connector::Bmw20: return L("BMW-Rundstecker, 20 Pole", "BMW round plug, 20 pins");
        case Connector::Mb38: return L("Mercedes-Rundstecker, 38 Pole", "Mercedes round plug, 38 pins");
        case Connector::Other: return L("herstellereigener Stecker", "manufacturer plug");
    }
    return "";
}

const char* locationText(SocketLocation location) {
    switch (location) {
        case SocketLocation::Unknown:
            return L("Meist unter dem Armaturenbrett auf der Fahrerseite, manchmal hinter einer Abdeckung "
                     "oder in der Mittelkonsole.",
                     "Usually under the dashboard on the driver side, sometimes behind a cover or in the "
                     "centre console.");
        case SocketLocation::UnderDashLeft:
            return L("Unter dem Armaturenbrett auf der Fahrerseite, nahe der Lenksäule.",
                     "Under the dashboard on the driver side, near the steering column.");
        case SocketLocation::UnderSteeringColumn:
            return L("Direkt unter der Lenksäule.", "Directly below the steering column.");
        case SocketLocation::FootwellCover:
            return L("Im Fahrerfußraum hinter einer Abdeckung.", "In the driver's footwell behind a cover.");
        case SocketLocation::FuseBox:
            return L("Am oder im Sicherungskasten auf der Fahrerseite, hinter dessen Deckel.",
                     "At or in the fuse box on the driver side, behind its lid.");
        case SocketLocation::CentreConsole:
            return L("In der Mittelkonsole hinter einer Abdeckung vor dem Schalthebel.",
                     "In the centre console behind a cover in front of the gear lever.");
        case SocketLocation::BehindAshtray:
            return L("Hinter oder unter dem Aschenbecher in der Mittelkonsole.",
                     "Behind or below the ashtray in the centre console.");
        case SocketLocation::PassengerSide:
            return L("Unter dem Armaturenbrett auf der Beifahrerseite.",
                     "Under the dashboard on the passenger side.");
        case SocketLocation::EngineBay:
            return L("Im Motorraum, meist unter einer runden Kappe.",
                     "In the engine bay, usually under a round cap.");
        case SocketLocation::RearConsole:
            return L("Hinten an der Mittelkonsole beziehungsweise unter den Rücksitzen.",
                     "At the rear of the centre console or under the rear seats.");
    }
    return "";
}

std::string locationText(const VehicleGeneration& g, int year) {
    if (g.location != SocketLocation::Unknown) return locationText(g.location);
    switch (catalog::connectorFor(g, year)) {
        case Connector::None:
            return L("Kein Diagnosestecker.", "No diagnostic connector.");
        case Connector::Bmw20:
        case Connector::Mb38:
            return locationText(SocketLocation::EngineBay);
        case Connector::Vag2x2:
            return L("Je nach Modell im Sicherungskasten unter dem Armaturenbrett, unter der Schaltkulisse "
                     "oder im Motorraum.",
                     "Depending on the model in the fuse box under the dashboard, under the gear lever "
                     "gaiter or in the engine bay.");
        case Connector::Other:
            return L("Herstellerstecker, meist im Motorraum oder unter dem Armaturenbrett.",
                     "Manufacturer plug, usually in the engine bay or under the dashboard.");
        case Connector::Obd2:
            break;
    }
    return locationText(SocketLocation::Unknown);
}

std::string powertrainText(unsigned powertrains) {
    struct Name {
        unsigned flag;
        Text text;
    };
    static const Name kNames[] = {
        {PtPetrol, {"Benzin", "Petrol"}},
        {PtDiesel, {"Diesel", "Diesel"}},
        {PtHybrid, {"Hybrid", "Hybrid"}},
        {PtPlugIn, {"Plug-in-Hybrid", "Plug-in hybrid"}},
        {PtElectric, {"Elektro", "Electric"}},
        {PtGas, {"Gas (LPG/CNG)", "Gas (LPG/CNG)"}},
    };
    std::string out;
    for (const Name& n : kNames) {
        if (!(powertrains & n.flag)) continue;
        if (!out.empty()) out += ", ";
        out += n.text.get();
    }
    return out;
}

const char* supportName(ObdSupport support) {
    switch (support) {
        case ObdSupport::None: return L("keine Diagnose", "no diagnosis");
        case ObdSupport::Manufacturer: return L("nur Herstellerdiagnose", "manufacturer diagnosis only");
        case ObdSupport::Kkl: return L("KKL-Kabel (KWP1281)", "KKL cable (KWP1281)");
        case ObdSupport::Partial: return L("OBD-II teilweise", "OBD-II in part");
        case ObdSupport::Full: return L("OBD-II vollständig", "full OBD-II");
    }
    return "";
}

}  // namespace caruspex
