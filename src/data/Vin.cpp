#include "data/Vin.h"

#include <cctype>
#include <cstring>
#include <ctime>

namespace caruspex {
namespace {

using Y = VinYearRule;
using C = VinModelCodeRule;

// Short names keep the table below readable.
constexpr Y kStd = Y::Standard;
constexpr Y kUnk = Y::Unknown;
constexpr Y kNoYear = Y::None;
constexpr C kNone = C::None;
constexpr C kVag = C::Vag78;
constexpr C kMb = C::Mercedes456;
constexpr C kPor = C::Porsche78;
constexpr C kTesla = C::Tesla4;

// Plant countries that differ from the ISO range of the code.
constexpr Text kInAustria = {"Österreich", "Austria"};
constexpr Text kInGermany = {"Deutschland", "Germany"};
constexpr Text kInFrance = {"Frankreich", "France"};
constexpr Text kInSpain = {"Spanien", "Spain"};

// World manufacturer identifiers. Three character entries are exact, two
// character entries catch the rest of a range that belongs to one maker
// (JT is Toyota unless a more specific entry says otherwise).
//
// Sources are the public WMI lists (SAE, NHTSA, ISO 3780 ranges). Where a
// code is shared by several brands of one group, all of them are listed and
// the catalog decides by the model code or leaves it open.
const WmiEntry kWmi[] = {
    // --- Germany ---------------------------------------------------------
    {"WAU", "Audi AG", "Audi", kStd, kVag, {}},
    {"WA1", "Audi AG (SUV)", "Audi", kStd, kVag, {}},
    {"WUA", "Audi Sport GmbH", "Audi", kStd, kVag, {}},
    {"TRU", "Audi Hungaria", "Audi", kStd, kVag, {}},
    {"WVW", "Volkswagen AG", "Volkswagen", kStd, kVag, {}},
    {"WVG", "Volkswagen AG (SUV)", "Volkswagen", kStd, kVag, {}},
    {"WV1", "Volkswagen Nutzfahrzeuge", "Volkswagen", kStd, kVag, {}},
    {"WV2", "Volkswagen Nutzfahrzeuge", "Volkswagen", kStd, kVag, {}},
    {"1VW", "Volkswagen of America", "Volkswagen", kStd, kVag, {}},
    {"1V1", "Volkswagen of America", "Volkswagen", kStd, kVag, {}},
    {"1V2", "Volkswagen of America (SUV)", "Volkswagen", kStd, kVag, {}},
    {"3VW", "Volkswagen de México", "Volkswagen", kStd, kVag, {}},
    {"3VV", "Volkswagen de México (SUV)", "Volkswagen", kStd, kVag, {}},
    {"9BW", "Volkswagen do Brasil", "Volkswagen", kStd, kVag, {}},
    {"8AW", "Volkswagen Argentina", "Volkswagen", kStd, kVag, {}},
    {"AAV", "Volkswagen South Africa", "Volkswagen", kStd, kVag, {}},
    {"LSV", "SAIC Volkswagen", "Volkswagen|Škoda", kStd, kVag, {}},
    {"LFV", "FAW-Volkswagen", "Volkswagen|Audi", kStd, kVag, {}},
    {"XW8", "Volkswagen Group Rus", "Volkswagen|Škoda", kStd, kVag, {}},
    {"TMB", "Škoda Auto", "Škoda", kStd, kVag, {}},
    {"VSS", "SEAT", "SEAT|Cupra", kStd, kVag, {}},
    {"WP0", "Porsche AG", "Porsche", kStd, kPor, {}},
    {"WP1", "Porsche AG (SUV)", "Porsche", kStd, kPor, {}},
    {"WBA", "BMW AG", "BMW", kUnk, kNone, {}},
    {"WBS", "BMW M GmbH", "BMW", kUnk, kNone, {}},
    {"WBX", "BMW AG (SUV)", "BMW", kUnk, kNone, {}},
    {"WBY", "BMW AG (BMW i)", "BMW", kUnk, kNone, {}},
    {"4US", "BMW Manufacturing USA", "BMW", kStd, kNone, {}},
    {"5UX", "BMW Manufacturing USA (SUV)", "BMW", kStd, kNone, {}},
    {"5YM", "BMW Manufacturing USA (M SUV)", "BMW", kStd, kNone, {}},
    {"3MW", "BMW de México", "BMW", kStd, kNone, {}},
    {"LBV", "BMW Brilliance", "BMW", kStd, kNone, {}},
    {"WMW", "BMW AG (MINI)", "Mini", kUnk, kNone, {}},
    {"WAP", "Alpina", "Alpina", kUnk, kNone, {}},
    {"WDB", "Mercedes-Benz", "Mercedes-Benz|Maybach", kUnk, kMb, {}},
    {"WDD", "Mercedes-Benz (Pkw)", "Mercedes-Benz", kUnk, kMb, {}},
    {"WDC", "Mercedes-Benz (SUV)", "Mercedes-Benz", kUnk, kMb, {}},
    {"WDF", "Mercedes-Benz (Transporter)", "Mercedes-Benz", kUnk, kMb, {}},
    {"W1K", "Mercedes-Benz (Pkw)", "Mercedes-Benz", kUnk, kMb, {}},
    {"W1N", "Mercedes-Benz (SUV)", "Mercedes-Benz", kUnk, kMb, {}},
    {"W1V", "Mercedes-Benz (Transporter)", "Mercedes-Benz", kUnk, kMb, {}},
    {"W1W", "Mercedes-Benz (Transporter, Nordamerika)", "Mercedes-Benz", kStd, kNone, {}},
    {"4JG", "Mercedes-Benz U.S. International (SUV)", "Mercedes-Benz", kStd, kNone, {}},
    {"55S", "Mercedes-Benz U.S. International", "Mercedes-Benz", kStd, kNone, {}},
    {"LE4", "Beijing Benz", "Mercedes-Benz", kStd, kNone, {}},
    {"9BM", "Mercedes-Benz do Brasil", "Mercedes-Benz", kStd, kNone, {}},
    {"WME", "smart", "Smart", kUnk, kMb, {}},
    {"TCC", "Micro Compact Car", "Smart", kUnk, kMb, {}},
    {"W1A", "smart", "Smart", kUnk, kNone, {}},
    {"W0L", "Adam Opel AG", "Opel", kStd, kNone, {}},
    {"W0V", "Opel Automobile GmbH", "Opel", kStd, kNone, {}},
    {"VXK", "Opel Automobile GmbH (Stellantis)", "Opel", kUnk, kNone, kInFrance},
    {"VXE", "Opel Automobile GmbH (Stellantis, Transporter)", "Opel", kUnk, kNone, kInFrance},
    {"VSX", "Opel España", "Opel", kStd, kNone, {}},
    {"WF0", "Ford-Werke (Ford of Europe)", "Ford", kUnk, kNone, {}},
    {"WF1", "Ford-Werke (Merkur)", "Merkur", kStd, kNone, {}},
    {"WMA", "MAN Truck & Bus", "MAN", kUnk, kNone, {}},
    {"WZ1", "Toyota Supra, gebaut von Magna Steyr", "Toyota", kStd, kNone, kInAustria},
    {"XP7", "Tesla Gigafactory Berlin-Brandenburg", "Tesla", kStd, kTesla, kInGermany},

    // --- France ----------------------------------------------------------
    {"VF1", "Renault", "Renault|Dacia|Alpine", kUnk, kNone, {}},
    {"VF2", "Renault", "Renault", kUnk, kNone, {}},
    {"VF6", "Renault Trucks", "Renault", kUnk, kNone, {}},
    {"VS5", "Renault España", "Renault", kUnk, kNone, {}},
    {"VFA", "Alpine", "Alpine", kUnk, kNone, {}},
    {"VF3", "Peugeot", "Peugeot", kUnk, kNone, {}},
    {"VR3", "Peugeot (Stellantis)", "Peugeot", kUnk, kNone, {}},
    {"VF7", "Citroën", "Citroën|DS", kUnk, kNone, {}},
    {"VR7", "Citroën (Stellantis)", "Citroën", kUnk, kNone, {}},
    {"VS7", "Citroën España", "Citroën", kUnk, kNone, {}},
    {"VR1", "DS Automobiles", "DS", kUnk, kNone, {}},
    {"VF8", "Matra Automobile", "Matra", kUnk, kNone, {}},
    {"VF9", "Bugatti Automobiles", "Bugatti", kUnk, kNone, {}},
    {"VNK", "Toyota Motor Manufacturing France", "Toyota", kUnk, kNone, {}},

    // --- Italy -----------------------------------------------------------
    {"ZFA", "Fiat", "Fiat|Abarth", kUnk, kNone, {}},
    {"ZAR", "Alfa Romeo", "Alfa Romeo", kUnk, kNone, {}},
    {"ZAS", "Alfa Romeo", "Alfa Romeo", kUnk, kNone, {}},
    {"ZLA", "Lancia", "Lancia", kUnk, kNone, {}},
    {"ZFF", "Ferrari", "Ferrari", kUnk, kNone, {}},
    {"ZHW", "Lamborghini", "Lamborghini", kUnk, kNone, {}},
    {"ZPB", "Lamborghini (SUV)", "Lamborghini", kUnk, kNone, {}},
    {"ZAM", "Maserati", "Maserati", kUnk, kNone, {}},
    {"ZAC", "FCA Italy (Jeep, Dodge)", "Jeep|Dodge", kUnk, kNone, {}},
    {"ZCF", "Iveco", "Iveco", kUnk, kNone, {}},
    {"3C3", "FCA México", "Fiat|Chrysler", kStd, kNone, {}},
    {"9BD", "Fiat Automóveis (Brasilien)", "Fiat", kStd, kNone, {}},
    {"8AP", "Fiat Auto Argentina", "Fiat", kStd, kNone, {}},
    {"SUF", "Fiat Auto Poland", "Fiat", kUnk, kNone, {}},
    {"NM4", "Tofaş", "Fiat", kUnk, kNone, {}},

    // --- United Kingdom --------------------------------------------------
    {"SAJ", "Jaguar Cars", "Jaguar", kStd, kNone, {}},
    {"SAD", "Jaguar Land Rover (Jaguar SUV)", "Jaguar", kStd, kNone, {}},
    {"SAL", "Land Rover", "Land Rover", kStd, kNone, {}},
    {"SC6", "INEOS Automotive (SUV)", "INEOS", kStd, kNone, {}},
    {"SH7", "INEOS Automotive (pick-up)", "INEOS", kStd, kNone, {}},
    {"ZS1", "Leapmotor International", "Leapmotor", kStd, kNone, {}},
    {"SAR", "Rover Group / MG Rover", "Rover|MG", kUnk, kNone, {}},
    {"SAX", "Austin Rover Group", "Austin|Rover", kUnk, kNone, {}},
    {"SCA", "Rolls-Royce", "Rolls-Royce", kStd, kNone, {}},
    {"SCB", "Bentley", "Bentley", kStd, kNone, {}},
    {"SCC", "Lotus Cars", "Lotus", kStd, kNone, {}},
    {"SCE", "DeLorean Motor Company", "DeLorean", kStd, kNone, {}},
    {"SCF", "Aston Martin", "Aston Martin", kStd, kNone, {}},
    {"SBM", "McLaren Automotive", "McLaren", kStd, kNone, {}},
    {"SFZ", "Tesla Roadster, gebaut von Lotus", "Tesla", kStd, kNone, {}},
    {"SFA", "Ford Motor Company UK", "Ford", kUnk, kNone, {}},
    {"SHH", "Honda of the UK Manufacturing", "Honda", kUnk, kNone, {}},
    {"SHS", "Honda of the UK Manufacturing (SUV)", "Honda", kUnk, kNone, {}},
    {"SJN", "Nissan Motor Manufacturing UK", "Nissan", kUnk, kNone, {}},
    {"SJK", "Nissan Motor Manufacturing UK (Infiniti)", "Infiniti", kUnk, kNone, {}},
    {"SB1", "Toyota Motor Manufacturing UK", "Toyota", kUnk, kNone, {}},

    // --- Sweden, Finland, Netherlands -----------------------------------
    {"YV1", "Volvo Cars", "Volvo", kStd, kNone, {}},
    {"YV4", "Volvo Cars (SUV)", "Volvo", kStd, kNone, {}},
    {"7JR", "Volvo Car USA", "Volvo", kStd, kNone, {}},
    {"LVY", "Volvo Cars Daqing", "Volvo", kStd, kNone, {}},
    {"LYV", "Volvo Cars Chengdu / Luqiao", "Volvo|Polestar", kStd, kNone, {}},
    {"LPS", "Polestar", "Polestar", kStd, kNone, {}},
    {"YSM", "Polestar", "Polestar", kStd, kNone, {}},
    {"YSR", "Polestar (SUV)", "Polestar", kStd, kNone, {}},
    {"YS3", "Saab Automobile", "Saab", kStd, kNone, {}},
    {"YK1", "Saab, gebaut von Valmet Automotive", "Saab", kStd, kNone, {}},
    {"3G0", "Saab 9-4X (GM México)", "Saab", kStd, kNone, {}},
    {"YH4", "Fisker Automotive (Karma), gebaut von Valmet", "Fisker", kStd, kNone, {}},
    {"XMC", "NedCar (Mitsubishi)", "Mitsubishi", kUnk, kNone, {}},
    {"XLR", "DAF Trucks", "DAF", kUnk, kNone, {}},
    {"YAR", "Toyota Motor Europe", "Toyota", kUnk, kNone, {}},

    // --- Central and Eastern Europe, Russia, Turkey ---------------------
    {"TMA", "Hyundai Motor Manufacturing Czech", "Hyundai", kStd, kNone, {}},
    {"TMC", "Hyundai Motor Manufacturing Czech (SUV)", "Hyundai", kStd, kNone, {}},
    {"TMT", "Tatra", "Tatra", kUnk, kNone, {}},
    {"TSM", "Magyar Suzuki", "Suzuki", kUnk, kNone, {}},
    {"TW1", "Toyota Caetano Portugal", "Toyota", kUnk, kNone, {}},
    {"U5Y", "Kia Slovakia", "Kia", kStd, kNone, {}},
    {"U6Y", "Kia Slovakia (SUV)", "Kia", kStd, kNone, {}},
    {"UU1", "Dacia", "Dacia", kUnk, kNone, {}},
    {"UU6", "Daewoo Automobile Romania", "Daewoo", kUnk, kNone, {}},
    {"VSK", "Nissan Motor Ibérica", "Nissan", kUnk, kNone, {}},
    {"VSE", "Santana Motor (Suzuki)", "Suzuki", kUnk, kNone, {}},
    {"VS6", "Ford España", "Ford", kUnk, kNone, {}},
    {"VCF", "Fisker Ocean, gebaut von Magna Steyr", "Fisker", kStd, kNone, {}},
    {"SUP", "FSO / Daewoo-FSO", "Daewoo|FSO", kUnk, kNone, {}},
    {"XTA", "AvtoVAZ", "Lada", kUnk, kNone, {}},
    {"XTT", "UAZ", "UAZ", kUnk, kNone, {}},
    {"X9L", "GM-AvtoVAZ", "Chevrolet", kUnk, kNone, {}},
    {"X7L", "Renault Russia", "Renault", kUnk, kNone, {}},
    {"NM0", "Ford Otosan", "Ford", kStd, kNone, {}},
    {"NMT", "Toyota Motor Manufacturing Turkey", "Toyota", kUnk, kNone, {}},
    {"NLH", "Hyundai Assan", "Hyundai", kStd, kNone, {}},

    // --- Japan -----------------------------------------------------------
    {"JT", "Toyota Motor Corporation", "Toyota", kUnk, kNone, {}},
    {"JTH", "Toyota Motor Corporation (Lexus)", "Lexus", kUnk, kNone, {}},
    {"JTJ", "Toyota Motor Corporation (Lexus SUV)", "Lexus", kUnk, kNone, {}},
    {"JTK", "Toyota Motor Corporation (Scion)", "Scion", kStd, kNone, {}},
    {"JTL", "Toyota Motor Corporation (Scion)", "Scion", kStd, kNone, {}},
    {"JHM", "Honda Motor Co.", "Honda", kUnk, kNone, {}},
    {"JHL", "Honda Motor Co. (SUV)", "Honda", kUnk, kNone, {}},
    {"JHG", "Honda Motor Co.", "Honda", kUnk, kNone, {}},
    {"JH4", "Honda Motor Co. (Acura)", "Acura", kStd, kNone, {}},
    {"JN", "Nissan Motor Co.", "Nissan", kUnk, kNone, {}},
    {"JNK", "Nissan Motor Co. (Infiniti)", "Infiniti", kUnk, kNone, {}},
    {"JNR", "Nissan Motor Co. (Infiniti SUV)", "Infiniti", kUnk, kNone, {}},
    {"JM1", "Mazda Motor Corporation", "Mazda", kUnk, kNone, {}},
    {"JM3", "Mazda Motor Corporation (SUV)", "Mazda", kUnk, kNone, {}},
    {"JMZ", "Mazda Motor Corporation (Europa)", "Mazda", kUnk, kNone, {}},
    {"JA3", "Mitsubishi Motors", "Mitsubishi", kUnk, kNone, {}},
    {"JA4", "Mitsubishi Motors (SUV)", "Mitsubishi", kUnk, kNone, {}},
    {"JMB", "Mitsubishi Motors (Europa)", "Mitsubishi", kUnk, kNone, {}},
    {"JMY", "Mitsubishi Motors", "Mitsubishi", kUnk, kNone, {}},
    {"JF1", "Subaru (Fuji Heavy Industries)", "Subaru", kUnk, kNone, {}},
    {"JF2", "Subaru (Fuji Heavy Industries, SUV)", "Subaru", kUnk, kNone, {}},
    {"JS2", "Suzuki Motor Corporation", "Suzuki", kUnk, kNone, {}},
    {"JS3", "Suzuki Motor Corporation (SUV)", "Suzuki", kUnk, kNone, {}},
    {"JSA", "Suzuki Motor Corporation (Europa)", "Suzuki", kUnk, kNone, {}},
    {"JDA", "Daihatsu Motor Co.", "Daihatsu", kUnk, kNone, {}},
    {"JAA", "Isuzu Motors", "Isuzu", kUnk, kNone, {}},
    {"JAC", "Isuzu Motors (SUV)", "Isuzu", kUnk, kNone, {}},

    // --- Korea -----------------------------------------------------------
    {"KMH", "Hyundai Motor Company", "Hyundai", kStd, kNone, {}},
    {"KM8", "Hyundai Motor Company (SUV)", "Hyundai", kStd, kNone, {}},
    {"KMF", "Hyundai Motor Company (Transporter)", "Hyundai", kStd, kNone, {}},
    {"KMT", "Genesis Motor", "Genesis", kStd, kNone, {}},
    {"KMU", "Genesis Motor (SUV)", "Genesis", kStd, kNone, {}},
    {"KNA", "Kia Corporation", "Kia", kStd, kNone, {}},
    {"KND", "Kia Corporation (SUV)", "Kia", kStd, kNone, {}},
    {"KNE", "Kia Corporation (Europa)", "Kia", kStd, kNone, {}},
    {"KLA", "Daewoo Motor", "Daewoo", kUnk, kNone, {}},
    {"KL1", "GM Daewoo / GM Korea", "Chevrolet|Daewoo", kStd, kNone, {}},
    {"KL4", "GM Korea (Buick)", "Buick", kStd, kNone, {}},
    {"KL7", "GM Korea (Chevrolet)", "Chevrolet", kStd, kNone, {}},
    {"KL8", "GM Korea (Chevrolet)", "Chevrolet", kStd, kNone, {}},
    {"KPT", "SsangYong Motor", "SsangYong", kUnk, kNone, {}},
    {"KPA", "SsangYong Motor (Pick-up)", "SsangYong", kUnk, kNone, {}},
    {"KNM", "Renault Samsung Motors", "Renault|Nissan", kStd, kNone, {}},

    // --- China, Vietnam, India, South East Asia -------------------------
    {"LRW", "Tesla Gigafactory Shanghai", "Tesla", kStd, kTesla, {}},
    {"LSJ", "SAIC Motor (MG, Roewe)", "MG", kStd, kNone, {}},
    {"LSG", "SAIC-GM", "Buick|Chevrolet|Cadillac", kStd, kNone, {}},
    {"LGX", "BYD Auto", "BYD", kStd, kNone, {}},
    {"LC0", "BYD Auto", "BYD", kStd, kNone, {}},
    {"LPE", "BYD Auto", "BYD", kStd, kNone, {}},
    {"LA9", "BYD Auto", "BYD", kStd, kNone, {}},
    {"L6T", "Zhejiang Geely (Geely, Zeekr, Lynk & Co)", "Geely|Zeekr|Lynk & Co", kStd, kNone, {}},
    {"LB3", "Geely Automobile", "Geely", kStd, kNone, {}},
    {"HJN", "NIO", "NIO", kStd, kNone, {}},
    {"LGW", "Great Wall Motor (GWM, Haval, ORA, Tank, WEY)", "Great Wall|ORA", kStd, kNone, {}},
    {"LVV", "Chery Automobile (Omoda, Jaecoo)", "Chery|Omoda|Jaecoo", kStd, kNone, {}},
    {"PRB", "Chery (Jaecoo)", "Jaecoo", kStd, kNone, {}},
    {"LVX", "Aiways Automobiles", "Aiways", kStd, kNone, {}},
    {"SDB", "Talbot", "Talbot", kUnk, kNone, {}},
    {"VF4", "Talbot", "Talbot", kUnk, kNone, {}},
    {"VSC", "Talbot", "Talbot", kUnk, kNone, {}},
    {"SNT", "Trabant (VEB Sachsenring)", "Trabant", kUnk, kNone, {}},
    {"SNE", "Wartburg (Automobilwerk Eisenach)", "Wartburg", kUnk, kNone, {}},
    {"LNN", "Chery Automobile (Omoda, Jaecoo)", "Chery|Omoda|Jaecoo", kStd, kNone, {}},
    {"LVU", "Chery Automobile (Jetour, Omoda, Jaecoo)", "Chery|Omoda|Jaecoo", kStd, kNone, {}},
    {"L1N", "XPeng Motors", "Xpeng", kStd, kNone, {}},
    {"LMV", "XPeng Motors (G3, made by Haima)", "Xpeng", kStd, kNone, {}},
    {"LJ1", "NIO", "NIO", kStd, kNone, {}},
    {"HLX", "Li Auto", "Li Auto", kStd, kNone, {}},
    {"LW4", "Li Auto", "Li Auto", kStd, kNone, {}},
    {"HXM", "Xiaomi Auto", "Xiaomi", kStd, kNone, {}},
    {"LFZ", "Leapmotor", "Leapmotor", kStd, kNone, {}},
    {"LFB", "FAW Group (Hongqi, Bestune)", "Hongqi", kStd, kNone, {}},
    {"LFP", "FAW Car (Hongqi, Bestune)", "Hongqi", kStd, kNone, {}},
    {"LSF", "SAIC Maxus", "Maxus", kStd, kNone, {}},
    {"LSH", "SAIC Maxus", "Maxus", kStd, kNone, {}},
    {"LSK", "SAIC Maxus", "Maxus", kStd, kNone, {}},
    {"LM8", "Seres (AITO)", "Aito", kStd, kNone, {}},
    {"LDP", "Dongfeng (Voyah)", "Voyah", kStd, kNone, {}},
    {"LS6", "Changan (Deepal, Avatr)", "Avatr", kStd, kNone, {}},
    {"LK6", "SAIC-GM-Wuling", "Wuling", kStd, kNone, {}},
    {"LPA", "Changan PSA (DS)", "DS", kStd, kNone, {}},
    {"LBE", "Beijing Hyundai", "Hyundai", kStd, kNone, {}},
    {"LFM", "FAW Toyota", "Toyota", kStd, kNone, {}},
    {"LHG", "GAC Honda", "Honda", kStd, kNone, {}},
    {"LVH", "Dongfeng Honda", "Honda", kStd, kNone, {}},
    {"LDC", "Dongfeng Peugeot Citroën", "Peugeot|Citroën", kStd, kNone, {}},
    {"LGB", "Dongfeng Nissan", "Nissan", kStd, kNone, {}},
    {"LZW", "SAIC-GM-Wuling", "Wuling", kStd, kNone, {}},
    {"RLL", "VinFast", "VinFast", kStd, kNone, {}},
    {"RLN", "VinFast", "VinFast", kStd, kNone, {}},
    {"MA1", "Mahindra & Mahindra", "Mahindra", kUnk, kNone, {}},
    {"MA3", "Maruti Suzuki", "Suzuki", kUnk, kNone, {}},
    {"MAT", "Tata Motors", "Tata", kUnk, kNone, {}},
    {"MAL", "Hyundai Motor India", "Hyundai", kStd, kNone, {}},
    {"MAJ", "Ford India", "Ford", kUnk, kNone, {}},
    {"MR0", "Toyota Motor Thailand", "Toyota", kUnk, kNone, {}},
    {"MMB", "Mitsubishi Motors Thailand", "Mitsubishi", kUnk, kNone, {}},
    {"MPA", "Isuzu Motors Thailand", "Isuzu", kUnk, kNone, {}},
    {"MRX", "BYD Auto Thailand", "BYD", kStd, kNone, {}},
    {"PL1", "Proton", "Proton", kUnk, kNone, {}},

    // --- USA: General Motors --------------------------------------------
    {"1G", "General Motors USA", "Chevrolet|GMC|Buick|Cadillac|Pontiac|Oldsmobile|Saturn", kStd, kNone, {}},
    {"1G1", "General Motors (Chevrolet)", "Chevrolet", kStd, kNone, {}},
    {"1GC", "General Motors (Chevrolet Truck)", "Chevrolet", kStd, kNone, {}},
    {"1GN", "General Motors (Chevrolet SUV)", "Chevrolet", kStd, kNone, {}},
    {"1GB", "General Motors (Chevrolet Nutzfahrzeug)", "Chevrolet", kStd, kNone, {}},
    {"1G2", "General Motors (Pontiac)", "Pontiac", kStd, kNone, {}},
    {"1GM", "General Motors (Pontiac MPV)", "Pontiac", kStd, kNone, {}},
    {"1G3", "General Motors (Oldsmobile)", "Oldsmobile", kStd, kNone, {}},
    {"1G4", "General Motors (Buick)", "Buick", kStd, kNone, {}},
    {"1G6", "General Motors (Cadillac)", "Cadillac", kStd, kNone, {}},
    {"1GY", "General Motors (Cadillac SUV)", "Cadillac", kStd, kNone, {}},
    {"1G8", "General Motors (Saturn)", "Saturn", kStd, kNone, {}},
    {"1GT", "General Motors (GMC Truck)", "GMC", kStd, kNone, {}},
    {"1GK", "General Motors (GMC SUV)", "GMC", kStd, kNone, {}},
    {"1GD", "General Motors (GMC Nutzfahrzeug)", "GMC", kStd, kNone, {}},
    {"2G", "General Motors of Canada", "Chevrolet|Pontiac|Buick|GMC", kStd, kNone, {}},
    {"2G1", "General Motors of Canada (Chevrolet)", "Chevrolet", kStd, kNone, {}},
    {"2G2", "General Motors of Canada (Pontiac)", "Pontiac", kStd, kNone, {}},
    {"2G4", "General Motors of Canada (Buick)", "Buick", kStd, kNone, {}},
    {"2GN", "General Motors of Canada (Chevrolet SUV)", "Chevrolet", kStd, kNone, {}},
    {"2GK", "General Motors of Canada (GMC SUV)", "GMC", kStd, kNone, {}},
    {"2CN", "CAMI Automotive (Chevrolet)", "Chevrolet", kStd, kNone, {}},
    {"3G", "General Motors de México", "Chevrolet|GMC|Buick|Cadillac|Pontiac", kStd, kNone, {}},
    {"3G1", "General Motors de México (Chevrolet)", "Chevrolet", kStd, kNone, {}},
    {"3GN", "General Motors de México (Chevrolet SUV)", "Chevrolet", kStd, kNone, {}},
    {"3G2", "General Motors de México (Pontiac)", "Pontiac", kStd, kNone, {}},
    {"3G7", "General Motors de México (Pontiac SUV)", "Pontiac", kStd, kNone, {}},
    {"3GS", "General Motors de México (Saturn SUV)", "Saturn", kStd, kNone, {}},
    {"5GA", "General Motors (Buick SUV)", "Buick", kStd, kNone, {}},
    {"5GR", "General Motors (Hummer H2)", "Hummer", kStd, kNone, {}},
    {"5GT", "General Motors (Hummer H3)", "Hummer", kStd, kNone, {}},
    {"5GN", "General Motors (Hummer H3T)", "Hummer", kStd, kNone, {}},
    {"5GZ", "General Motors (Saturn SUV)", "Saturn", kStd, kNone, {}},
    {"5Y2", "NUMMI (Pontiac Vibe)", "Pontiac", kStd, kNone, {}},
    {"137", "AM General (Hummer H1)", "Hummer", kStd, kNone, {}},
    {"6G1", "General Motors-Holden", "Holden|Chevrolet", kStd, kNone, {}},
    {"6G2", "General Motors-Holden (Pontiac)", "Pontiac", kStd, kNone, {}},
    {"6H8", "General Motors-Holden", "Holden", kUnk, kNone, {}},
    {"9BG", "General Motors do Brasil", "Chevrolet", kStd, kNone, {}},
    {"8AG", "General Motors de Argentina", "Chevrolet", kStd, kNone, {}},
    {"KL", "GM Daewoo / GM Korea", "Chevrolet|Daewoo", kStd, kNone, {}},

    // --- USA: Ford --------------------------------------------------------
    {"1F", "Ford Motor Company", "Ford", kStd, kNone, {}},
    {"1FA", "Ford Motor Company (Pkw)", "Ford", kStd, kNone, {}},
    {"1FM", "Ford Motor Company (SUV, MPV)", "Ford", kStd, kNone, {}},
    {"1FT", "Ford Motor Company (Truck)", "Ford", kStd, kNone, {}},
    {"1FD", "Ford Motor Company (Nutzfahrzeug)", "Ford", kStd, kNone, {}},
    {"1FU", "Freightliner", "Freightliner", kStd, kNone, {}},
    {"1FV", "Freightliner", "Freightliner", kStd, kNone, {}},
    {"1ZV", "AutoAlliance International (Ford)", "Ford", kStd, kNone, {}},
    {"2F", "Ford Motor Company of Canada", "Ford", kStd, kNone, {}},
    {"3F", "Ford de México", "Ford", kStd, kNone, {}},
    {"1L", "Ford Motor Company (Lincoln)", "Lincoln", kStd, kNone, {}},
    {"5L", "Ford Motor Company (Lincoln SUV)", "Lincoln", kStd, kNone, {}},
    {"2LM", "Ford Motor Company of Canada (Lincoln)", "Lincoln", kStd, kNone, {}},
    {"1ME", "Ford Motor Company (Mercury)", "Mercury", kStd, kNone, {}},
    {"2ME", "Ford Motor Company of Canada (Mercury)", "Mercury", kStd, kNone, {}},
    {"4M2", "Ford Motor Company (Mercury SUV)", "Mercury", kStd, kNone, {}},
    {"9BF", "Ford do Brasil", "Ford", kStd, kNone, {}},
    {"8AF", "Ford Argentina", "Ford", kStd, kNone, {}},
    {"6FP", "Ford Australia", "Ford", kUnk, kNone, {}},
    {"AFA", "Ford South Africa", "Ford", kUnk, kNone, {}},

    // --- USA: Chrysler, Dodge, Jeep, Plymouth, Ram ----------------------
    {"1C3", "Chrysler (Pkw)", "Chrysler|Dodge", kStd, kNone, {}},
    {"1C4", "Chrysler (MPV, SUV)", "Jeep|Chrysler|Dodge", kStd, kNone, {}},
    {"1C6", "Chrysler (Ram Truck)", "Ram|Dodge", kStd, kNone, {}},
    {"1C8", "Chrysler (MPV)", "Chrysler", kStd, kNone, {}},
    {"1B3", "Dodge (Pkw)", "Dodge", kStd, kNone, {}},
    {"1B4", "Dodge (MPV)", "Dodge", kStd, kNone, {}},
    {"1B7", "Dodge (Truck)", "Dodge", kStd, kNone, {}},
    {"1D3", "Dodge (Ram Truck)", "Dodge|Ram", kStd, kNone, {}},
    {"1D4", "Dodge (SUV)", "Dodge", kStd, kNone, {}},
    {"1D7", "Dodge (Ram Truck)", "Dodge|Ram", kStd, kNone, {}},
    {"1J4", "Jeep (SUV)", "Jeep", kStd, kNone, {}},
    {"1J8", "Jeep (SUV)", "Jeep", kStd, kNone, {}},
    {"1P3", "Plymouth (Pkw)", "Plymouth", kStd, kNone, {}},
    {"1P4", "Plymouth (MPV)", "Plymouth", kStd, kNone, {}},
    {"2B3", "Chrysler Canada (Dodge)", "Dodge", kStd, kNone, {}},
    {"2B4", "Chrysler Canada (Dodge MPV)", "Dodge", kStd, kNone, {}},
    {"2C3", "Chrysler Canada (Pkw)", "Chrysler|Dodge", kStd, kNone, {}},
    {"2C4", "Chrysler Canada (MPV)", "Chrysler|Dodge", kStd, kNone, {}},
    {"2P4", "Chrysler Canada (Plymouth MPV)", "Plymouth", kStd, kNone, {}},
    {"2A4", "Chrysler Canada (MPV)", "Chrysler", kStd, kNone, {}},
    {"3C4", "Chrysler de México (MPV)", "Chrysler|Dodge|Jeep", kStd, kNone, {}},
    {"3C6", "Chrysler de México (Ram)", "Ram", kStd, kNone, {}},
    {"3D7", "Chrysler de México (Ram)", "Dodge|Ram", kStd, kNone, {}},

    // --- USA, Canada, Mexico: other makers ------------------------------
    {"1HG", "Honda of America", "Honda", kStd, kNone, {}},
    {"19X", "Honda Manufacturing of Indiana", "Honda", kStd, kNone, {}},
    {"19U", "Honda of America (Acura)", "Acura", kStd, kNone, {}},
    {"2HG", "Honda of Canada", "Honda", kStd, kNone, {}},
    {"2HK", "Honda of Canada (SUV)", "Honda", kStd, kNone, {}},
    {"2HN", "Honda of Canada (Acura SUV)", "Acura", kStd, kNone, {}},
    {"3HG", "Honda de México", "Honda", kStd, kNone, {}},
    {"5FN", "Honda Manufacturing of Alabama", "Honda", kStd, kNone, {}},
    {"5J6", "Honda of America (SUV)", "Honda", kStd, kNone, {}},
    {"5J8", "Honda of America (Acura SUV)", "Acura", kStd, kNone, {}},
    {"7FA", "Honda Manufacturing of Indiana (SUV)", "Honda", kStd, kNone, {}},
    {"4T1", "Toyota Motor Manufacturing Kentucky", "Toyota", kStd, kNone, {}},
    {"4T3", "Toyota Motor Manufacturing (SUV)", "Toyota", kStd, kNone, {}},
    {"4T4", "Toyota Motor Manufacturing", "Toyota", kStd, kNone, {}},
    {"5TD", "Toyota Motor Manufacturing Indiana", "Toyota", kStd, kNone, {}},
    {"5TF", "Toyota Motor Manufacturing Texas", "Toyota", kStd, kNone, {}},
    {"5TB", "Toyota Motor Manufacturing (Truck)", "Toyota", kStd, kNone, {}},
    {"5YF", "Toyota Motor Manufacturing Mississippi", "Toyota", kStd, kNone, {}},
    {"1NX", "NUMMI (Toyota)", "Toyota", kStd, kNone, {}},
    {"2T1", "Toyota Motor Manufacturing Canada", "Toyota", kStd, kNone, {}},
    {"2T2", "Toyota Motor Manufacturing Canada (Lexus)", "Lexus", kStd, kNone, {}},
    {"2T3", "Toyota Motor Manufacturing Canada (SUV)", "Toyota", kStd, kNone, {}},
    {"3TM", "Toyota Motor Manufacturing de Baja California", "Toyota", kStd, kNone, {}},
    {"58A", "Toyota Motor Manufacturing Kentucky (Lexus)", "Lexus", kStd, kNone, {}},
    {"7MU", "Mazda Toyota Manufacturing (Toyota)", "Toyota", kStd, kNone, {}},
    {"7MM", "Mazda Toyota Manufacturing (Mazda)", "Mazda", kStd, kNone, {}},
    {"1YV", "Mazda (AutoAlliance)", "Mazda", kStd, kNone, {}},
    {"4F2", "Mazda (gebaut von Ford, SUV)", "Mazda", kStd, kNone, {}},
    {"4F4", "Mazda (gebaut von Ford, Truck)", "Mazda", kStd, kNone, {}},
    {"3MZ", "Mazda de México", "Mazda", kStd, kNone, {}},
    {"3MD", "Mazda de México", "Mazda", kStd, kNone, {}},
    {"3MV", "Mazda de México (SUV)", "Mazda", kStd, kNone, {}},
    {"1N4", "Nissan North America (Pkw)", "Nissan", kStd, kNone, {}},
    {"1N6", "Nissan North America (Truck)", "Nissan", kStd, kNone, {}},
    {"5N1", "Nissan North America (SUV)", "Nissan|Infiniti", kStd, kNone, {}},
    {"3N1", "Nissan Mexicana", "Nissan", kStd, kNone, {}},
    {"3PC", "Nissan Mexicana (Infiniti)", "Infiniti", kStd, kNone, {}},
    {"4A3", "Mitsubishi Motors North America", "Mitsubishi", kStd, kNone, {}},
    {"4A4", "Mitsubishi Motors North America (SUV)", "Mitsubishi", kStd, kNone, {}},
    {"4S3", "Subaru of Indiana", "Subaru", kStd, kNone, {}},
    {"4S4", "Subaru of Indiana (SUV)", "Subaru", kStd, kNone, {}},
    {"2S3", "CAMI Automotive (Suzuki)", "Suzuki", kStd, kNone, {}},
    {"5NP", "Hyundai Motor Manufacturing Alabama", "Hyundai", kStd, kNone, {}},
    {"5NM", "Hyundai Motor Manufacturing Alabama (SUV)", "Hyundai", kStd, kNone, {}},
    {"5XX", "Kia Motors Manufacturing Georgia", "Kia", kStd, kNone, {}},
    {"5XY", "Kia Motors Manufacturing Georgia (SUV)", "Kia", kStd, kNone, {}},
    {"3KP", "Kia Motors México", "Kia", kStd, kNone, {}},
    {"5YJ", "Tesla", "Tesla", kStd, kTesla, {}},
    {"7SA", "Tesla", "Tesla", kStd, kTesla, {}},
    {"7G2", "Tesla (Cybertruck, Semi)", "Tesla", kStd, kNone, {}},
    {"7PD", "Rivian Automotive", "Rivian", kStd, kNone, {}},
    {"50E", "Lucid Motors", "Lucid", kStd, kNone, {}},
    {"1M8", "Motor Coach Industries", "MCI", kStd, kNone, {}},

    // --- South America, Africa, Australia: other makers -----------------
    {"9BR", "Toyota do Brasil", "Toyota", kStd, kNone, {}},
    {"8AJ", "Toyota Argentina", "Toyota", kStd, kNone, {}},
    {"93H", "Honda Automóveis do Brasil", "Honda", kStd, kNone, {}},
    {"93Y", "Renault do Brasil", "Renault", kStd, kNone, {}},
    {"8A1", "Renault Argentina", "Renault", kStd, kNone, {}},
    {"936", "Peugeot Citroën do Brasil (Peugeot)", "Peugeot", kStd, kNone, {}},
    {"935", "Peugeot Citroën do Brasil (Citroën)", "Citroën", kStd, kNone, {}},
    {"8AD", "Peugeot Citroën Argentina (Peugeot)", "Peugeot", kStd, kNone, {}},
    {"94D", "Nissan do Brasil", "Nissan", kStd, kNone, {}},
    {"9BH", "Hyundai Motor Brasil", "Hyundai", kStd, kNone, {}},
    {"AHT", "Toyota South Africa", "Toyota", kUnk, kNone, {}},
    {"6T1", "Toyota Motor Corporation Australia", "Toyota", kUnk, kNone, {}},
    {"6MM", "Mitsubishi Motors Australia", "Mitsubishi", kUnk, kNone, {}},
};

constexpr std::size_t kWmiCount = sizeof(kWmi) / sizeof(kWmi[0]);

// ISO 3780 sorts the second character in this order; ranges like "AA-AH"
// are meant along it.
constexpr const char* kRangeOrder = "ABCDEFGHJKLMNPRSTUVWXYZ1234567890";

int rangeIndex(char c) {
    const char* p = std::strchr(kRangeOrder, c);
    return p && c ? static_cast<int>(p - kRangeOrder) : -1;
}

struct CountryRange {
    char first;
    char from;
    char to;
    Text name;
};

const CountryRange kCountries[] = {
    {'A', 'A', 'H', {"Südafrika", "South Africa"}},
    {'A', 'J', 'N', {"Elfenbeinküste", "Ivory Coast"}},
    {'C', 'L', 'R', {"Tunesien", "Tunisia"}},
    {'D', 'A', 'E', {"Ägypten", "Egypt"}},
    {'D', 'F', 'K', {"Marokko", "Morocco"}},
    {'F', 'F', 'K', {"Nigeria", "Nigeria"}},
    {'H', 'A', '0', {"China", "China"}},
    {'J', 'A', '0', {"Japan", "Japan"}},
    {'K', 'A', 'E', {"Sri Lanka", "Sri Lanka"}},
    {'K', 'F', 'K', {"Israel", "Israel"}},
    {'K', 'L', 'R', {"Südkorea", "South Korea"}},
    {'K', 'S', '0', {"Kasachstan", "Kazakhstan"}},
    {'L', 'A', '0', {"China", "China"}},
    {'M', 'A', 'E', {"Indien", "India"}},
    {'M', 'F', 'K', {"Indonesien", "Indonesia"}},
    {'M', 'L', 'R', {"Thailand", "Thailand"}},
    {'N', 'A', 'E', {"Iran", "Iran"}},
    {'N', 'F', 'K', {"Pakistan", "Pakistan"}},
    {'N', 'L', 'R', {"Türkei", "Turkey"}},
    {'P', 'A', 'E', {"Philippinen", "Philippines"}},
    {'P', 'F', 'K', {"Singapur", "Singapore"}},
    {'P', 'L', 'R', {"Malaysia", "Malaysia"}},
    {'R', 'A', 'E', {"Vereinigte Arabische Emirate", "United Arab Emirates"}},
    {'R', 'F', 'K', {"Taiwan", "Taiwan"}},
    {'R', 'L', 'R', {"Vietnam", "Vietnam"}},
    {'R', 'S', '0', {"Saudi-Arabien", "Saudi Arabia"}},
    {'S', 'A', 'M', {"Vereinigtes Königreich", "United Kingdom"}},
    {'S', 'N', 'T', {"Deutschland (ehemals DDR)", "Germany (former East Germany)"}},
    {'S', 'U', 'Z', {"Polen", "Poland"}},
    {'S', '1', '4', {"Lettland", "Latvia"}},
    {'T', 'A', 'H', {"Schweiz", "Switzerland"}},
    {'T', 'J', 'P', {"Tschechien", "Czech Republic"}},
    {'T', 'R', 'V', {"Ungarn", "Hungary"}},
    {'T', 'W', '1', {"Portugal", "Portugal"}},
    {'U', 'H', 'M', {"Dänemark", "Denmark"}},
    {'U', 'N', 'T', {"Irland", "Ireland"}},
    {'U', 'U', 'X', {"Rumänien", "Romania"}},
    {'U', '5', '7', {"Slowakei", "Slovakia"}},
    {'V', 'A', 'E', {"Österreich", "Austria"}},
    {'V', 'F', 'R', {"Frankreich", "France"}},
    {'V', 'S', 'W', {"Spanien", "Spain"}},
    {'V', 'X', '2', {"Frankreich oder Serbien", "France or Serbia"}},
    {'V', '3', '5', {"Kroatien", "Croatia"}},
    {'V', '6', '0', {"Estland", "Estonia"}},
    {'W', 'A', '0', {"Deutschland", "Germany"}},
    {'X', 'A', 'E', {"Bulgarien", "Bulgaria"}},
    {'X', 'F', 'K', {"Griechenland", "Greece"}},
    {'X', 'L', 'R', {"Niederlande", "Netherlands"}},
    {'X', 'S', 'W', {"Russland", "Russia"}},
    {'X', 'X', '2', {"Luxemburg", "Luxembourg"}},
    {'X', '3', '0', {"Russland", "Russia"}},
    {'Y', 'A', 'E', {"Belgien", "Belgium"}},
    {'Y', 'F', 'K', {"Finnland", "Finland"}},
    {'Y', 'S', 'W', {"Schweden", "Sweden"}},
    {'Y', 'X', '2', {"Norwegen", "Norway"}},
    {'Y', '3', '5', {"Belarus", "Belarus"}},
    {'Y', '6', '0', {"Ukraine", "Ukraine"}},
    {'Z', 'A', 'R', {"Italien", "Italy"}},
    {'Z', 'X', '2', {"Slowenien", "Slovenia"}},
    {'Z', '3', '5', {"Litauen", "Lithuania"}},
    {'Z', '6', '0', {"Russland", "Russia"}},
    {'1', 'A', '0', {"USA", "USA"}},
    {'2', 'A', '0', {"Kanada", "Canada"}},
    {'3', 'A', 'W', {"Mexiko", "Mexico"}},
    {'3', 'X', '7', {"Costa Rica", "Costa Rica"}},
    {'4', 'A', '0', {"USA", "USA"}},
    {'5', 'A', '0', {"USA", "USA"}},
    {'6', 'A', 'W', {"Australien", "Australia"}},
    {'7', 'A', 'E', {"Neuseeland", "New Zealand"}},
    {'7', 'F', '0', {"USA", "USA"}},
    {'8', 'A', 'E', {"Argentinien", "Argentina"}},
    {'8', 'F', 'K', {"Chile", "Chile"}},
    {'8', 'L', 'R', {"Ecuador", "Ecuador"}},
    {'8', 'S', 'W', {"Peru", "Peru"}},
    {'8', 'X', '2', {"Venezuela", "Venezuela"}},
    {'9', 'A', 'E', {"Brasilien", "Brazil"}},
    {'9', 'F', 'J', {"Kolumbien", "Colombia"}},
    {'9', 'S', 'V', {"Uruguay", "Uruguay"}},
    {'9', '3', '9', {"Brasilien", "Brazil"}},
};

// Values of the characters for the check digit (49 CFR 565.15).
int transliterate(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    switch (c) {
        case 'A': case 'J': return 1;
        case 'B': case 'K': case 'S': return 2;
        case 'C': case 'L': case 'T': return 3;
        case 'D': case 'M': case 'U': return 4;
        case 'E': case 'N': case 'V': return 5;
        case 'F': case 'W': return 6;
        case 'G': case 'P': case 'X': return 7;
        case 'H': case 'Y': return 8;
        case 'R': case 'Z': return 9;
        default: return -1;   // I, O, Q and everything else
    }
}

constexpr int kWeights[17] = {8, 7, 6, 5, 4, 3, 2, 10, 0, 9, 8, 7, 6, 5, 4, 3, 2};

int systemYear() {
    const std::time_t now = std::time(nullptr);
    std::tm local{};
#ifdef _WIN32
    localtime_s(&local, &now);
#else
    localtime_r(&now, &local);
#endif
    return local.tm_year + 1900;
}

std::vector<std::string> splitBrands(const char* list) {
    std::vector<std::string> out;
    std::string current;
    for (const char* p = list; *p; ++p) {
        if (*p == '|') {
            if (!current.empty()) out.push_back(current);
            current.clear();
        } else {
            current += *p;
        }
    }
    if (!current.empty()) out.push_back(current);
    return out;
}

bool isDigit(char c) { return c >= '0' && c <= '9'; }

}  // namespace

const WmiEntry* wmiTable(std::size_t& count) {
    count = kWmiCount;
    return kWmi;
}

const WmiEntry* findWmi(std::string_view text) {
    if (text.size() < 2) return nullptr;
    char key[4] = {0, 0, 0, 0};
    for (std::size_t i = 0; i < 3 && i < text.size(); ++i) {
        key[i] = static_cast<char>(std::toupper(static_cast<unsigned char>(text[i])));
    }
    const WmiEntry* prefix = nullptr;
    for (const WmiEntry& e : kWmi) {
        const std::size_t len = std::strlen(e.wmi);
        if (len == 3 && key[2] && std::strncmp(e.wmi, key, 3) == 0) return &e;
        if (len == 2 && std::strncmp(e.wmi, key, 2) == 0) prefix = &e;
    }
    return prefix;
}

std::string vinCountry(std::string_view vin) {
    if (vin.size() < 2) return {};
    const char first = static_cast<char>(std::toupper(static_cast<unsigned char>(vin[0])));
    const int second = rangeIndex(static_cast<char>(std::toupper(static_cast<unsigned char>(vin[1]))));
    if (second < 0) return {};
    for (const CountryRange& r : kCountries) {
        if (r.first != first) continue;
        if (second >= rangeIndex(r.from) && second <= rangeIndex(r.to)) return r.name.get();
    }
    return L("nicht zugeordnet", "not assigned");
}

std::string vinRegion(std::string_view vin) {
    if (vin.empty()) return {};
    const char c = static_cast<char>(std::toupper(static_cast<unsigned char>(vin[0])));
    if (c >= 'A' && c <= 'H') return L("Afrika", "Africa");
    if (c >= 'J' && c <= 'R') return L("Asien", "Asia");
    if (c >= 'S' && c <= 'Z') return L("Europa", "Europe");
    if (c >= '1' && c <= '5') return L("Nordamerika", "North America");
    if (c == '6') return L("Ozeanien", "Oceania");
    if (c == '7') {
        // 7A to 7E is New Zealand, the rest of the range went to the USA.
        const char s = vin.size() > 1
            ? static_cast<char>(std::toupper(static_cast<unsigned char>(vin[1]))) : 0;
        return (s >= 'A' && s <= 'E') ? L("Ozeanien", "Oceania") : L("Nordamerika", "North America");
    }
    if (c == '8' || c == '9') return L("Südamerika", "South America");
    return L("unbekannt", "unknown");
}

int vinBaseYear(char code) {
    static constexpr const char* kCodes = "ABCDEFGHJKLMNPRSTVWXY123456789";
    if (!code) return 0;
    const char* p = std::strchr(kCodes, code);
    return p ? 1980 + static_cast<int>(p - kCodes) : 0;
}

char vinCheckDigit(std::string_view vin) {
    if (vin.size() != 17) return 0;
    int sum = 0;
    for (std::size_t i = 0; i < 17; ++i) {
        const int v = transliterate(static_cast<char>(std::toupper(static_cast<unsigned char>(vin[i]))));
        if (v < 0) return 0;
        sum += v * kWeights[i];
    }
    const int rest = sum % 11;
    return rest == 10 ? 'X' : static_cast<char>('0' + rest);
}

VinInfo decodeVin(std::string_view input, int currentYear) {
    VinInfo info;
    if (currentYear <= 0) currentYear = systemYear();

    // Normalise: upper case, drop the separators people type or copy along.
    bool badChars = false;
    bool ioq = false;
    for (char raw : input) {
        const char c = static_cast<char>(std::toupper(static_cast<unsigned char>(raw)));
        if (c == ' ' || c == '-' || c == '.' || c == '\t' || c == '*' || c == '\r' || c == '\n') continue;
        if (c == 'I' || c == 'O' || c == 'Q') ioq = true;
        else if (!std::isalnum(static_cast<unsigned char>(c))) badChars = true;
        info.vin += c;
    }

    if (info.vin.empty()) {
        info.problems.push_back(L("Keine Fahrgestellnummer angegeben.", "No VIN given."));
        return info;
    }
    if (info.vin.size() != 17) {
        info.problems.push_back(info.vin.size() < 17
            ? L("Zu kurz: eine Fahrgestellnummer hat 17 Zeichen. Autos vor 1981 hatten oft kürzere Nummern, "
                "die sich nicht nach dieser Norm deuten lassen.",
                "Too short: a VIN has 17 characters. Cars built before 1981 often had shorter numbers "
                "that cannot be decoded after this standard.")
            : L("Zu lang: eine Fahrgestellnummer hat 17 Zeichen.",
                "Too long: a VIN has 17 characters."));
    }
    if (ioq) {
        info.problems.push_back(L("Enthält I, O oder Q. Diese Buchstaben kommen in Fahrgestellnummern nicht vor, "
                                  "gemeint ist wohl 1 oder 0.",
                                  "Contains I, O or Q. These letters never appear in a VIN, "
                                  "probably 1 or 0 is meant."));
    }
    if (badChars) {
        info.problems.push_back(L("Enthält Zeichen, die keine Buchstaben oder Ziffern sind.",
                                  "Contains characters that are neither letters nor digits."));
    }
    info.valid = info.vin.size() == 17 && !ioq && !badChars;

    // Manufacturer and origin work with the first three characters alone.
    if (info.vin.size() >= 3) info.wmi = info.vin.substr(0, 3);
    info.country = vinCountry(info.vin);
    info.region = vinRegion(info.vin);
    const char first = info.vin[0];
    const char second = info.vin.size() > 1 ? info.vin[1] : 0;
    const bool chinese = first == 'L' || first == 'H';
    info.northAmerica = (first >= '1' && first <= '5') ||
                        (first == '7' && rangeIndex(second) >= rangeIndex('F'));

    const WmiEntry* wmi = info.vin.size() >= 2 ? findWmi(info.vin) : nullptr;
    if (wmi) {
        info.manufacturer = wmi->manufacturer;
        info.brandCandidates = splitBrands(wmi->brands);
        if (info.brandCandidates.size() == 1) info.brand = info.brandCandidates.front();
        if (wmi->country.de) info.country = wmi->country.get();
    } else if (info.vin.size() >= 3) {
        info.notes.push_back(L("Der Hersteller ist in der Tabelle nicht verzeichnet.",
                               "The manufacturer is not in the table."));
    }

    if (info.vin.size() != 17) return info;

    // Check digit. Mandatory in North America and China, optional elsewhere.
    info.checkDigit = info.vin[8];
    info.expectedCheckDigit = vinCheckDigit(info.vin);
    info.checkDigitOk = info.expectedCheckDigit != 0 && info.checkDigit == info.expectedCheckDigit;
    info.checkDigitApplies = info.northAmerica || chinese;
    if (info.checkDigitApplies && !info.checkDigitOk && info.valid) {
        info.problems.push_back(L("Die Prüfziffer an Stelle 9 stimmt nicht. Wahrscheinlich ist ein Zeichen "
                                  "falsch abgeschrieben.",
                                  "The check digit at position 9 does not match. Probably a character "
                                  "was copied wrongly."));
    } else if (!info.checkDigitApplies && info.valid) {
        if (info.checkDigitOk) {
            info.notes.push_back(L("Die Prüfziffer an Stelle 9 stimmt. Das spricht für ein Fahrzeug, das für "
                                   "Nordamerika gebaut wurde.",
                                   "The check digit at position 9 matches. That suggests a vehicle built "
                                   "for North America."));
        } else {
            info.notes.push_back(L("Die Prüfziffer passt nicht, ist bei Fahrzeugen für Europa aber nicht "
                                   "vorgeschrieben. Das ist kein Fehler.",
                                   "The check digit does not match, but it is not required for vehicles "
                                   "built for Europe. That is not an error."));
        }
    }
    // A matching check digit outside North America almost always means a
    // car built to US rules, including the position 7 rule below.
    const bool usRules = info.northAmerica || (!chinese && info.checkDigitOk);

    info.plant = info.vin[10];
    info.serial = info.vin.substr(11);

    // Maker specific model codes.
    if (wmi) {
        switch (wmi->codeRule) {
            case C::Vag78:
            case C::Porsche78:
                if (info.vin.substr(6, 2) != "ZZ") info.modelCode = info.vin.substr(6, 2);
                break;
            case C::Tesla4:
                info.modelCode = info.vin.substr(3, 1);
                break;
            case C::Mercedes456:
                // European layout: three digits of the model series. North
                // American VINs use letter codes there, which we leave alone.
                if (isDigit(info.vin[3]) && isDigit(info.vin[4]) && isDigit(info.vin[5]))
                    info.modelCode = info.vin.substr(3, 3);
                break;
            case C::None:
                break;
        }
        if (!info.modelCode.empty()) info.modelCodeRule = wmi->codeRule;
    }

    // Model year from position 10.
    info.yearCode = info.vin[9];
    VinYearRule rule = wmi ? wmi->yearRule : (info.northAmerica || chinese ? kStd : kUnk);
    if (usRules || chinese) rule = kStd;
    if (wmi && wmi->codeRule == C::Mercedes456 && !info.modelCode.empty() && !usRules) {
        // Mercedes-Benz writes no model year into European VINs.
        rule = kNoYear;
    }

    const int base = vinBaseYear(info.yearCode);
    if (rule == kNoYear) {
        info.notes.push_back(L("Dieser Hersteller schreibt bei Fahrzeugen für Europa kein Baujahr in die "
                               "Fahrgestellnummer. Es steht im Fahrzeugschein.",
                               "This manufacturer writes no model year into VINs for Europe. It is in "
                               "the registration papers."));
        return info;
    }
    if (base == 0) {
        info.notes.push_back(L("Stelle 10 enthält kein Baujahr. In Europa ist das erlaubt, das Baujahr steht "
                               "dann nur im Fahrzeugschein.",
                               "Position 10 holds no model year. That is allowed in Europe, the year is "
                               "then only in the registration papers."));
        return info;
    }

    const int maxYear = currentYear + 1;
    if (usRules) {
        // US rule since model year 2010: a letter at position 7 marks the
        // second cycle (2010-2039), a digit the first one (1980-2009).
        const bool secondCycle = !isDigit(info.vin[6]);
        info.modelYears.push_back(secondCycle ? base + 30 : base);
        info.modelYearCertain = true;
    } else {
        if (base + 30 <= maxYear) info.modelYears.push_back(base + 30);
        info.modelYears.push_back(base);
        info.modelYearCertain = rule == kStd && info.modelYears.size() == 1;
        if (rule == kUnk) {
            info.notes.push_back(L("Ob dieser Hersteller an Stelle 10 das Baujahr schreibt, ist nicht sicher. "
                                   "Das Baujahr ist daher nur eine Vermutung.",
                                   "It is not certain that this manufacturer puts the model year at "
                                   "position 10. The year is therefore only a guess."));
        }
    }
    info.modelYear = info.modelYears.front();
    return info;
}

}  // namespace caruspex
