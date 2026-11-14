use crate::data::{Exam, Tracker, atomic_write, read_json};
use chrono::NaiveDate;
use scraper::{ElementRef, Html, Selector};
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Default)]
struct Counts {
    correct: i64,
    wrong: i64,
    blank: i64,
}

fn includes_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn map_legacy_outcome(subject_id: &str, outcome: &str) -> Option<&'static str> {
    let text = fold(outcome);
    let result = match subject_id {
        "turkce" => {
            // Must come before the broad "sözcük" rule below, which would otherwise swallow it.
            if text.contains("sözcük tür") {
                "turkce-sozcuk-turleri"
            } else if includes_any(
                &text,
                &["sözcükte anlam", "sözcük", "kelime", "deyim", "atasöz"],
            ) {
                "turkce-sozcuk-anlam"
            } else if includes_any(&text, &["cümlede anlam", "cümle"]) {
                "turkce-cumlede-anlam"
            } else if includes_any(
                &text,
                &["anlatım teknik", "anlatım biçim", "düşünceyi geliştirme"],
            ) {
                "turkce-paragraf-anlatim"
            } else if includes_any(
                &text,
                &[
                    "ana fikir",
                    "ana düşünce",
                    "yapı",
                    "konu",
                    "ikiye ayırma",
                    "akışını bozan",
                ],
            ) {
                "turkce-paragraf-yapi"
            } else if includes_any(
                &text,
                &[
                    "paragraf",
                    "yardımcı düşünce",
                    "çıkarım",
                    "çoklu soru",
                    "metin",
                ],
            ) {
                "turkce-paragraf-yorum"
            } else if text.contains("ses") {
                "turkce-ses-bilgisi"
            } else if text.contains("yazım") || text.contains("büyük harf") {
                "turkce-yazim"
            } else if text.contains("noktalama") {
                "turkce-noktalama"
            } else if includes_any(
                &text,
                &[
                    "isim",
                    "sıfat",
                    "zamir",
                    "zarf",
                    "sözcük tür",
                    "çekim eki",
                    "ekleri",
                ],
            ) {
                "turkce-sozcuk-turleri"
            } else if includes_any(&text, &["edat", "bağlaç", "ünlem"]) {
                "turkce-edat-baglac-unlem"
            } else if text.contains("fiilimsi") {
                "turkce-fiilimsiler"
            } else if text.contains("fiil") {
                "turkce-fiiller"
            } else if text.contains("öge") || text.contains("öğe") {
                "turkce-cumle-ogeleri"
            } else if text.contains("cümle tür") {
                "turkce-cumle-turleri"
            } else if text.contains("anlatım bozuk") {
                "turkce-anlatim-bozukluklari"
            } else {
                "turkce-paragraf-yorum"
            }
        }
        "matematik" => {
            if includes_any(&text, &["açı", "doğruda açı"]) {
                "geometri-temel-kavramlar-acilar"
            } else if text.contains("öklid") || text.contains("dik üçgen") {
                "geometri-dik-ucgen-oklid"
            } else if text.contains("eşkenar") || text.contains("ikizkenar") {
                "geometri-eskenar-ikizkenar"
            } else if text.contains("alan") && includes_any(&text, &["üçgen", "geometri"]) {
                "geometri-ucgende-alan"
            } else if includes_any(&text, &["benzerlik", "eşlik"]) {
                "geometri-benzerlik-eslik"
            } else if includes_any(&text, &["üçgen", "kenarorta", "açıortay"]) {
                "geometri-ucgende-aci-uzunluk"
            } else if includes_any(&text, &["çember", "daire"]) {
                "geometri-cember-daire"
            } else if includes_any(
                &text,
                &[
                    "katı cisim",
                    "prizma",
                    "piramit",
                    "koni",
                    "silindir",
                    "küre",
                ],
            ) {
                "geometri-kati-cisimler"
            } else if text.contains("analitik") {
                "geometri-analitik"
            } else if text.contains("dönüşüm") {
                "geometri-donusum"
            } else if includes_any(
                &text,
                &[
                    "dörtgen",
                    "çokgen",
                    "yamuk",
                    "paralelkenar",
                    "eşkenar dörtgen",
                    "dikdörtgen",
                    "kare",
                ],
            ) {
                "geometri-ucgende-aci-uzunluk"
            } else if includes_any(
                &text,
                &[
                    "sayı küme",
                    "temel kavram",
                    "ardışık",
                    "tek ve çift",
                    "faktöriyel",
                    "asal sayı",
                ],
            ) {
                "matematik-temel-kavramlar"
            } else if text.contains("basamak") {
                "matematik-sayi-basamaklari"
            } else if text.contains("bölün") || text.contains("bölme") {
                "matematik-bolme-bolunebilme"
            } else if text.contains("ebob") || text.contains("ekok") {
                "matematik-ebob-ekok"
            } else if text.contains("rasyonel") {
                "matematik-rasyonel-sayilar"
            } else if text.contains("ondalık") {
                "matematik-ondalik-sayilar"
            } else if text.contains("eşitsizlik") || text.contains("aralık") {
                "matematik-basit-esitsizlikler"
            } else if text.contains("mutlak") {
                "matematik-mutlak-deger"
            } else if text.contains("üslü") {
                "matematik-uslu-sayilar"
            } else if text.contains("köklü") {
                "matematik-koklu-sayilar"
            } else if text.contains("çarpan") || text.contains("özdeşlik") {
                "matematik-carpanlara-ayirma"
            } else if text.contains("oran") || text.contains("orantı") {
                "matematik-oran-oranti"
            } else if text.contains("denklem") {
                "matematik-birinci-derece-denklemler"
            } else if text.contains("yaş") && text.contains("problem") {
                "matematik-problem-yas"
            } else if includes_any(&text, &["işçi", "havuz"]) && text.contains("problem") {
                "matematik-problem-isci-havuz"
            } else if includes_any(&text, &["hız", "hareket"]) && text.contains("problem") {
                "matematik-problem-hareket"
            } else if text.contains("karışım") && text.contains("problem") {
                "matematik-problem-karisim"
            } else if includes_any(&text, &["yüzde", "kâr", "zarar", "faiz"])
                && text.contains("problem")
            {
                "matematik-problem-yuzde-kar-zarar"
            } else if includes_any(
                &text,
                &["kesir", "sayı problem", "rutin olmayan", "periyodik"],
            ) {
                "matematik-problem-sayi-kesir"
            } else if text.contains("küme") {
                "matematik-kumeler"
            } else if text.contains("fonksiyon") {
                "matematik-fonksiyonlar"
            } else if includes_any(&text, &["permütasyon", "kombinasyon", "sayma"]) {
                "matematik-permutasyon-kombinasyon"
            } else if text.contains("olasılık") {
                "matematik-olasilik"
            } else if includes_any(
                &text,
                &[
                    "veri",
                    "grafik",
                    "merkezi eğilim",
                    "medyan",
                    "mod",
                    "açıklık",
                    "standart sapma",
                ],
            ) {
                "matematik-veri-grafik-tablo"
            } else if text.contains("mantık") || text.contains("önerme") {
                "matematik-sayisal-mantik"
            } else if text.contains("problem") {
                "matematik-problem-sayi-kesir"
            } else {
                "matematik-temel-kavramlar"
            }
        }
        "fizik" => {
            if text.contains("fizik bilimine giriş") || text.contains("fiziksel nicelik") {
                "fizik-bilimine-giris"
            } else if includes_any(&text, &["özkütle", "madde", "dayanıklılık"]) {
                "fizik-madde-ozellikleri"
            } else if text.contains("basınç") {
                "fizik-basinc"
            } else if text.contains("kaldırma") {
                "fizik-kaldirma-kuvveti"
            } else if includes_any(&text, &["ısı", "sıcaklık", "genleşme"]) {
                "fizik-isi-sicaklik-genlesme"
            } else if includes_any(
                &text,
                &[
                    "hız",
                    "ivme",
                    "konum",
                    "alınan yol",
                    "sürat",
                    "doğrusal hareket",
                ],
            ) {
                "fizik-hareket"
            } else if includes_any(&text, &["kuvvet", "newton", "dinamik", "sürtünme"]) {
                "fizik-dinamik"
            } else if includes_any(&text, &["iş", "güç", "enerji"]) {
                "fizik-is-guc-enerji"
            } else if text.contains("elektrostatik") || text.contains("yük") {
                "fizik-elektrostatik"
            } else if includes_any(
                &text,
                &[
                    "elektrik akımı",
                    "devre",
                    "direnç",
                    "potansiyel fark",
                    "lamba",
                    "manyetizma",
                    "mıknatıs",
                ],
            ) {
                "fizik-elektrik-manyetizma"
            } else if includes_any(
                &text,
                &[
                    "optik",
                    "ışık",
                    "gölge",
                    "yansıma",
                    "kırılma",
                    "ayna",
                    "mercek",
                    "renk",
                ],
            ) {
                "fizik-optik"
            } else if text.contains("dalga") {
                "fizik-dalgalar"
            } else {
                "fizik-bilimine-giris"
            }
        }
        "kimya" => {
            if includes_any(
                &text,
                &[
                    "simya",
                    "güvenlik",
                    "laboratuvar",
                    "çalışma alanları",
                    "kimya bilimi",
                ],
            ) {
                "kimya-kimya-bilimi"
            } else if includes_any(&text, &["atom", "periyodik sistem", "periyodik özellik"]) {
                "kimya-atom-periyodik-sistem"
            } else if includes_any(
                &text,
                &["kimyasal tür", "etkileşim", "bağ", "kovalent", "iyonik"],
            ) {
                "kimya-kimyasal-turler-etkilesim"
            } else if includes_any(
                &text,
                &[
                    "maddenin halleri",
                    "gazlar",
                    "sıvılar",
                    "katılar",
                    "viskozite",
                ],
            ) {
                "kimya-maddenin-halleri"
            } else if includes_any(&text, &["mol", "hesaplama", "kimyasal tepkime"]) {
                "kimya-kimyasal-hesaplamalar"
            } else if includes_any(
                &text,
                &["karışım", "çözelti", "ayırma", "saflaştırma", "çözünme"],
            ) {
                "kimya-karisimlar"
            } else if includes_any(&text, &["asit", "baz", "tuz", "nötrleşme"]) {
                "kimya-asitler-bazlar-tuzlar"
            } else if includes_any(
                &text,
                &["her yerde", "temizlik", "polimer", "kozmetik", "ilaç"],
            ) {
                "kimya-her-yerde"
            } else {
                "kimya-kimya-bilimi"
            }
        }
        "biyoloji" => {
            if includes_any(
                &text,
                &[
                    "canlıların ortak",
                    "organik",
                    "inorganik",
                    "karbohidrat",
                    "lipit",
                    "protein",
                    "enzim",
                    "vitamin",
                    "nükleik",
                ],
            ) {
                "biyoloji-temel-bilesenler"
            } else if includes_any(&text, &["hücre", "organel", "hücresel", "zarından madde"]) {
                "biyoloji-hucre"
            } else if includes_any(
                &text,
                &[
                    "sınıflandırma",
                    "canlılar dünyası",
                    "hayvan",
                    "bitki",
                    "bakteri",
                    "mantar",
                ],
            ) {
                "biyoloji-canlilar-dunyasi"
            } else if includes_any(&text, &["mitoz", "eşeysiz", "mayoz", "eşeyli", "bölünme"]) {
                "biyoloji-hucre-bolunmeleri"
            } else if includes_any(&text, &["kalıtım", "soyağacı", "mendel", "cinsiyete bağlı"])
            {
                "biyoloji-kalitim"
            } else if includes_any(&text, &["ekosistem", "ekoloji", "besin", "madde döngü"]) {
                "biyoloji-ekosistem-ekolojisi"
            } else {
                "biyoloji-temel-bilesenler"
            }
        }
        "tarih" => {
            if includes_any(&text, &["tarih ve zaman", "tarih öğrenme", "tarih bilimi"]) {
                "tarih-tarih-ve-zaman"
            } else if includes_any(&text, &["ilk çağ", "uygarlık"]) {
                "tarih-ilk-ve-orta-caglar"
            } else if includes_any(&text, &["orta çağ", "avrupa", "dünya"]) {
                "tarih-orta-cagda-dunya"
            } else if includes_any(&text, &["türk dünyası", "ilk türk"]) {
                "tarih-ilk-turk-devletleri"
            } else if includes_any(&text, &["islam medeniyeti", "islam"]) {
                "tarih-islam-medeniyetinin-dogusu"
            } else if includes_any(
                &text,
                &["selçuklu", "eyyubi", "türk islam", "türklerin islam"],
            ) {
                "tarih-turk-islam-devletleri"
            } else if includes_any(&text, &["beylikten devlete", "kuruluş", "osmanlı devleti"]) {
                "tarih-yerlesme-ve-devletlesme"
            } else if includes_any(&text, &["dünya gücü", "yükselme"]) {
                "tarih-dunya-gucu-osmanli"
            } else if includes_any(&text, &["değişim", "17. yüzyıl", "18. yüzyıl"]) {
                "tarih-degisen-dunya-dengeleri"
            } else if includes_any(
                &text,
                &[
                    "tanzimat",
                    "ıslahat",
                    "kanun-ı esasi",
                    "meşrutiyet",
                    "ihtilal",
                ],
            ) {
                "tarih-devrimler-caginda-degisen-devlet-toplum"
            } else if includes_any(&text, &["20. yüzyıl", "mustafa kemal", "birinci dünya"]) {
                "tarih-20-yuzyil-baslarinda-osmanli"
            } else if includes_any(
                &text,
                &[
                    "milli mücadele",
                    "meclis",
                    "büyük millet",
                    "sevr",
                    "lozan",
                    "kurtuluş",
                ],
            ) {
                "tarih-milli-mucadele"
            } else {
                "tarih-tarih-ve-zaman"
            }
        }
        "cografya" => {
            if includes_any(&text, &["doğa", "insan", "bölümlenme", "coğrafyanın konu"]) {
                "cografya-doga-ve-insan"
            } else if includes_any(&text, &["dünya'nın şekli", "dünyanın şekli", "hareket"]) {
                "cografya-dunyanin-sekli-ve-hareketleri"
            } else if includes_any(&text, &["koordinat", "paralel", "meridyen", "yerel saat"]) {
                "cografya-cografi-konum"
            } else if includes_any(&text, &["harita", "ölçek", "izohips"]) {
                "cografya-harita-bilgisi"
            } else if includes_any(
                &text,
                &["iklim", "sıcaklık", "basınç", "rüzgar", "nem", "yağış"],
            ) {
                "cografya-iklim-bilgisi"
            } else if includes_any(&text, &["tektonik", "iç kuvvet", "deprem", "volkan"]) {
                "cografya-ic-kuvvetler"
            } else if includes_any(&text, &["dış kuvvet", "akarsu", "rüzgar", "buzul"]) {
                "cografya-dis-kuvvetler"
            } else if includes_any(&text, &["nüfus", "piramit"]) {
                "cografya-nufus-ve-yerlesme"
            } else if text.contains("bölge") {
                "cografya-bolge-kavrami"
            } else if includes_any(&text, &["afet", "deprem"]) {
                "cografya-dogal-afetler"
            } else {
                "cografya-doga-ve-insan"
            }
        }
        "felsefe" => {
            if includes_any(
                &text,
                &[
                    "felsefeyi tanıma",
                    "felsefi düşünce",
                    "felsefenin anlamı",
                    "rolü",
                ],
            ) {
                "felsefe-felsefeyi-tanima"
            } else if includes_any(&text, &["felsefe ile düşünme", "sorular oluşturur"]) {
                "felsefe-felsefe-ile-dusunme"
            } else if text.contains("varlık") {
                "felsefe-varlik-felsefesi"
            } else if text.contains("bilgi") {
                "felsefe-bilgi-felsefesi"
            } else if includes_any(&text, &["bilim", "bilim felsefesi"]) {
                "felsefe-bilim-felsefesi"
            } else if text.contains("ahlak") {
                "felsefe-ahlak-felsefesi"
            } else if text.contains("din felsefesi") {
                "felsefe-din-felsefesi"
            } else if text.contains("siyaset") {
                "felsefe-siyaset-felsefesi"
            } else if text.contains("sanat") {
                "felsefe-sanat-felsefesi"
            } else if includes_any(&text, &["mö 6", "ms 2", "ilk çağ"]) {
                "felsefe-mo6-ms2-felsefesi"
            } else if includes_any(&text, &["ms 2. yüzyıl-15", "orta çağ"]) {
                "felsefe-ms2-ms15-felsefesi"
            } else if includes_any(
                &text,
                &[
                    "15. yüzyıl",
                    "17. yüzyıl",
                    "18. yüzyıl",
                    "19. yüzyıl",
                    "20. yüzyıl",
                ],
            ) {
                "felsefe-15-17-yuzyil-felsefesi"
            } else {
                "felsefe-felsefeyi-tanima"
            }
        }
        "din" => {
            if includes_any(&text, &["bilgi", "inanç", "iman"]) {
                "din-bilgi-ve-inanc"
            } else if includes_any(&text, &["din", "islam", "tanım"]) {
                "din-din-ve-islam"
            } else if includes_any(&text, &["islam ve ibadet", "ibadet"]) {
                "din-islam-ve-ibadet"
            } else if includes_any(&text, &["gençlik", "ahlak", "değer"]) {
                "din-genclik-ve-degerler"
            } else if includes_any(&text, &["allah", "insan"]) {
                "din-allah-insan-iliskisi"
            } else if includes_any(&text, &["hz. muhammed", "peygamber"]) {
                "din-hz-muhammed-ve-genclik"
            } else if includes_any(
                &text,
                &[
                    "itikadi", "siyasi", "fıkhi", "tasavvuf", "yorumlar", "güncel",
                ],
            ) {
                "din-islam-dusuncesinde-yorumlar"
            } else {
                "din-bilgi-ve-inanc"
            }
        }
        _ => return None,
    };
    Some(result)
}

/// Conditional remaps: (legacy id, any-of needles in the lowercased outcome, canonical id).
/// Checked before `LEGACY_REMAP`.
const LEGACY_REMAP_IF: &[(&str, &[&str], &str)] = &[
    (
        "biyoloji-hucre",
        &["zarından madde"],
        "biyoloji-hucre-zari-madde-gecisi",
    ),
    (
        "cografya-nufus-ve-yerlesme",
        &["yerleşme"],
        "cografya-yerlesme",
    ),
    ("din-din-ve-islam", &["ibadet"], "din-islam-ibadetler"),
    (
        "felsefe-15-17-yuzyil-felsefesi",
        &["20. yüzyıl"],
        "felsefe-20-yuzyil",
    ),
    (
        "fizik-elektrik-manyetizma",
        &["manyetizma", "mıknatıs"],
        "fizik-manyetizma",
    ),
];

/// Unconditional remaps from the ids produced by `map_legacy_outcome` to the ids in `topics/tyt.json`.
const LEGACY_REMAP: &[(&str, &str)] = &[
    ("biyoloji-temel-bilesenler", "biyoloji-temel-bilesikler"),
    ("biyoloji-hucre", "biyoloji-hucre-organeller"),
    ("biyoloji-canlilar-dunyasi", "biyoloji-siniflandirma"),
    ("cografya-doga-ve-insan", "cografya-doga-insan"),
    (
        "cografya-dunyanin-sekli-ve-hareketleri",
        "cografya-dunya-sekli-hareketleri",
    ),
    ("cografya-cografi-konum", "cografya-koordinat-sistemi"),
    ("cografya-iklim-bilgisi", "cografya-iklim-hava-durumu"),
    ("cografya-nufus-ve-yerlesme", "cografya-nufus"),
    (
        "cografya-bolge-kavrami",
        "cografya-turkiye-cografi-ozellikleri",
    ),
    ("cografya-dogal-afetler", "cografya-dogal-afetler-cevre"),
    ("din-allah-insan-iliskisi", "din-islam-inanc-esaslari"),
    ("din-bilgi-ve-inanc", "din-islam-inanc-esaslari"),
    ("din-din-ve-islam", "din-ve-islam"),
    ("din-islam-dusuncesinde-yorumlar", "din-ve-islam"),
    ("din-islam-ve-ibadet", "din-islam-ibadetler"),
    ("din-genclik-ve-degerler", "din-islam-ahlak"),
    ("din-hz-muhammed-ve-genclik", "din-hz-muhammed-hayati"),
    ("felsefe-15-17-yuzyil-felsefesi", "felsefe-17-19-yuzyil"),
    ("felsefe-felsefe-ile-dusunme", "felsefe-felsefenin-konusu"),
    ("felsefe-felsefeyi-tanima", "felsefe-felsefenin-konusu"),
    ("felsefe-mo6-ms2-felsefesi", "felsefe-ilk-cag"),
    ("felsefe-ms2-ms15-felsefesi", "felsefe-orta-cag-ronesans"),
    ("fizik-dinamik", "fizik-kuvvet-newton"),
    ("fizik-elektrik-manyetizma", "fizik-elektrik-akimi-devreler"),
    ("fizik-optik", "fizik-isik-optik"),
    ("kimya-kimya-bilimi", "kimya-bilimi-laboratuvar"),
    (
        "kimya-kimyasal-turler-etkilesim",
        "kimya-kimyasal-etkilesimler",
    ),
    ("kimya-kimyasal-hesaplamalar", "kimya-kimyasal-tepkimeler"),
    ("kimya-asitler-bazlar-tuzlar", "kimya-asit-baz-tuz"),
    ("tarih-tarih-ve-zaman", "tarih-bilimi"),
    ("tarih-ilk-ve-orta-caglar", "tarih-ilk-cag-uygarliklari"),
    (
        "tarih-orta-cagda-dunya",
        "tarih-turkiye-selcuklulari-beylikler",
    ),
    (
        "tarih-ilk-turk-devletleri",
        "tarih-islamiyet-oncesi-turk-tarihi",
    ),
    (
        "tarih-islam-medeniyetinin-dogusu",
        "tarih-islam-tarihi-ilk-turk-islam-devletleri",
    ),
    (
        "tarih-turk-islam-devletleri",
        "tarih-islam-tarihi-ilk-turk-islam-devletleri",
    ),
    ("tarih-yerlesme-ve-devletlesme", "tarih-osmanli-kurulus"),
    ("tarih-dunya-gucu-osmanli", "tarih-osmanli-yukselme"),
    (
        "tarih-degisen-dunya-dengeleri",
        "tarih-osmanli-duraklama-gerileme",
    ),
    (
        "tarih-devrimler-caginda-degisen-devlet-toplum",
        "tarih-osmanli-dagilma",
    ),
    (
        "tarih-20-yuzyil-baslarinda-osmanli",
        "tarih-osmanli-dagilma",
    ),
    ("tarih-milli-mucadele", "tarih-kurtulus-savasi"),
];

/// True for TYT topic ids that only existed in older `tyt.json` versions. The importer never
/// assigns results to them, so they are removed from existing data files on load.
pub fn is_retired_tyt_topic(id: &str) -> bool {
    LEGACY_REMAP.iter().any(|(source, _)| *source == id)
        || LEGACY_REMAP_IF.iter().any(|(source, _, _)| *source == id)
}

fn canonical_topic_id(legacy_id: &str, outcome: &str) -> Option<String> {
    let text = fold(outcome);
    if let Some((_, _, target)) = LEGACY_REMAP_IF
        .iter()
        .find(|(id, needles, _)| *id == legacy_id && includes_any(&text, needles))
    {
        return Some((*target).to_owned());
    }
    let canonical = LEGACY_REMAP
        .iter()
        .find(|(id, _)| *id == legacy_id)
        .map_or(legacy_id, |(_, target)| *target);
    Some(canonical.to_owned())
}

fn map_outcome(subject_id: &str, outcome: &str) -> Option<String> {
    let legacy_id = map_legacy_outcome(subject_id, outcome)?;
    canonical_topic_id(legacy_id, outcome)
}

fn map_ayt_outcome(subject_id: &str, outcome: &str) -> Option<&'static str> {
    let text = fold(outcome);
    let topic = match subject_id {
        "matematik" => {
            if includes_any(&text, &["binom", "pascal"]) {
                "ayt-mat-binom"
            } else if includes_any(
                &text,
                &[
                    "trigonometri",
                    "trigonometrik",
                    "sinüs",
                    "kosinüs",
                    "tanjant",
                    "birim çember",
                    "toplam-fark",
                ],
            ) {
                "ayt-mat-trigonometri"
            } else if includes_any(&text, &["analitik"]) {
                "ayt-geo-analitik"
            } else if includes_any(&text, &["dönüşüm geometrisi", "dönüşüm", "vektör"]) {
                "ayt-geo-donusum-vektor"
            } else if includes_any(&text, &["üçgende alan"]) {
                "ayt-geo-ucgende-alan"
            } else if includes_any(&text, &["çember", "daire"]) {
                "ayt-geo-cember-daire"
            } else if includes_any(
                &text,
                &[
                    "silindir",
                    "koni",
                    "küre",
                    "prizma",
                    "piramit",
                    "katı cisim",
                ],
            ) {
                "ayt-geo-kati-cisimler"
            } else if includes_any(&text, &["üçgen", "kenarortay", "açıortay"]) {
                "ayt-geo-ucgen-aci-uzunluk"
            } else if includes_any(&text, &["benzerlik", "eşlik"]) {
                "ayt-geo-benzerlik-eslik"
            } else if includes_any(
                &text,
                &[
                    "çokgen",
                    "dörtgen",
                    "yamuk",
                    "paralelkenar",
                    "eşkenar dörtgen",
                    "dikdörtgen",
                    "kare",
                ],
            ) {
                "ayt-geo-cokgen-dortgen"
            } else if includes_any(&text, &["doğruda açı", "temel kavram"]) {
                "ayt-geo-temel-kavramlar"
            } else if includes_any(&text, &["logaritma", "logaritmik"]) {
                "ayt-mat-logaritma"
            } else if text.contains("integral") {
                "ayt-mat-integral"
            } else if includes_any(
                &text,
                &[
                    "türev",
                    "maksimum-minimum",
                    "minimum-maksimum",
                    "ekstremum",
                    "artanlık",
                    "azalanlık",
                ],
            ) {
                "ayt-mat-turev"
            } else if text.contains("süreklilik") {
                "ayt-mat-sureklilik"
            } else if includes_any(&text, &["limit", "0/0", "belirsizliği"]) {
                "ayt-mat-limit"
            } else if includes_any(&text, &["önerme", "mantık"]) {
                "ayt-mat-mantik"
            } else if text.contains("fonksiyon grafik") {
                "ayt-mat-fonksiyonlar"
            } else if includes_any(
                &text,
                &[
                    "ikinci dereceden eşitsizlik",
                    "ikinci derece eşitsizlik",
                    "eşitsizlik sistemi",
                    "eşitsizlik sistemleri",
                ],
            ) {
                "ayt-mat-ikinci-derece-esitsizlikler"
            } else if includes_any(&text, &["eşitsizlik", "mutlak"]) {
                "ayt-mat-esitsizlik-mutlak"
            } else if text.contains("parabol") {
                "ayt-mat-parabol"
            } else if includes_any(
                &text,
                &[
                    "kök-katsayı",
                    "kökleri verilen denklem",
                    "ikinci derece denklem",
                ],
            ) {
                "ayt-mat-ikinci-derece-denklemler"
            } else if text.contains("polinom") {
                "ayt-mat-polinomlar"
            } else if includes_any(&text, &["çarpanlara ayırma", "özdeşlik"]) {
                "ayt-mat-carpanlara-ayirma"
            } else if includes_any(&text, &["üslü", "köklü", "radikal"]) {
                "ayt-mat-uslu-koklu"
            } else if includes_any(
                &text,
                &[
                    "asal sayı",
                    "temel kavram",
                    "sayı küme",
                    "ardışık",
                    "tek ve çift",
                ],
            ) {
                "ayt-mat-temel-kavramlar"
            } else if text.contains("basamak") {
                "ayt-mat-sayi-basamaklari"
            } else if includes_any(&text, &["bölme", "bölün"]) {
                "ayt-mat-bolme-bolunebilme"
            } else if text.contains("ebob") || text.contains("ekok") {
                "ayt-mat-ebob-ekok"
            } else if includes_any(&text, &["rasyonel", "ondalık"]) {
                "ayt-mat-rasyonel-ondalik"
            } else if includes_any(&text, &["oran", "orantı"]) {
                "ayt-mat-oran-oranti"
            } else if includes_any(&text, &["problem", "yaş", "hareket"]) {
                "ayt-mat-problemler"
            } else if text.contains("küme") {
                "ayt-mat-kumeler"
            } else if text.contains("fonksiyon") {
                "ayt-mat-fonksiyonlar"
            } else if includes_any(&text, &["karmaşık sayı", "kompleks sayı"]) {
                "ayt-mat-karmasik-sayilar"
            } else if text.contains("dizi") || text.contains("seri") {
                "ayt-mat-diziler"
            } else if includes_any(
                &text,
                &["toplam sembolü", "çarpım sembolü", "sigma", "pi sembolü"],
            ) {
                "ayt-mat-toplam-carpim"
            } else if includes_any(&text, &["permütasyon", "kombinasyon", "sayma"]) {
                "ayt-mat-permutasyon-kombinasyon"
            } else if text.contains("olasılık") {
                "ayt-mat-olasilik"
            } else if includes_any(&text, &["istatistik", "standart sapma", "medyan", "mod"]) {
                "ayt-mat-istatistik"
            } else {
                return None;
            }
        }
        "fizik" => {
            if text.contains("vektör") {
                "ayt-fizik-vektorler"
            } else if includes_any(&text, &["tork", "kütle merkezi"]) {
                "ayt-fizik-tork-kutle-merkezi"
            } else if text.contains("basit makine") {
                "ayt-fizik-basit-makineler"
            } else if includes_any(&text, &["açısal momentum", "çembersel", "dairesel hareket"])
            {
                "ayt-fizik-cembersel-hareket"
            } else if text.contains("basit harmonik") {
                "ayt-fizik-basit-harmonik-hareket"
            } else if includes_any(&text, &["itme", "momentum"]) {
                "ayt-fizik-itme-momentum"
            } else if includes_any(&text, &["elektromanyetik indüksiyon", "indüksiyon"]) {
                "ayt-fizik-elektromanyetik-induksiyon"
            } else if includes_any(&text, &["alternatif akım", "transformatör", "trafo"]) {
                "ayt-fizik-alternatif-akim"
            } else if includes_any(&text, &["özel görelilik", "görelilik"]) {
                "ayt-fizik-ozel-gorelilik"
            } else if includes_any(
                &text,
                &[
                    "fotoelektrik",
                    "fotoelektron",
                    "compton",
                    "modern atom",
                    "modern fizik",
                ],
            ) {
                "ayt-fizik-modern-fizik"
            } else if includes_any(
                &text,
                &["modern teknoloji", "yarı iletken", "nanoteknoloji"],
            ) {
                "ayt-fizik-modern-teknoloji"
            } else if includes_any(&text, &["dalga", "doppler"]) {
                "ayt-fizik-dalgalar"
            } else if includes_any(&text, &["optik", "ışık", "girişim", "kırınım"]) {
                "ayt-fizik-isik-optik"
            } else if includes_any(&text, &["manyetizma", "manyetik alan", "mıknatıs"]) {
                "ayt-fizik-manyetizma"
            } else if includes_any(&text, &["elektrik akımı", "devre", "direnç"]) {
                "ayt-fizik-elektrik-akimi-devreler"
            } else if includes_any(
                &text,
                &["elektrostatik", "elektriksel potansiyel", "elektrik alanı"],
            ) {
                "ayt-fizik-elektrostatik"
            } else if includes_any(&text, &["iş", "güç", "enerji", "kinetik", "potansiyel"]) {
                "ayt-fizik-is-guc-enerji"
            } else if text.contains("newton") {
                "ayt-fizik-newton"
            } else if includes_any(&text, &["kuvvet", "denge"]) {
                "ayt-fizik-kuvvet-denge"
            } else if includes_any(&text, &["hareket", "hız", "ivme", "atış"]) {
                "ayt-fizik-hareket"
            } else {
                return None;
            }
        }
        "kimya" => {
            if includes_any(&text, &["hibritleşme", "hibritleşmesi"]) {
                "ayt-kimya-etkilesimler"
            } else if includes_any(&text, &["kuantum", "orbital", "elektron dizilimi"]) {
                "ayt-kimya-kuantum-modeli"
            } else if includes_any(&text, &["periyodik sistem", "periyodik özellik"]) {
                "ayt-kimya-periyodik-sistem"
            } else if includes_any(&text, &["gaz yasa", "gazlar", "gaz"]) {
                "ayt-kimya-gazlar"
            } else if includes_any(
                &text,
                &["asit-baz", "asit", "baz", "ph", "titrasyon", "sulu çözelti"],
            ) {
                "ayt-kimya-asit-baz-denge"
            } else if includes_any(&text, &["çözünürlük dengesi", "ksp"]) {
                "ayt-kimya-cozunurluk-dengesi"
            } else if includes_any(&text, &["çözelti", "derişim", "derişik"]) {
                "ayt-kimya-cozeltiler"
            } else if includes_any(&text, &["entalpi", "termokimya", "termik", "ısı değişimi"])
            {
                "ayt-kimya-termokimya"
            } else if includes_any(
                &text,
                &[
                    "redoks",
                    "elektroliz",
                    "elektrot",
                    "elektrokimya",
                    "elektrokimyasal",
                ],
            ) {
                "ayt-kimya-elektrokimya"
            } else if includes_any(&text, &["tepkime hızı", "reaksiyon hızı", "kinetik"]) {
                "ayt-kimya-hiz"
            } else if text.contains("denge") {
                "ayt-kimya-denge"
            } else if includes_any(
                &text,
                &[
                    "fonksiyonel grup",
                    "alkol",
                    "eter",
                    "aldehit",
                    "keton",
                    "karboksilik",
                ],
            ) {
                "ayt-kimya-fonksiyonel-gruplar"
            } else if includes_any(&text, &["hidrokarbon", "alkan", "alken", "alkin"]) {
                "ayt-kimya-hidrokarbonlar"
            } else if includes_any(&text, &["aromatik", "benzen"]) {
                "ayt-kimya-aromatik"
            } else if includes_any(&text, &["organik tepkime", "izomer"]) {
                "ayt-kimya-organik-tepkimeler"
            } else if includes_any(&text, &["karbon kimyası", "organik kimya"]) {
                "ayt-kimya-karbon-kimyasi"
            } else if includes_any(
                &text,
                &["karbonhidrat", "lipit", "protein", "enzim", "vitamin"],
            ) {
                "ayt-kimya-yasam-kimyasi"
            } else if includes_any(&text, &["polimer", "günlük hayat"]) {
                "ayt-kimya-polimerler"
            } else if includes_any(&text, &["kimyasal tür", "etkileşim", "kovalent", "iyonik"]) {
                "ayt-kimya-etkilesimler"
            } else if includes_any(&text, &["tepkime", "reaksiyon"]) {
                "ayt-kimya-tepkimeler"
            } else if includes_any(&text, &["mol", "stokiyometri", "kimyasal hesaplama"]) {
                "ayt-kimya-hesaplamalar"
            } else {
                return None;
            }
        }
        "biyoloji" => {
            if includes_any(&text, &["bağışıklık", "antikor"]) {
                "ayt-biyo-bagisiklik"
            } else if includes_any(&text, &["boşaltım", "üriner", "böbrek"]) {
                "ayt-biyo-bosaltim"
            } else if includes_any(
                &text,
                &[
                    "kas çeşitleri",
                    "iskelet",
                    "kas sistemi",
                    "destek ve hareket",
                ],
            ) {
                "ayt-biyo-destek-hareket"
            } else if text.contains("sindirim") {
                "ayt-biyo-sindirim"
            } else if includes_any(&text, &["duyu organ"]) {
                "ayt-biyo-duyu-organlari"
            } else if includes_any(
                &text,
                &[
                    "komünite",
                    "popülasyon",
                    "ekosistem",
                    "ekoloji",
                    "besin zinciri",
                    "madde döngü",
                    "simbiyotik",
                ],
            ) {
                "ayt-biyo-ekosistem"
            } else if includes_any(
                &text,
                &[
                    "bitkilerde taşınma",
                    "bitkilerde su",
                    "su ve mineral taşınma",
                    "bitkilerde üreme",
                    "bitki üreme",
                ],
            ) {
                "ayt-biyo-bitki-tasima-ureme"
            } else if includes_any(&text, &["fotosentez", "fermantasyon", "bitkilerde solunum"]) {
                "ayt-biyo-fotosentez-solunum"
            } else if includes_any(
                &text,
                &["genetik mühendisliği", "biyoteknoloji", "gen aktarım"],
            ) {
                "ayt-biyo-biyoteknoloji"
            } else if includes_any(
                &text,
                &["nükleik asit", "dna eşlen", "rna", "moleküler kalıtım"],
            ) {
                "ayt-biyo-molekuler-kalitim"
            } else if includes_any(&text, &["protein sentez", "transkripsiyon", "translasyon"]) {
                "ayt-biyo-protein-sentezi"
            } else if includes_any(
                &text,
                &[
                    "kalıtım",
                    "soyağacı",
                    "mendel",
                    "kan grubu",
                    "cinsiyete bağlı",
                ],
            ) {
                "ayt-biyo-kalitim"
            } else if includes_any(&text, &["hücre zarı", "zarından madde", "madde geçiş"]) {
                "ayt-biyo-hucre-zari"
            } else if includes_any(&text, &["hücre bölün", "mitoz", "mayoz"]) {
                "ayt-biyo-hucre-bolunmeleri"
            } else if includes_any(&text, &["hücre", "organel"]) {
                "ayt-biyo-hucre-organeller"
            } else if includes_any(
                &text,
                &["sınıflandırma", "canlılar dünyası", "alem", "kingdom"],
            ) {
                "ayt-biyo-siniflandirma"
            } else if text.contains("solunum") {
                "ayt-biyo-solunum"
            } else if includes_any(&text, &["dolaşım", "lenf"]) {
                "ayt-biyo-dolasim"
            } else if includes_any(&text, &["sinir sistemi", "nöron"]) {
                "ayt-biyo-sinir"
            } else if includes_any(&text, &["endokrin", "hormon"]) {
                "ayt-biyo-endokrin"
            } else if includes_any(&text, &["üreme", "gelişim"]) {
                "ayt-biyo-ureme"
            } else if includes_any(&text, &["bitki doku", "bitkisel organ", "bitki organ"]) {
                "ayt-biyo-bitki-dokular"
            } else if includes_any(
                &text,
                &["çevre sorunu", "küresel ısınma", "çevre kirliliği"],
            ) {
                "ayt-biyo-cevre"
            } else if includes_any(
                &text,
                &[
                    "canlıların ortak",
                    "organik",
                    "inorganik",
                    "karbonhidrat",
                    "lipit",
                    "protein",
                    "enzim",
                    "vitamin",
                ],
            ) {
                "ayt-biyo-temel-bilesenler"
            } else {
                return None;
            }
        }
        "edebiyat" => {
            if includes_any(
                &text,
                &[
                    "edebi sanat",
                    "söz sanat",
                    "teşbih",
                    "istiare",
                    "mecaz-ı mürsel",
                    "tezat",
                    "tenasüp",
                    "hüsn-i talil",
                    "tecahül",
                    "kinaye",
                    "tariz",
                    "tevriye",
                ],
            ) {
                "ayt-edebiyat-edebi-sanatlar"
            } else if includes_any(
                &text,
                &[
                    "şiir bilgisi",
                    "nazım birimi",
                    "ölçü",
                    "kafiye",
                    "redif",
                    "durak",
                    "koşma",
                    "aruz",
                    "şiirde tema",
                    "şiirde ahenk",
                ],
            ) {
                "ayt-edebiyat-siir-bilgisi"
            } else if includes_any(&text, &["anlam", "sözcük", "cümle", "paragraf"]) {
                "ayt-edebiyat-anlam-bilgisi"
            } else if includes_any(
                &text,
                &[
                    "metin tür",
                    "masal",
                    "fabl",
                    "destan",
                    "tiyatro",
                    "mektup",
                    "günlük",
                    "hatıra",
                ],
            ) {
                "ayt-edebiyat-metin-turleri"
            } else if includes_any(
                &text,
                &[
                    "islamiyet öncesi",
                    "geçiş dönemi",
                    "kutadgu bilig",
                    "divanü lügati",
                    "atabetül",
                    "divan-ı hikmet",
                    "dede korkut",
                    "sagu",
                    "koşuk",
                ],
            ) {
                "ayt-edebiyat-islamiyet-oncesi-ve-gecis"
            } else if includes_any(
                &text,
                &[
                    "halk edebiyatı",
                    "âşık",
                    "aşık",
                    "yunus emre",
                    "karacaoğlan",
                    "dadaloğlu",
                    "köroğlu",
                    "pir sultan",
                    "veysel",
                    "tekke",
                    "tasavvuf",
                ],
            ) {
                "ayt-edebiyat-halk-edebiyati"
            } else if includes_any(
                &text,
                &[
                    "divan nazım",
                    "divan edebiyatında nazım",
                    "gazel",
                    "kaside",
                    "mesnevi",
                    "rubai",
                    "şarkı",
                    "tuyuğ",
                    "murabba",
                ],
            ) {
                "ayt-edebiyat-divan-nazim"
            } else if includes_any(
                &text,
                &[
                    "divan",
                    "fuzuli",
                    "baki",
                    "nedim",
                    "şeyh galip",
                    "nabi",
                    "nefi",
                    "katip çelebi",
                    "evliya çelebi",
                ],
            ) {
                "ayt-edebiyat-divan-nesir-ve-sairler"
            } else if includes_any(
                &text,
                &[
                    "tanzimat",
                    "şinasi",
                    "namık kemal",
                    "ziya paşa",
                    "recaizade",
                    "abdülhak hamit",
                ],
            ) {
                "ayt-edebiyat-tanzimat-donemi"
            } else if includes_any(
                &text,
                &[
                    "servet",
                    "fecr-i ati",
                    "fecriati",
                    "ahmet haşim",
                    "tevfik fikret",
                    "cenap şahabettin",
                    "halit ziya",
                ],
            ) {
                "ayt-edebiyat-servetifunun-ve-fecriati"
            } else if includes_any(
                &text,
                &[
                    "milli edebiyat",
                    "milli edebiyat",
                    "genç kalemler",
                    "ömer seyfettin",
                    "ziya gökalp",
                    "mehmet akif",
                    "yahya kemal",
                ],
            ) {
                "ayt-edebiyat-milli-edebiyat"
            } else if includes_any(
                &text,
                &[
                    "saf şiir",
                    "öz şiir",
                    "ahmet hamdi",
                    "cahit sıtkı",
                    "necip fazıl",
                    "yedi meşale",
                ],
            ) {
                "ayt-edebiyat-cumhuriyet-siir-saf"
            } else if includes_any(
                &text,
                &[
                    "toplumcu gerçekçi şiir",
                    "toplumcu şiir",
                    "nazım hikmet",
                    "ahmed arif",
                    "rıfat ilgaz",
                ],
            ) {
                "ayt-edebiyat-cumhuriyet-siir-toplumcu"
            } else if includes_any(
                &text,
                &["milli edebiyat zevk", "hisarcılar", "arif nihat asya"],
            ) {
                "ayt-edebiyat-cumhuriyet-siir-milli"
            } else if includes_any(
                &text,
                &["garip", "orhan veli", "oktay rifat", "melih cevdet"],
            ) {
                "ayt-edebiyat-cumhuriyet-siir-garip"
            } else if includes_any(
                &text,
                &[
                    "ikinci yeni",
                    "cemal süreya",
                    "edip cansever",
                    "turgut uyar",
                    "sezai karakoç",
                ],
            ) {
                "ayt-edebiyat-cumhuriyet-siir-ikinci-yeni"
            } else if includes_any(&text, &["1960", "1970", "ismet özel", "ataol behramoğlu"]) {
                "ayt-edebiyat-cumhuriyet-siir-1960-sonrasi"
            } else if includes_any(&text, &["1980", "çağdaş türk şiiri"]) {
                "ayt-edebiyat-cumhuriyet-siir-1980-sonrasi"
            } else if includes_any(
                &text,
                &[
                    "milli edebiyat zevkini sürdüren roman",
                    "yakup kadri",
                    "halide edib",
                    "reşat nuri",
                ],
            ) {
                "ayt-edebiyat-cumhuriyet-roman-milli-gercekci"
            } else if includes_any(
                &text,
                &[
                    "toplumcu gerçekçi roman",
                    "yaşar kemal",
                    "orhan kemal",
                    "kemal tahir",
                    "sabahattin ali",
                ],
            ) {
                "ayt-edebiyat-cumhuriyet-roman-toplumcu"
            } else if includes_any(&text, &["bireyin iç dünyası", "peyami safa", "tarık buğra"])
            {
                "ayt-edebiyat-cumhuriyet-roman-bireyin-ic-dunyasi"
            } else if includes_any(
                &text,
                &[
                    "modernizm",
                    "postmodernizm",
                    "oğuz atay",
                    "orhan pamuk",
                    "bilge karasu",
                    "latife tekin",
                ],
            ) {
                "ayt-edebiyat-cumhuriyet-roman-modernist"
            } else if includes_any(
                &text,
                &[
                    "cumhuriyet tiyatro",
                    "haldun taner",
                    "turgut özakman",
                    "güngör dilmen",
                ],
            ) {
                "ayt-edebiyat-cumhuriyet-tiyatro"
            } else if includes_any(
                &text,
                &[
                    "öğretici metin",
                    "deneme",
                    "makale",
                    "fıkra",
                    "eleştiri",
                    "nurullah ataç",
                    "cemil meriç",
                ],
            ) {
                "ayt-edebiyat-ogretici-metinler"
            } else if includes_any(
                &text,
                &[
                    "akım",
                    "romantizm",
                    "klasisizm",
                    "klasizm",
                    "realizm",
                    "natüralizm",
                    "sembolizm",
                    "parnasizm",
                    "sürrealizm",
                    "varoluşçuluk",
                ],
            ) {
                "ayt-edebiyat-edebiyat-akimlari-dunya"
            } else {
                return None;
            }
        }
        "tarih1" => {
            if includes_any(&text, &["tarih ve zaman", "tarih bilimi"]) {
                "ayt-tar1-tarih-ve-zaman"
            } else if includes_any(
                &text,
                &[
                    "ilk çağ",
                    "eski çağ",
                    "mezopotamya",
                    "anadolu uygarlık",
                    "mısır",
                ],
            ) {
                "ayt-tar1-insanligin-ilk-donemleri"
            } else if includes_any(&text, &["orta çağ", "feodalite", "haçlı", "bizans"]) {
                "ayt-tar1-orta-cagda-dunya"
            } else if includes_any(
                &text,
                &[
                    "ilk türk devleti",
                    "hun",
                    "göktürk",
                    "uygur",
                    "orta asya",
                    "türk dünyası",
                ],
            ) {
                "ayt-tar1-ilk-ve-orta-cagda-turk-dunyasi"
            } else if includes_any(
                &text,
                &["islam medeniyeti", "dört halife", "emeviler", "abbasiler"],
            ) {
                "ayt-tar1-islam-medeniyetinin-dogusu"
            } else if includes_any(
                &text,
                &["karahanlı", "gazneli", "büyük selçuklu", "türk islam"],
            ) {
                "ayt-tar1-turklerin-islamiyeti-kabulu"
            } else if includes_any(
                &text,
                &[
                    "türkiye selçuklu",
                    "anadolu selçuklu",
                    "beylikler",
                    "malazgirt",
                    "kösedağ",
                ],
            ) {
                "ayt-tar1-yerlesme-ve-devletlesme-turkiye-selcuklulari"
            } else if includes_any(&text, &["beylikten devlete", "kuruluş", "ankara savaşı"]) {
                "ayt-tar1-beylikten-devlete-osmanli"
            } else if includes_any(
                &text,
                &[
                    "dünya gücü",
                    "yükselme",
                    "fatih",
                    "kanuni",
                    "yavuz",
                    "kara ve deniz politika",
                    "1453-1520",
                ],
            ) {
                "ayt-tar1-dunya-gucu-osmanli"
            } else if includes_any(
                &text,
                &["değişen dünya dengeleri", "duraklama", "celali", "karlofça"],
            ) {
                "ayt-tar1-degisen-dunya-dengeleri"
            } else if includes_any(
                &text,
                &[
                    "değişim çağında",
                    "gerileme",
                    "lale devri",
                    "küçük kaynarca",
                ],
            ) {
                "ayt-tar1-degisim-caginda-avrupa-ve-osmanli"
            } else if includes_any(
                &text,
                &["denge stratejisi", "dağılma", "tanzimat", "meşrutiyet"],
            ) {
                "ayt-tar1-uluslararasi-iliskilerde-denge-stratejisi"
            } else if includes_any(
                &text,
                &[
                    "20. yüzyıl",
                    "trablusgarp",
                    "balkan savaşı",
                    "birinci dünya",
                    "mondros",
                    "1914-1918",
                ],
            ) {
                "ayt-tar1-20-yy-baslarinda-osmanli-ve-dunya"
            } else if includes_any(
                &text,
                &[
                    "milli mücadeleye hazırlık",
                    "kongreler",
                    "genelgeler",
                    "misak-ı milli",
                    "tbmm",
                ],
            ) {
                "ayt-tar1-milli-mucadele-hazirlik"
            } else if includes_any(
                &text,
                &[
                    "cepheler",
                    "sakarya",
                    "büyük taarruz",
                    "inönü",
                    "mudanya",
                    "lozan",
                ],
            ) {
                "ayt-tar1-milli-mucadele-muharebeler"
            } else if includes_any(
                &text,
                &[
                    "inkılap",
                    "ilke",
                    "saltanatın kaldırılması",
                    "cumhuriyetin ilanı",
                    "halifelik",
                ],
            ) {
                "ayt-tar1-ataturkculuk-ve-turk-inkilabi"
            } else if includes_any(
                &text,
                &[
                    "dış politika",
                    "hatay",
                    "montrö",
                    "balkan antantı",
                    "sadabat",
                ],
            ) {
                "ayt-tar1-ataturk-donemi-dis-politika"
            } else {
                return None;
            }
        }
        "cografya1" => {
            if includes_any(
                &text,
                &["ekosistem", "biyoçeşitlilik", "biyom", "madde döngüsü"],
            ) {
                "ayt-cog1-biyo-ekosistem"
            } else if includes_any(
                &text,
                &["nüfus", "yerleşme", "şehirlerin fonksiyonları", "şehir"],
            ) {
                "ayt-cog1-nufus-politikasi-sehirler"
            } else if includes_any(&text, &["doğal kaynaklar", "tarım", "hayvancılık"]) {
                "ayt-cog1-turkiye-dogal-kaynaklar-ekonomi"
            } else if includes_any(&text, &["maden", "enerji kaynakları", "sanayi"]) {
                "ayt-cog1-turkiye-maden-enerji-sanayi"
            } else if includes_any(&text, &["bölgesel kalkınma", "gap", "dap", "dokap", "kop"]) {
                "ayt-cog1-turkiye-bolgesel-projeler"
            } else if includes_any(
                &text,
                &["ticaret", "ulaşım", "turizm", "boğazlar", "kanallar"],
            ) {
                "ayt-cog1-kuresel-ticaret-ve-ulasim"
            } else if includes_any(
                &text,
                &["çevre", "afet", "küresel ısınma", "sürdürülebilir"],
            ) {
                "ayt-cog1-cevre-afetler-surdurulebilirlik"
            } else {
                return None;
            }
        }
        "tarih2" => {
            if includes_any(
                &text,
                &[
                    "tarih metodolojisi",
                    "felsefesi",
                    "kaynak",
                    "tarihsel bilgi",
                ],
            ) {
                "ayt-tar2-tarih-metodolojisi"
            } else if includes_any(
                &text,
                &[
                    "eski çağ",
                    "töre",
                    "kut",
                    "kurultay",
                    "ilk türk devletlerinde",
                ],
            ) {
                "ayt-tar2-eski-cag-ve-turk-devlet-teskilati"
            } else if includes_any(&text, &["orta çağ", "skolastik", "feodal"]) {
                "ayt-tar2-orta-cag-avrupa-ve-islam"
            } else if includes_any(&text, &["divan-ı hümayun", "tımar", "iltizam", "eyalet"]) {
                "ayt-tar2-osmanli-siyaset-ve-teskilat"
            } else if includes_any(
                &text,
                &[
                    "millet sistemi",
                    "lonca",
                    "ahilik",
                    "vakıf",
                    "klasik dönemindeki ekonomik",
                ],
            ) {
                "ayt-tar2-osmanli-toplum-hukuk-ekonomi"
            } else if includes_any(
                &text,
                &["rönesans", "reform", "aydınlanma", "sanayi devrimi"],
            ) {
                "ayt-tar2-avrupada-donusum-ve-aydinlanma"
            } else if includes_any(
                &text,
                &[
                    "sened-i ittifak",
                    "tanzimat fermanı",
                    "kanun-i esasi",
                    "idari, askeri ve ekonomik",
                ],
            ) {
                "ayt-tar2-osmanli-modernlesme-kurumlari"
            } else if includes_any(
                &text,
                &[
                    "londra konferansı",
                    "moskova antlaşması",
                    "lozan görüşmeleri",
                ],
            ) {
                "ayt-tar2-milli-mucadele-diplomasi"
            } else if includes_any(
                &text,
                &[
                    "iki savaş arası",
                    "milletler cemiyeti",
                    "1929",
                    "faşizm",
                    "nazizm",
                ],
            ) {
                "ayt-tar2-iki-savas-arasi-dunya"
            } else if includes_any(
                &text,
                &[
                    "ikinci dünya savaşı",
                    "mihver",
                    "müttefik",
                    "yalta",
                    "potsdam",
                ],
            ) {
                "ayt-tar2-ikinci-dunya-savasi"
            } else if includes_any(
                &text,
                &[
                    "soğuk savaş",
                    "truman",
                    "marshall",
                    "nato",
                    "varşova",
                    "kore",
                ],
            ) {
                "ayt-tar2-soguk-savas-donemi"
            } else if includes_any(
                &text,
                &["yumuşama", "küba krizi", "vietnam", "kıbrıs barış harekâtı"],
            ) {
                "ayt-tar2-yumusama-donemi-ve-krizler"
            } else if includes_any(
                &text,
                &[
                    "küreselleşme",
                    "sscb'nin dağılması",
                    "bosna",
                    "avrupa birliği",
                ],
            ) {
                "ayt-tar2-kuresellesen-dunya-ve-turkiye"
            } else {
                return None;
            }
        }
        "cografya2" => {
            if includes_any(
                &text,
                &[
                    "ekosistem",
                    "besin zinciri",
                    "azot döngüsü",
                    "karbon döngüsü",
                ],
            ) {
                "ayt-cog2-ekosistem-ve-madde-dongusu"
            } else if includes_any(
                &text,
                &[
                    "ilk medeniyetler",
                    "nüfus",
                    "şehirleşme",
                    "yerleşme",
                    "şehir",
                ],
            ) {
                "ayt-cog2-ilk-medeniyet-ve-sehirler"
            } else if includes_any(&text, &["tarım", "hayvancılık", "sulama"]) {
                "ayt-cog2-turkiye-tarim-ve-hayvancilik"
            } else if includes_any(&text, &["maden", "enerji", "petrol", "kömür"]) {
                "ayt-cog2-turkiye-maden-ve-enerji"
            } else if includes_any(&text, &["sanayi", "ulaşım hatları", "ticaret"]) {
                "ayt-cog2-turkiye-sanayi-ve-ulasim"
            } else if includes_any(&text, &["bölgesel kalkınma", "gap", "dap", "dokap", "kop"]) {
                "ayt-cog2-bolgesel-kalkinma-projeleri"
            } else if includes_any(&text, &["jeopolitik", "türk dünyası"]) {
                "ayt-cog2-jeopolitik-ve-turk-dunyasi"
            } else if includes_any(
                &text,
                &["örgüt", "birleşmiş milletler", "nato", "opec", "ab"],
            ) {
                "ayt-cog2-kuresel-ve-bolgesel-orgutler"
            } else if includes_any(&text, &["çatışma", "orta doğu", "su sorunu", "hazar"]) {
                "ayt-cog2-uluslararasi-catisma-alanlari"
            } else if includes_any(
                &text,
                &[
                    "çevre kirliliği",
                    "atık",
                    "erozyon",
                    "çölleşme",
                    "geri dönüşüm",
                ],
            ) {
                "ayt-cog2-cevre-sorunlari-ve-atiklar"
            } else if includes_any(
                &text,
                &["sera gazı", "kyoto", "paris antlaşması", "küresel iklim"],
            ) {
                "ayt-cog2-iklim-ve-surdurulebilirlik"
            } else {
                return None;
            }
        }
        "felsefe_grubu" => {
            if includes_any(
                &text,
                &[
                    "varlık felsefesi",
                    "bilgi felsefesi",
                    "ahlak felsefesi",
                    "sanat felsefesi",
                    "siyaset felsefesi",
                    "felsefi soru",
                ],
            ) {
                "ayt-fel-felsefeye-giris-ve-temel-alanlar"
            } else if includes_any(
                &text,
                &[
                    "ilk çağ felsefesi",
                    "orta çağ felsefesi",
                    "sokrates",
                    "platon",
                    "aristoteles",
                ],
            ) {
                "ayt-fel-ilk-cag-ve-orta-cag"
            } else if includes_any(
                &text,
                &[
                    "modern felsefe",
                    "akılcılık",
                    "deneycilik",
                    "kant",
                    "hegel",
                    "varoluşçuluk",
                ],
            ) {
                "ayt-fel-modern-ve-cagdas-felsefe"
            } else if includes_any(
                &text,
                &[
                    "psikolojinin yöntemleri",
                    "davranışçılık",
                    "psikanaliz",
                    "bilişsel yaklaşım",
                    "psikoloji araştırma",
                ],
            ) {
                "ayt-psi-psikolojiye-giris-ve-yaklasimlar"
            } else if includes_any(
                &text,
                &["duyum", "algı", "illüzyon", "halüsinasyon", "dikkat"],
            ) {
                "ayt-psi-bilissel-surecler-ve-algilama"
            } else if includes_any(
                &text,
                &[
                    "öğrenme",
                    "klasik koşullanma",
                    "edimsel koşullanma",
                    "bellek",
                    "unutma",
                ],
            ) {
                "ayt-psi-ogrenme-bellek-dusunme"
            } else if includes_any(&text, &["gelişim", "piaget", "erikson", "kişilik", "freud"]) {
                "ayt-psi-gelisim-ve-kisilik"
            } else if includes_any(
                &text,
                &[
                    "sosyal psikoloji",
                    "tutum",
                    "stres",
                    "savunma mekanizmaları",
                    "ruh sağlığı",
                ],
            ) {
                "ayt-psi-sosyal-davranis-ve-ruh-sagligi"
            } else if includes_any(
                &text,
                &["sosyolojinin alanı", "sosyolojik yöntem", "kültür"],
            ) {
                "ayt-sos-sosyolojiye-giris-ve-yontem"
            } else if includes_any(&text, &["toplumsal yapı", "statü", "rol", "değer", "norm"]) {
                "ayt-sos-toplumsal-yapi-ve-etkilesim"
            } else if includes_any(&text, &["toplumsal değişme", "modernleşme", "küreselleşme"])
            {
                "ayt-sos-toplumsal-degisme-ve-kuresellesme"
            } else if includes_any(
                &text,
                &["toplumsal kurum", "aile", "eğitim", "ekonomi", "din kurumu"],
            ) {
                "ayt-sos-toplumsal-kurumlar"
            } else if includes_any(
                &text,
                &[
                    "tabakalaşma",
                    "toplumsal sınıf",
                    "hareketlili",
                    "sapma",
                    "kontrol",
                ],
            ) {
                "ayt-sos-toplumsal-tabakalasma-ve-sapma"
            } else if includes_any(
                &text,
                &["kavram", "terim", "içlem", "kaplam", "beş tümel", "tanım"],
            ) {
                "ayt-man-klasik-mantik-kavramlar"
            } else if includes_any(&text, &["önerme", "karşı olum", "kıyas"]) {
                "ayt-man-onermeler-ve-kiyas"
            } else if includes_any(&text, &["mantık ve dil", "anlam", "safsata"]) {
                "ayt-man-mantik-ve-dil"
            } else if includes_any(
                &text,
                &[
                    "önermeler mantığı",
                    "doğruluk tablosu",
                    "tutarlılık",
                    "geçerlilik",
                ],
            ) {
                "ayt-man-sembolik-onermeler-mantigi"
            } else if includes_any(&text, &["yüklemler mantığı", "niceleyici", "evren"]) {
                "ayt-man-sembolik-yuklemler-mantigi"
            } else {
                return None;
            }
        }
        "din" => {
            if includes_any(
                &text,
                &[
                    "inanç", "iman", "allah", "melek", "kitap", "kader", "ahiret", "tevhit",
                ],
            ) {
                "ayt-din-inanc-ve-esaslari"
            } else if includes_any(&text, &["ibadet", "namaz", "oruç", "zekat", "hac", "dua"]) {
                "ayt-din-ibadet-ve-manevi-hayat"
            } else if includes_any(&text, &["hz. muhammed", "peygamber", "sünnet", "örnek"]) {
                "ayt-din-hz-muhammed-ve-ornek-sahasi"
            } else if includes_any(&text, &["kur'an", "kuran", "insan", "sorumluluk"]) {
                "ayt-din-kuran-ve-insan"
            } else if includes_any(
                &text,
                &["bilim", "akıl", "felsefe", "müslüman bilim insanları"],
            ) {
                "ayt-din-islam-ve-bilim-dusuncesi"
            } else if includes_any(
                &text,
                &[
                    "ahlak",
                    "tasavvuf",
                    "ihsan",
                    "takva",
                    "mevlana",
                    "yunus emre",
                ],
            ) {
                "ayt-din-islamda-ahlak-ve-tasavvuf"
            } else if includes_any(
                &text,
                &["mezhep", "maturidilik", "eş'arilik", "hanefi", "şafii"],
            ) {
                "ayt-din-mezhepler-ve-dini-yorumlar"
            } else if includes_any(
                &text,
                &[
                    "yaşayan din",
                    "yahudilik",
                    "hristiyanlık",
                    "budizm",
                    "hinduizm",
                ],
            ) {
                "ayt-din-yasayan-dunya-dinleri"
            } else {
                return None;
            }
        }
        _ => return None,
    };
    Some(topic)
}

fn is_html(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm")
        })
}

fn parse_net(value: &str, label: &str) -> Result<f64, Box<dyn Error>> {
    value
        .trim()
        .replace(',', ".")
        .parse::<f64>()
        .map_err(|_| format!("could not read net for '{label}': '{}'", value.trim()).into())
}

fn parse_count(value: &str, label: &str, what: &str) -> Result<i64, Box<dyn Error>> {
    let number = value.trim().replace(',', ".").parse::<f64>().ok();
    match number {
        Some(number) if number >= 0.0 && number.fract() == 0.0 => Ok(number as i64),
        _ => Err(format!("could not read {what} for '{label}': '{}'", value.trim()).into()),
    }
}

/// Lowercases Turkish text so substring rules behave: `İ` -> `i` and `I` -> `ı`
/// (Rust's `to_lowercase` would turn `İ` into `i` plus a combining dot and `I` into `i`),
/// `î`/`û` lose their circumflex ("Edebî" -> "edebi") and curly apostrophes become `'`.
/// `â` is kept on purpose so that "kâr" (profit) stays distinct from "kar".
/// Rule needles must be written in this folded form.
fn fold(text: &str) -> String {
    text.replace("I\u{307}", "i")
        .replace('İ', "i")
        .replace('I', "ı")
        .to_lowercase()
        .replace('\u{307}', "")
        .replace('î', "i")
        .replace('û', "u")
        .replace(['’', '‘'], "'")
}

fn element_text(element: ElementRef<'_>) -> String {
    element.text().collect::<String>().trim().to_owned()
}

fn cell_text(cell: ElementRef<'_>, spans: &Selector) -> String {
    let span_text: Vec<String> = cell.select(spans).map(element_text).collect();
    if span_text.len() > 1 {
        span_text[0].clone()
    } else {
        element_text(cell)
    }
}

fn date_from_filename(path: &Path) -> Result<String, Box<dyn Error>> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("invalid HTML filename")?;
    let bytes = name.as_bytes();
    for start in 0..bytes.len().saturating_sub(9) {
        let candidate = &bytes[start..start + 10];
        if candidate[2] == b'.'
            && candidate[5] == b'.'
            && candidate
                .iter()
                .enumerate()
                .all(|(index, byte)| index == 2 || index == 5 || byte.is_ascii_digit())
        {
            let day = std::str::from_utf8(&candidate[0..2])?.parse::<u32>()?;
            let month = std::str::from_utf8(&candidate[3..5])?.parse::<u32>()?;
            let year = std::str::from_utf8(&candidate[6..10])?.parse::<i32>()?;
            let date = NaiveDate::from_ymd_opt(year, month, day).ok_or("invalid rehearsal date")?;
            return Ok(date.format("%Y-%m-%d").to_string());
        }
    }
    Err(format!(
        "Could not determine rehearsal date from filename: {}",
        path.display()
    )
    .into())
}

fn resolve_date(path: &Path, date_override: Option<&str>) -> Result<String, Box<dyn Error>> {
    match date_override {
        Some(value) => Ok(NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map_err(|_| format!("invalid --date value (expected YYYY-MM-DD): {value}"))?
            .format("%Y-%m-%d")
            .to_string()),
        None => date_from_filename(path),
    }
}

fn archived_filename(path: &Path, date: &str) -> Result<String, Box<dyn Error>> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("invalid HTML filename")?;
    if date_from_filename(path).is_ok() {
        return Ok(name.to_owned());
    }
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| format!("invalid rehearsal date: {date}"))?
        .format("%d.%m.%Y")
        .to_string();
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("invalid HTML filename")?;
    match path.extension().and_then(|value| value.to_str()) {
        Some(extension) => Ok(format!("{stem}_{date}.{extension}")),
        None => Ok(format!("{stem}_{date}")),
    }
}

fn copy_to_results(
    source: &Path,
    results_dir: &Path,
    filename: &str,
) -> Result<(), Box<dyn Error>> {
    let destination = results_dir.join(filename);
    if destination.exists() {
        if fs::read(&destination)? == fs::read(source)? {
            return Ok(());
        }
        return Err(format!(
            "Refusing to overwrite a different file in results: {}",
            destination.display()
        )
        .into());
    }
    fs::copy(source, &destination)?;
    println!("Copied HTML file to results: {filename}");
    Ok(())
}

/// Returns the analysis file name and path, failing if it already belongs to a different
/// result file. Called before anything is copied so a refused import leaves no trace.
fn analysis_target(
    analysis_dir: &Path,
    analysis: &Value,
) -> Result<(String, PathBuf), Box<dyn Error>> {
    let filename = format!(
        "{}-{}.json",
        analysis["date"].as_str().unwrap_or_default(),
        slug(analysis["exam_name"].as_str().unwrap_or_default())
    );
    let target = analysis_dir.join(&filename);
    if target.exists() {
        let existing = read_json(&target)?;
        if existing.get("source_file") != analysis.get("source_file") {
            return Err(format!(
                "Analysis {filename} already exists for a different result file ({}). \
                 Analyses are named after the rehearsal date and the exam name inside the HTML, \
                 so two different results with the same date and exam name would overwrite \
                 each other. Pass a different --date, or move the old analysis and its results/ \
                 file away.",
                existing
                    .get("source_file")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
            )
            .into());
        }
    }
    Ok((filename, target))
}

fn write_analysis(analysis_dir: &Path, analysis: &Value) -> Result<(), Box<dyn Error>> {
    let (filename, target) = analysis_target(analysis_dir, analysis)?;
    let mut contents = serde_json::to_vec_pretty(analysis)?;
    contents.push(b'\n');
    atomic_write(&target, &contents)?;
    println!("Created analysis file: {filename}");
    Ok(())
}

const ELECTIVE_PHILOSOPHY: &str = "Felsefe (Seçmeli)";

/// Summary-table row label -> subject id, for TYT exports. Several rows may feed one subject.
const TYT_SUMMARY_LABELS: &[(&str, &str)] = &[
    ("TYT Türkçe", "turkce"),
    ("Tarih-1", "tarih"),
    ("Coğrafya-1", "cografya"),
    ("Felsefe", "felsefe"),
    ("Din Kül. ve Ahl. Bil.", "din"),
    (ELECTIVE_PHILOSOPHY, "felsefe"),
    ("Matematik-1", "matematik"),
    ("Geometri", "matematik"),
    ("Fizik", "fizik"),
    ("Kimya", "kimya"),
    ("Biyoloji", "biyoloji"),
];

/// Same for AYT exports. Türkçe + Edebiyat together make up the 24 Türk Dili ve Edebiyatı
/// questions; Felsefe, Felsefe Grubu and the elective together make up Felsefe Grubu.
const AYT_SUMMARY_LABELS: &[(&str, &str)] = &[
    ("Türk Dili ve Edebiyatı", "edebiyat"),
    ("Türkçe", "edebiyat"),
    ("Edebiyat", "edebiyat"),
    ("Tarih-1", "tarih1"),
    ("Coğrafya-1", "cografya1"),
    ("Tarih-2", "tarih2"),
    ("Coğrafya-2", "cografya2"),
    ("Felsefe", "felsefe_grubu"),
    ("Felsefe Grubu", "felsefe_grubu"),
    (ELECTIVE_PHILOSOPHY, "felsefe_grubu"),
    ("Din Kül. ve Ahl. Bil.", "din"),
    ("Matematik-2", "matematik"),
    ("Geometri", "matematik"),
    ("Fizik", "fizik"),
    ("Kimya", "kimya"),
    ("Biyoloji", "biyoloji"),
];

struct SummaryRow {
    label: String,
    subject: &'static str,
    questions: i64,
    counts: Counts,
    skipped: bool,
}

fn parse_html_exam(
    path: &Path,
    exam: Exam,
    canonical_topics: &HashSet<String>,
    date: &str,
    source_file: &str,
) -> Result<Value, Box<dyn Error>> {
    let html = fs::read_to_string(path)?;
    let document = Html::parse_document(&html);
    let tables_selector = Selector::parse("table").expect("valid table selector");
    let row_selector = Selector::parse("tr").expect("valid row selector");
    let cell_selector = Selector::parse("th, td").expect("valid cell selector");
    let td_selector = Selector::parse("td").expect("valid td selector");
    let span_selector = Selector::parse("span").expect("valid span selector");
    let mut tables = Vec::new();
    let mut detail_tables = Vec::new();
    for table in document.select(&tables_selector) {
        let mut rows = Vec::new();
        let mut detail_rows = Vec::new();
        for row in table.select(&row_selector) {
            rows.push(
                row.select(&cell_selector)
                    .map(|cell| cell_text(cell, &span_selector))
                    .collect::<Vec<_>>(),
            );
            detail_rows.push(
                row.select(&td_selector)
                    .map(|cell| cell_text(cell, &span_selector))
                    .collect::<Vec<_>>(),
            );
        }
        tables.push(rows);
        detail_tables.push(detail_rows);
    }
    let first_table = tables.first().ok_or("HTML result contains no tables")?;

    let exam_anchor_selector = Selector::parse("a.fw-bold").expect("valid anchor selector");
    let keywords = ["ENDEMİK", "3D", "PLE", "EIS", "POZİTİF", "DENEME", "TG"];
    let mut exam_name = document
        .select(&exam_anchor_selector)
        .map(element_text)
        .find(|text| keywords.iter().any(|keyword| text.contains(keyword)))
        .unwrap_or_default();
    if exam_name.is_empty() {
        let heading_selector = Selector::parse("div, h2, h3, h4").expect("valid heading selector");
        exam_name = document
            .select(&heading_selector)
            .map(element_text)
            .find(|text| {
                text.len() < 80 && keywords[..5].iter().any(|keyword| text.contains(keyword))
            })
            .unwrap_or_default();
    }
    if exam_name.is_empty() {
        exam_name = match exam {
            Exam::Tyt => "TYT Deneme",
            Exam::Ayt => "AYT Deneme",
        }
        .into();
    }
    exam_name = exam_name.trim().to_owned();

    let labels: &[(&str, &str)] = match exam {
        Exam::Tyt => TYT_SUMMARY_LABELS,
        Exam::Ayt => AYT_SUMMARY_LABELS,
    };
    let subject_names: &[&str] = match exam {
        Exam::Tyt => &[
            "turkce",
            "matematik",
            "fizik",
            "kimya",
            "biyoloji",
            "tarih",
            "cografya",
            "felsefe",
            "din",
        ],
        Exam::Ayt => &[
            "matematik",
            "fizik",
            "kimya",
            "biyoloji",
            "edebiyat",
            "tarih1",
            "cografya1",
            "tarih2",
            "cografya2",
            "felsefe_grubu",
            "din",
        ],
    };

    // Summary table: one row per subject (in page order), plus subtotal rows that are ignored.
    let mut subject_rows: Vec<SummaryRow> = Vec::new();
    let mut page_total_net: Option<f64> = None;
    for row in first_table {
        let Some(label) = row.first().map(|cell| cell.trim()) else {
            continue;
        };
        if label.is_empty() || label == "Ders" || label.ends_with("Toplamı:") {
            continue;
        }
        if label.contains("Toplam:") {
            if row.len() >= 6 {
                page_total_net = Some(parse_net(&row[5], label)?);
            }
            continue;
        }
        let Some((_, subject)) = labels.iter().find(|(name, _)| *name == label) else {
            if row.len() >= 6 {
                eprintln!(
                    "Warning: unrecognized row '{label}' in the summary table of {source_file}; its results are not counted."
                );
            }
            continue;
        };
        if row.len() < 6 {
            return Err(format!(
                "summary row '{label}' has {} cells, expected 6 (Ders, Soru, Doğru, Yanlış, Boş, Net)",
                row.len()
            )
            .into());
        }
        let counts = Counts {
            correct: parse_count(&row[2], label, "correct")?,
            wrong: parse_count(&row[3], label, "wrong")?,
            blank: parse_count(&row[4], label, "blank")?,
        };
        // An elective philosophy block that was left entirely blank was simply not taken.
        let skipped = label == ELECTIVE_PHILOSOPHY && counts.correct + counts.wrong == 0;
        subject_rows.push(SummaryRow {
            label: label.to_owned(),
            subject,
            questions: parse_count(&row[1], label, "question count")?,
            counts,
            skipped,
        });
    }
    if subject_rows.is_empty() {
        return Err(format!(
            "no known subject rows found in the first table of {source_file}; the export layout is not recognised"
        )
        .into());
    }

    let mut totals: HashMap<&str, Counts> = HashMap::new();
    for row in subject_rows.iter().filter(|row| !row.skipped) {
        let entry = totals.entry(row.subject).or_default();
        entry.correct += row.counts.correct;
        entry.wrong += row.counts.wrong;
        entry.blank += row.counts.blank;
    }
    let net_of = |counts: &Counts| {
        ((counts.correct as f64 - counts.wrong as f64 / 4.0) * 100.0).round() / 100.0
    };
    let mut sections = Map::new();
    let mut computed_net = 0.0;
    for subject in subject_names {
        let counts = totals.get(subject).copied().unwrap_or_default();
        computed_net += net_of(&counts);
        sections.insert(
            (*subject).to_owned(),
            json!({
                "correct": counts.correct,
                "wrong": counts.wrong,
                "blank": counts.blank,
                "net": net_of(&counts)
            }),
        );
    }
    computed_net = (computed_net * 100.0).round() / 100.0;
    let total_net = match (exam, page_total_net) {
        // The AYT "Toplam:" row also counts sections that are not part of the student's track,
        // so the AYT total is always the sum of the imported sections.
        (Exam::Ayt, _) => computed_net,
        (Exam::Tyt, Some(page)) => {
            if (page - computed_net).abs() > 0.01 {
                eprintln!(
                    "Warning: {source_file}: page total net {page} differs from the sum of imported sections {computed_net}."
                );
            }
            page
        }
        (Exam::Tyt, None) => {
            eprintln!("Warning: {source_file}: no 'Toplam:' row found; using the sum of sections.");
            computed_net
        }
    };

    // Per-question tables follow the summary table in the same order as its subject rows.
    let detail: Vec<&Vec<Vec<String>>> = tables
        .iter()
        .zip(&detail_tables)
        .skip(1)
        .filter(|(rows, _)| {
            rows.first().is_some_and(|header| {
                header
                    .iter()
                    .any(|cell| cell == "Kazanım" || cell == "D.C.")
            })
        })
        .map(|(_, details)| details)
        .collect();
    let expected: Vec<&SummaryRow> = subject_rows
        .iter()
        .filter(|row| row.questions > 0)
        .collect();
    if detail.is_empty() {
        return Err(format!(
            "no per-question tables (Soru / D.C. / Ö.C. / Kazanım) found in {source_file}"
        )
        .into());
    }
    if detail.len() != expected.len() {
        return Err(format!(
            "{source_file}: found {} per-question tables but {} subject rows with questions; the export layout is not recognised",
            detail.len(),
            expected.len()
        )
        .into());
    }

    let mut question_map = Vec::new();
    for (row, rows) in expected.iter().zip(detail) {
        let data_rows: Vec<Vec<String>> = rows
            .iter()
            .filter(|cells| cells.len() >= 4)
            .map(|cells| cells.iter().map(|cell| cell.trim().to_owned()).collect())
            .collect();
        if data_rows.len() as i64 != row.questions {
            return Err(format!(
                "{source_file}: '{}' has {} questions in the summary table but {} in its per-question table",
                row.label,
                row.questions,
                data_rows.len()
            )
            .into());
        }
        let mut seen = Counts::default();
        let mut mapped_rows = Vec::new();
        for (index, cells) in data_rows.iter().enumerate() {
            let (question, correct_answer, student_answer, outcome) = if cells.len() == 4 {
                (index as i64 + 1, &cells[1], &cells[2], &cells[3])
            } else {
                let question = cells[0].parse::<i64>().unwrap_or(index as i64 + 1);
                (question, &cells[2], &cells[3], &cells[4])
            };
            let outcome = outcome.trim();
            let outcome = if outcome.starts_with('[') {
                outcome
                    .find(']')
                    .map(|end| outcome[end + 1..].trim())
                    .unwrap_or(outcome)
            } else {
                outcome
            }
            .to_owned();
            let result = if student_answer == correct_answer {
                "correct"
            } else if student_answer.trim().is_empty() || student_answer.trim() == "-" {
                "blank"
            } else {
                "wrong"
            };
            match result {
                "correct" => seen.correct += 1,
                "wrong" => seen.wrong += 1,
                _ => seen.blank += 1,
            }
            let mapped = match exam {
                Exam::Tyt => map_outcome(row.subject, &outcome),
                Exam::Ayt => map_ayt_outcome(row.subject, &outcome).map(str::to_owned),
            };
            let topic_id = mapped.filter(|id| canonical_topics.contains(id));
            mapped_rows.push(json!({
                "question":question,
                "subject_id":row.subject,
                "topic_id":topic_id,
                "kazanim":outcome,
                "correct_answer":correct_answer,
                "student_answer":student_answer,
                "result":result
            }));
        }
        if seen.correct != row.counts.correct
            || seen.wrong != row.counts.wrong
            || seen.blank != row.counts.blank
        {
            eprintln!(
                "Warning: {source_file}: '{}' answers give {}/{}/{} (correct/wrong/blank) but the summary row says {}/{}/{}.",
                row.label,
                seen.correct,
                seen.wrong,
                seen.blank,
                row.counts.correct,
                row.counts.wrong,
                row.counts.blank
            );
        }
        if !row.skipped {
            question_map.extend(mapped_rows);
        }
    }

    Ok(json!({
        "date":date,
        "source_file":source_file,
        "exam_name":exam_name,
        "sections":sections,
        "total_net":total_net,
        "question_map":question_map,
        "assumptions":[],
        "raw_notes":format!("Auto-imported from Okulizyon HTML export on {date}.")
    }))
}

/// Questions without a topic are left out of the topic statistics, so say so.
fn report_unmapped(analysis: &Value) {
    let questions = analysis
        .get("question_map")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let unmapped: Vec<&Value> = questions
        .iter()
        .filter(|question| question.get("topic_id").is_none_or(Value::is_null))
        .collect();
    if unmapped.is_empty() {
        return;
    }
    let mut examples: Vec<String> = Vec::new();
    for question in &unmapped {
        let example = format!(
            "{}: '{}'",
            question["subject_id"].as_str().unwrap_or("?"),
            question["kazanim"].as_str().unwrap_or("?")
        );
        if !examples.contains(&example) && examples.len() < 6 {
            examples.push(example);
        }
    }
    eprintln!(
        "Warning: {} of {} questions in {} have no topic mapping and are left out of the topic statistics, e.g. {}",
        unmapped.len(),
        questions.len(),
        analysis["source_file"].as_str().unwrap_or("?"),
        examples.join("; ")
    );
}

fn topic_ids(data: &Value) -> HashSet<String> {
    data.get("subjects")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|subject| {
            subject
                .get("topics")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|topic| topic.get("id").and_then(Value::as_str).map(str::to_owned))
        .collect()
}

fn update_topics(data: &mut Value, analyses: &[Value]) {
    if let Some(subjects) = data.get_mut("subjects").and_then(Value::as_array_mut) {
        for subject in subjects {
            if let Some(topics) = subject.get_mut("topics").and_then(Value::as_array_mut) {
                for topic in topics {
                    topic["question_total"] = json!(0);
                    topic["correct_total"] = json!(0);
                    topic["wrong_total"] = json!(0);
                    topic["blank_total"] = json!(0);
                    topic["last_tested"] = Value::Null;
                    topic["last_result"] = Value::Null;
                }
            }
        }
    }

    let mut sorted: Vec<&Value> = analyses.iter().collect();
    sorted.sort_by_key(|analysis| analysis.get("date").and_then(Value::as_str).unwrap_or(""));
    for analysis in sorted {
        let date = analysis.get("date").and_then(Value::as_str).unwrap_or("");
        let exam_name = analysis
            .get("exam_name")
            .and_then(Value::as_str)
            .unwrap_or("");
        let mut topic_results: HashMap<String, Counts> = HashMap::new();
        for question in analysis
            .get("question_map")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(topic_id) = question.get("topic_id").and_then(Value::as_str) else {
                continue;
            };
            let counts = topic_results.entry(topic_id.to_owned()).or_default();
            match question.get("result").and_then(Value::as_str) {
                Some("correct") => counts.correct += 1,
                Some("wrong") => counts.wrong += 1,
                Some("blank") => counts.blank += 1,
                _ => {}
            }
        }
        let Some(subjects) = data.get_mut("subjects").and_then(Value::as_array_mut) else {
            continue;
        };
        for subject in subjects {
            let Some(topics) = subject.get_mut("topics").and_then(Value::as_array_mut) else {
                continue;
            };
            for topic in topics {
                let Some(topic_id) = topic.get("id").and_then(Value::as_str) else {
                    continue;
                };
                let Some(stats) = topic_results.get(topic_id) else {
                    continue;
                };
                let total = stats.correct + stats.wrong + stats.blank;
                topic["question_total"] =
                    json!(topic["question_total"].as_i64().unwrap_or(0) + total);
                topic["correct_total"] =
                    json!(topic["correct_total"].as_i64().unwrap_or(0) + stats.correct);
                topic["wrong_total"] =
                    json!(topic["wrong_total"].as_i64().unwrap_or(0) + stats.wrong);
                topic["blank_total"] =
                    json!(topic["blank_total"].as_i64().unwrap_or(0) + stats.blank);
                topic["last_tested"] = json!(date);
                topic["last_result"] = json!({
                    "exam":exam_name,
                    "date":date,
                    "correct":stats.correct,
                    "wrong":stats.wrong,
                    "blank":stats.blank
                });
            }
        }
    }
}

fn slug(value: &str) -> String {
    let mut result = String::new();
    let mut previous_dash = false;
    for character in value.chars().take(25) {
        if character.is_ascii_alphanumeric() {
            result.push(character.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash && !result.is_empty() {
            result.push('-');
            previous_dash = true;
        }
    }
    result.trim_matches('-').to_owned()
}

pub fn import(
    tracker: &Tracker,
    date_override: Option<&str>,
    file: Option<&Path>,
) -> Result<usize, Box<dyn Error>> {
    let results_dir = tracker.directory.join("results");
    let analysis_dir = tracker.directory.join("analysis");
    let valid_ids = topic_ids(&tracker.topics);
    let mut analyses = Vec::new();
    match file {
        Some(path) => {
            if !path.is_file() {
                return Err(format!("HTML file not found: {}", path.display()).into());
            }
            if !is_html(path) {
                return Err(format!("Not an HTML file: {}", path.display()).into());
            }
            let date = resolve_date(path, date_override)?;
            let filename = archived_filename(path, &date)?;
            let source_file = format!("results/{filename}");
            let analysis = parse_html_exam(path, tracker.exam, &valid_ids, &date, &source_file)?;
            analysis_target(&analysis_dir, &analysis)?;
            copy_to_results(path, &results_dir, &filename)?;
            write_analysis(&analysis_dir, &analysis)?;
            report_unmapped(&analysis);
            analyses.push(analysis);
        }
        None => {
            let mut paths: Vec<PathBuf> = fs::read_dir(&results_dir)?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| is_html(path))
                .collect();
            paths.sort();
            if paths.is_empty() {
                return Err(format!(
                    "No HTML result files found in {}. Pass one with --file <path>.",
                    results_dir.display()
                )
                .into());
            }
            for path in paths {
                let date = resolve_date(&path, date_override)?;
                let name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default();
                let source_file = format!("results/{name}");
                let analysis =
                    parse_html_exam(&path, tracker.exam, &valid_ids, &date, &source_file)?;
                write_analysis(&analysis_dir, &analysis)?;
                report_unmapped(&analysis);
                analyses.push(analysis);
            }
        }
    }
    let mut all_analyses = Vec::new();
    let mut analysis_files: Vec<PathBuf> = fs::read_dir(&analysis_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect();
    analysis_files.sort();
    for path in analysis_files {
        all_analyses.push(read_json(&path)?);
    }
    let topics_path = tracker.directory.join("topics.json");
    let mut topics = read_json(&topics_path)?;
    update_topics(&mut topics, &all_analyses);
    let mut contents = serde_json::to_vec_pretty(&topics)?;
    contents.push(b'\n');
    atomic_write(&topics_path, &contents)?;
    println!("Updated topics.json successfully");
    Ok(analyses.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_imported_outcomes_to_known_tyt_topics() {
        let canonical: Value = serde_json::from_str(include_str!("../topics/tyt.json")).unwrap();
        let valid_ids = topic_ids(&canonical);
        for (subject, outcome, expected) in [
            ("turkce", "[1] Sözcükte Anlam", "turkce-sozcuk-anlam"),
            (
                "matematik",
                "Dik üçgende Öklid bağıntıları",
                "geometri-dik-ucgen-oklid",
            ),
            ("tarih", "Tarih ve Zaman", "tarih-bilimi"),
            ("fizik", "Manyetizma", "fizik-manyetizma"),
            (
                "biyoloji",
                "Zarından madde geçişi",
                "biyoloji-hucre-zari-madde-gecisi",
            ),
        ] {
            let mapped = map_outcome(subject, outcome).unwrap();
            assert_eq!(mapped, expected);
            assert!(
                valid_ids.contains(&mapped),
                "unknown canonical topic ID: {mapped}"
            );
        }
        assert_eq!(map_outcome("unknown", "unknown"), None);
    }

    #[test]
    fn maps_imported_outcomes_to_known_ayt_topics() {
        let canonical: Value = serde_json::from_str(include_str!("../topics/ayt.json")).unwrap();
        let valid_ids = topic_ids(&canonical);
        let cases = [
            (
                "matematik",
                "[1] Asal Sayılar[1] Asal Çarpanlara Ayırma",
                "ayt-mat-carpanlara-ayirma",
            ),
            (
                "matematik",
                "[1] Mutlak Değerli Denklemler",
                "ayt-mat-esitsizlik-mutlak",
            ),
            (
                "matematik",
                "[1] Çarpanlara Ayırma[1] Köklü ifadeler",
                "ayt-mat-carpanlara-ayirma",
            ),
            (
                "matematik",
                "[1] Sayı Basamakları",
                "ayt-mat-sayi-basamaklari",
            ),
            ("matematik", "[1] KÜMELER[1] KÜMELER", "ayt-mat-kumeler"),
            (
                "matematik",
                "[1] Fonksiyonlarda Bileşke İşlemi",
                "ayt-mat-fonksiyonlar",
            ),
            (
                "matematik",
                "[1] Kök- Katsayı İlişkisi ve Kökleri Verilen Denklemi Bulma",
                "ayt-mat-ikinci-derece-denklemler",
            ),
            (
                "matematik",
                "[1] İkinci dereceden bir bilinmeyenli eşitsizlik sistemlerinin çözüm kümesini bulur.",
                "ayt-mat-ikinci-derece-esitsizlikler",
            ),
            (
                "matematik",
                "[1] Koşullu önermeyi ve iki yönlü koşullu önermeyi açıklar.",
                "ayt-mat-mantik",
            ),
            (
                "matematik",
                "[1] Aritmetik ve geometrik dizilerin özelliklerini kullanarak işlemler yapar.",
                "ayt-mat-diziler",
            ),
            (
                "matematik",
                "[1] Fonksiyon Grafikleri[1] Eşitsizlik Sistemleri ve Eşitsizlik Sistemlerinin Grafikle Çözümü",
                "ayt-mat-fonksiyonlar",
            ),
            (
                "matematik",
                "[1] Parabol ile İlgili Uygulamalar",
                "ayt-mat-parabol",
            ),
            (
                "matematik",
                "[1] Logaritma Fonksiyonunun Özellikleri",
                "ayt-mat-logaritma",
            ),
            (
                "matematik",
                "[1] Üstel ve Logaritmik Fonksiyonları Modelleme",
                "ayt-mat-logaritma",
            ),
            (
                "matematik",
                "[1] Polinomların Çarpanlara Ayrılması",
                "ayt-mat-polinomlar",
            ),
            (
                "matematik",
                "[1] Permütasyon[1] Kombinasyon",
                "ayt-mat-permutasyon-kombinasyon",
            ),
            (
                "matematik",
                "[1] Pascal Üçgeni ve Binom Açılımı",
                "ayt-mat-binom",
            ),
            ("matematik", "[1] Koşullu Olasılık", "ayt-mat-olasilik"),
            ("matematik", "[1] 0/0 Belirsizliği", "ayt-mat-limit"),
            ("matematik", "[1] Süreklilik", "ayt-mat-sureklilik"),
            (
                "matematik",
                "[1] Türev Kuralları[1] Bileşke Fonksiyonunun Türevi",
                "ayt-mat-turev",
            ),
            (
                "matematik",
                "[1] Minimum-Maksimum Problemleri",
                "ayt-mat-turev",
            ),
            (
                "matematik",
                "[1] Türevin Fiziksel Anlamı ve Türevin Geometrik Yorumu[1] Artanlık-Azalanlık-Ekstremum Noktalar",
                "ayt-mat-turev",
            ),
            (
                "matematik",
                "[1] İntegral ile Alan Hesabı",
                "ayt-mat-integral",
            ),
            (
                "matematik",
                "[1] Belirli İntegral[1] Değişken Değiştirme Yöntemiyle İntegral Alma",
                "ayt-mat-integral",
            ),
            (
                "matematik",
                "[1] Birim çemberi tanımlar ve trigonometrik oranları birim çemberin üzerindeki noktanın koordinatlarıyla ilişkilendirir.[1] Dik Üçgende Trigonometrik Oranlar",
                "ayt-mat-trigonometri",
            ),
            (
                "matematik",
                "[1] Trigonometrik Fonksiyonlar ve Özdeşlikler",
                "ayt-mat-trigonometri",
            ),
            ("matematik", "[1] Kosinüs Teoremi", "ayt-mat-trigonometri"),
            (
                "matematik",
                "[1] Toplam-Fark Formülleri[1] Trigonometrik Denklemler",
                "ayt-mat-trigonometri",
            ),
            (
                "matematik",
                "[1] Trigonometrik Denklemler",
                "ayt-mat-trigonometri",
            ),
            ("matematik", "[1] Üçgende Alan", "ayt-geo-ucgende-alan"),
            ("matematik", "[1] Çemberde Açı", "ayt-geo-cember-daire"),
            (
                "matematik",
                "[1] Dairede Çevre ve Alan",
                "ayt-geo-cember-daire",
            ),
            (
                "matematik",
                "[1] Dönüşüm Geometrisi",
                "ayt-geo-donusum-vektor",
            ),
            (
                "matematik",
                "[1] Noktanın Analitik İncelenmesi",
                "ayt-geo-analitik",
            ),
            (
                "matematik",
                "[1] Doğrunun Analitik İncelenmesi",
                "ayt-geo-analitik",
            ),
            (
                "matematik",
                "[1] Çemberin Analitik İncelenmesi",
                "ayt-geo-analitik",
            ),
            ("matematik", "[1] Silindir", "ayt-geo-kati-cisimler"),
            ("fizik", "[1] Eğik Atış Hareketi", "ayt-fizik-hareket"),
            (
                "fizik",
                "[1] Newton'ın Hareket Yasaları",
                "ayt-fizik-newton",
            ),
            (
                "fizik",
                "[1] Yüklü, iletken ve paralel levhalar arasında oluşan elektrik alanının bağlı olduğu değişkenleri analiz eder.",
                "ayt-fizik-elektrostatik",
            ),
            (
                "fizik",
                "[1] Tork kavramını açıklar.",
                "ayt-fizik-tork-kutle-merkezi",
            ),
            (
                "fizik",
                "[1] Öteleme kinetik enerjisi, yer çekimi potansiyel enerjisi ve esneklik potansiyel enerjisinin bağlı olduğu değişkenleri analiz eder.",
                "ayt-fizik-is-guc-enerji",
            ),
            (
                "fizik",
                "[1] Alternatif akımı açıklar.",
                "ayt-fizik-alternatif-akim",
            ),
            (
                "fizik",
                "[1] Düzgün çembersel harekette merkezcil kuvvetin bağlı olduğu değişkenleri analiz eder.",
                "ayt-fizik-cembersel-hareket",
            ),
            (
                "fizik",
                "[1] Açısal momentumu çizgisel momentum ile ilişkilendirerek açıklar.",
                "ayt-fizik-cembersel-hareket",
            ),
            (
                "fizik",
                "[1] Doppler olayının etkilerini ışık ve ses dalgalarından örneklerle açıklar.",
                "ayt-fizik-dalgalar",
            ),
            (
                "fizik",
                "[1] Elektromanyetik dalgaların ortak özelliklerini açıklar.",
                "ayt-fizik-dalgalar",
            ),
            (
                "fizik",
                "[1] Üzerinden akım geçen iletken düz bir telin çevresinde, halkanın merkezinde ve akım makarasının merkez ekseninde oluşan manyetik alan ile ilgili hesaplamalar yapar.",
                "ayt-fizik-manyetizma",
            ),
            (
                "fizik",
                "[1] Modern atom teorisinin önemini açıklar.",
                "ayt-fizik-modern-fizik",
            ),
            (
                "fizik",
                "[1] Basit harmonik harekette kuvvet, hız ve ivmenin konuma göre değişimi ile ilgili hesaplamalar yapar.",
                "ayt-fizik-basit-harmonik-hareket",
            ),
            (
                "fizik",
                "[1] Fotoelektronların sahip olduğu maksimum kinetik enerji, durdurma gerilimi ve metalin eşik enerjisi arasındaki matematiksel ilişkiyi açıklar.",
                "ayt-fizik-modern-fizik",
            ),
            ("kimya", "[1] İdeal Gaz Yasası", "ayt-kimya-gazlar"),
            (
                "kimya",
                "[1] Redoks tepkimelerini tanır.",
                "ayt-kimya-elektrokimya",
            ),
            (
                "kimya",
                "[1] Standart Oluşum Entalpileriyle Tepkime Entalpisi Hesaplamaları",
                "ayt-kimya-termokimya",
            ),
            ("kimya", "[1] Çözeltilerde Derişim", "ayt-kimya-cozeltiler"),
            (
                "kimya",
                "[1] Atomu kuantum modeliyle açıklar.",
                "ayt-kimya-kuantum-modeli",
            ),
            (
                "kimya",
                "[1] Elektroliz olayını elektrik akımı, zaman ve değişime uğrayan madde kütlesi açısından açıklar.",
                "ayt-kimya-elektrokimya",
            ),
            (
                "kimya",
                "[1] Zayıf Asit ve Baz Çözeltilerinde pH",
                "ayt-kimya-asit-baz-denge",
            ),
            (
                "kimya",
                "[1] Tepkime hızına etki eden faktörleri açıklar.",
                "ayt-kimya-hiz",
            ),
            (
                "kimya",
                "[1] Dengeye etki eden faktörleri Le Chatelier ilkesi ile ilişkilendirir.",
                "ayt-kimya-denge",
            ),
            (
                "kimya",
                "[1] Elektrot ve elektrokimyasal hücre kavramlarını açıklar.",
                "ayt-kimya-elektrokimya",
            ),
            (
                "kimya",
                "[1] Organik bileşikleri fonksiyonel gruplarına göre sınıflandırır.",
                "ayt-kimya-fonksiyonel-gruplar",
            ),
            (
                "kimya",
                "[1] Hidrokarbonlar / Alkenler",
                "ayt-kimya-hidrokarbonlar",
            ),
            (
                "kimya",
                "[1] Moleküllerin geometrilerini merkez atomu orbitallerinin hibritleşmesi esasına göre belirler.",
                "ayt-kimya-etkilesimler",
            ),
            (
                "biyoloji",
                "[1] Bağışıklık (Doğal Bağışıklık, Kazanılmış Bağışıklık, Aktif ve Pasif Bağışıklık)",
                "ayt-biyo-bagisiklik",
            ),
            (
                "biyoloji",
                "[1] Böbreğin Boşaltım Dışı Görevleri",
                "ayt-biyo-bosaltim",
            ),
            ("biyoloji", "[1] Kas Çeşitleri", "ayt-biyo-destek-hareket"),
            (
                "biyoloji",
                "[1] Sindirim organlarının görev ve işleyişini açıklar.",
                "ayt-biyo-sindirim",
            ),
            (
                "biyoloji",
                "[1] Duyu organlarının yapısını ve işleyişini açıklar.",
                "ayt-biyo-duyu-organlari",
            ),
            (
                "biyoloji",
                "[1] Komünitede türler arasında simbiyotik ilişkileri örneklerle açıklar.",
                "ayt-biyo-ekosistem",
            ),
            (
                "biyoloji",
                "[1] Popülasyon dinamiğini açıklar.",
                "ayt-biyo-ekosistem",
            ),
            (
                "biyoloji",
                "[1] Bitkilerde su ve mineral taşınma mekanizmasını açıklar.",
                "ayt-biyo-bitki-tasima-ureme",
            ),
            (
                "biyoloji",
                "[1] Fotosentez hızını etkileyen faktörleri değerlendirir.",
                "ayt-biyo-fotosentez-solunum",
            ),
            (
                "biyoloji",
                "[1] Genetik mühendisliği ve biyoteknoloji uygulamalarını açıklar.",
                "ayt-biyo-biyoteknoloji",
            ),
            (
                "biyoloji",
                "[1] Fermantasyon (Laktik Asit Fermantasyonu, Etil Alkol Fermantasyonu)",
                "ayt-biyo-fotosentez-solunum",
            ),
            (
                "biyoloji",
                "[1] Nükleik asitlerin çeşitlerini ve görevlerini açıklar.",
                "ayt-biyo-molekuler-kalitim",
            ),
            (
                "edebiyat",
                "Divan edebiyatında gazel nazım biçimi ve özellikleri",
                "ayt-edebiyat-divan-nazim",
            ),
            (
                "edebiyat",
                "Tanzimat birinci dönem sanatçılarının edebi özellikleri",
                "ayt-edebiyat-tanzimat-donemi",
            ),
            (
                "edebiyat",
                "İkinci Yeni şiirinin temsilcileri ve poetikası",
                "ayt-edebiyat-cumhuriyet-siir-ikinci-yeni",
            ),
            (
                "tarih1",
                "İlk Türk-İslam devletlerinde Karahanlılar ve Gazneliler",
                "ayt-tar1-turklerin-islamiyeti-kabulu",
            ),
            (
                "tarih1",
                "Millî Mücadele Dönemi Batı Cephesi muharebeleri ve Sakarya",
                "ayt-tar1-milli-mucadele-muharebeler",
            ),
            (
                "cografya1",
                "Biyoçeşitlilik ve karasal biyomların dağılışı",
                "ayt-cog1-biyo-ekosistem",
            ),
            (
                "cografya1",
                "Türkiye'de bölgesel kalkınma projeleri ve GAP hedefleri",
                "ayt-cog1-turkiye-bolgesel-projeler",
            ),
            (
                "tarih2",
                "Soğuk Savaş Döneminde Truman Doktrini ve NATO'nun kuruluşu",
                "ayt-tar2-soguk-savas-donemi",
            ),
            (
                "tarih2",
                "Osmanlı klasik döneminde divan-ı hümayun ve taşra teşkilatı",
                "ayt-tar2-osmanli-siyaset-ve-teskilat",
            ),
            (
                "cografya2",
                "Küresel ve bölgesel örgütler: Birleşmiş Milletler ve NATO",
                "ayt-cog2-kuresel-ve-bolgesel-orgutler",
            ),
            (
                "cografya2",
                "Türkiye'nin jeopolitik konumu ve Türk dünyası ile ilişkileri",
                "ayt-cog2-jeopolitik-ve-turk-dunyasi",
            ),
            (
                "felsefe_grubu",
                "Klasik mantıkta kıyas kuralları ve önermeler",
                "ayt-man-onermeler-ve-kiyas",
            ),
            (
                "felsefe_grubu",
                "Bilişsel psikolojide algı ve duyum süreçleri",
                "ayt-psi-bilissel-surecler-ve-algilama",
            ),
            (
                "felsefe_grubu",
                "Toplumsal tabakalaşma ve sosyal hareketlilik türleri",
                "ayt-sos-toplumsal-tabakalasma-ve-sapma",
            ),
            (
                "din",
                "İslam düşüncesinde itikadi mezhepler: Maturidilik ve Eş'arilik",
                "ayt-din-mezhepler-ve-dini-yorumlar",
            ),
            (
                "din",
                "İslam ahlak esasları ve tasavvufi düşüncede Mevlana",
                "ayt-din-islamda-ahlak-ve-tasavvuf",
            ),
        ];
        for (subject, outcome, expected) in cases {
            let mapped = map_ayt_outcome(subject, outcome)
                .unwrap_or_else(|| panic!("unmapped AYT outcome: {outcome}"));
            assert_eq!(mapped, expected, "wrong mapping for: {outcome}");
            assert!(
                valid_ids.contains(mapped),
                "unknown canonical topic ID: {mapped}"
            );
        }
        assert_eq!(map_ayt_outcome("unknown", "unknown"), None);
        assert_eq!(map_ayt_outcome("biyoloji", "unrelated outcome"), None);
    }

    #[test]
    fn every_remap_target_exists_in_tyt_topics() {
        let canonical: Value = serde_json::from_str(include_str!("../topics/tyt.json")).unwrap();
        let valid_ids = topic_ids(&canonical);
        let targets = LEGACY_REMAP
            .iter()
            .map(|(_, target)| target)
            .chain(LEGACY_REMAP_IF.iter().map(|(_, _, target)| target));
        for target in targets {
            assert!(valid_ids.contains(*target), "missing topic id: {target}");
        }
    }

    #[test]
    fn remap_sources_are_not_topic_ids() {
        // A topic id that is also a remap source can never receive data.
        let canonical: Value = serde_json::from_str(include_str!("../topics/tyt.json")).unwrap();
        let valid_ids = topic_ids(&canonical);
        let sources = LEGACY_REMAP
            .iter()
            .map(|(source, _)| source)
            .chain(LEGACY_REMAP_IF.iter().map(|(source, _, _)| source));
        for source in sources {
            assert!(!valid_ids.contains(*source), "shadowed topic id: {source}");
        }
    }

    #[test]
    fn conditional_remap_wins_over_default() {
        assert_eq!(
            canonical_topic_id("cografya-nufus-ve-yerlesme", "Kır yerleşmeleri").unwrap(),
            "cografya-yerlesme"
        );
        assert_eq!(
            canonical_topic_id("cografya-nufus-ve-yerlesme", "Nüfus piramitleri").unwrap(),
            "cografya-nufus"
        );
    }

    #[test]
    fn slug_matches_analysis_file_convention() {
        assert_eq!(slug("3D TYT Denemesi"), "3d-tyt-denemesi");
        assert_eq!(slug("DENEME / A"), "deneme-a");
    }

    #[test]
    fn archives_undated_files_with_the_rehearsal_date() {
        assert_eq!(
            archived_filename(Path::new("/tmp/result.html"), "2026-11-10").unwrap(),
            "result_10.11.2026.html"
        );
        assert_eq!(
            archived_filename(
                Path::new("/tmp/YKS Sınav Sonucu _ Okulizyon.html"),
                "2026-11-10"
            )
            .unwrap(),
            "YKS Sınav Sonucu _ Okulizyon_10.11.2026.html"
        );
        assert_eq!(
            archived_filename(Path::new("/tmp/result_03.11.2026.html"), "2026-11-10").unwrap(),
            "result_03.11.2026.html"
        );
    }

    // ---- Tests against anonymized tables from real Okulizyon exports (tests/fixtures) ----

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    fn canonical_topics(exam: Exam) -> HashSet<String> {
        let json = match exam {
            Exam::Tyt => include_str!("../topics/tyt.json"),
            Exam::Ayt => include_str!("../topics/ayt.json"),
        };
        topic_ids(&serde_json::from_str(json).unwrap())
    }

    fn parse_fixture(name: &str, exam: Exam) -> Value {
        parse_html_exam(
            &fixture(name),
            exam,
            &canonical_topics(exam),
            "2026-11-09",
            "results/fixture.html",
        )
        .unwrap()
    }

    fn section(analysis: &Value, subject: &str) -> (i64, i64, i64, f64) {
        let section = &analysis["sections"][subject];
        (
            section["correct"].as_i64().unwrap(),
            section["wrong"].as_i64().unwrap(),
            section["blank"].as_i64().unwrap(),
            section["net"].as_f64().unwrap(),
        )
    }

    fn unmapped(analysis: &Value) -> usize {
        analysis["question_map"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|question| question["topic_id"].is_null())
            .count()
    }

    fn topic_of(analysis: &Value, outcome: &str) -> String {
        analysis["question_map"]
            .as_array()
            .unwrap()
            .iter()
            .find(|question| question["kazanim"] == outcome)
            .unwrap_or_else(|| panic!("no question with outcome {outcome}"))["topic_id"]
            .as_str()
            .unwrap_or("<unmapped>")
            .to_owned()
    }

    /// Every section's counts must equal what the per-question answers say.
    fn assert_sections_match_questions(analysis: &Value) {
        let mut seen: HashMap<(String, String), i64> = HashMap::new();
        for question in analysis["question_map"].as_array().unwrap() {
            *seen
                .entry((
                    question["subject_id"].as_str().unwrap().to_owned(),
                    question["result"].as_str().unwrap().to_owned(),
                ))
                .or_default() += 1;
        }
        for (subject, section) in analysis["sections"].as_object().unwrap() {
            let has_questions = analysis["question_map"]
                .as_array()
                .unwrap()
                .iter()
                .any(|question| question["subject_id"] == subject.as_str());
            if !has_questions {
                continue;
            }
            for result in ["correct", "wrong", "blank"] {
                assert_eq!(
                    seen.get(&(subject.clone(), result.to_owned()))
                        .copied()
                        .unwrap_or(0),
                    section[result].as_i64().unwrap(),
                    "{subject} {result}"
                );
            }
        }
    }

    #[test]
    fn imports_tyt_export_completely() {
        let analysis = parse_fixture("tyt-genel-deneme.html", Exam::Tyt);
        assert_eq!(
            analysis["exam_name"],
            "2026-27 PLE - YKS -TYT - Genel Deneme 01"
        );
        assert_eq!(analysis["question_map"].as_array().unwrap().len(), 120);
        assert_eq!(unmapped(&analysis), 0);
        assert_eq!(analysis["total_net"].as_f64().unwrap(), 79.5);
        assert_eq!(section(&analysis, "turkce"), (34, 4, 2, 33.0));
        assert_eq!(section(&analysis, "matematik"), (22, 1, 17, 21.75));
        assert_sections_match_questions(&analysis);
    }

    #[test]
    fn untaken_elective_philosophy_is_not_counted() {
        // "Felsefe (Seçmeli)" was left entirely blank: no extra felsefe questions or blanks.
        let analysis = parse_fixture("tyt-genel-deneme.html", Exam::Tyt);
        let felsefe = analysis["question_map"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|question| question["subject_id"] == "felsefe")
            .count();
        assert_eq!(felsefe, 5);
        assert_eq!(section(&analysis, "felsefe"), (5, 0, 0, 5.0));
    }

    #[test]
    fn imports_tyt_export_with_uppercase_outcomes() {
        let analysis = parse_fixture("tyt-tg.html", Exam::Tyt);
        assert_eq!(analysis["question_map"].as_array().unwrap().len(), 120);
        assert_eq!(unmapped(&analysis), 0);
        assert_eq!(analysis["total_net"].as_f64().unwrap(), 44.75);
        assert_sections_match_questions(&analysis);
        // ALL-CAPS outcomes used to fall through to catch-all topics.
        assert_eq!(
            topic_of(&analysis, "ISI VE SICAKLIK"),
            "fizik-isi-sicaklik-genlesme"
        );
        assert_eq!(topic_of(&analysis, "OLASILIK"), "matematik-olasilik");
        assert_eq!(
            topic_of(&analysis, "PARAGRAFIN YAPISI"),
            "turkce-paragraf-yapi"
        );
    }

    #[test]
    fn imports_sayisal_ayt_export() {
        let analysis = parse_fixture("ayt-sayisal.html", Exam::Ayt);
        assert_eq!(analysis["question_map"].as_array().unwrap().len(), 80);
        assert_eq!(unmapped(&analysis), 0);
        assert_eq!(section(&analysis, "matematik"), (7, 7, 26, 5.25));
        // Matematik 5.25 + Fen Bilimleri 19.50; the page's own "Toplam:" row is not usable.
        assert_eq!(analysis["total_net"].as_f64().unwrap(), 24.75);
        assert_sections_match_questions(&analysis);
    }

    #[test]
    fn imports_ayt_export_with_social_sections_by_table_order() {
        // 16 tables: Türkçe, Edebiyat, Tarih, ... come before Matematik. Tables must be paired
        // with summary rows by order, not by fixed index.
        let analysis = parse_fixture("ayt-full.html", Exam::Ayt);
        // 166 questions minus the untaken elective philosophy block.
        assert_eq!(analysis["question_map"].as_array().unwrap().len(), 160);
        assert_sections_match_questions(&analysis);
        // Türkçe (5/1) + Edebiyat (2/16) feed one Türk Dili ve Edebiyatı section.
        assert_eq!(section(&analysis, "edebiyat"), (7, 17, 0, 2.75));
        // Felsefe (1/5) + Felsefe Grubu (0/6) feed one Felsefe Grubu section.
        assert_eq!(section(&analysis, "felsefe_grubu"), (1, 11, 0, -1.75));
        let net = |subjects: &[&str]| -> f64 {
            subjects
                .iter()
                .map(|s| section(&analysis, s).3)
                .sum::<f64>()
        };
        // Subtotals printed on the page: Edebiyat-Sosyal-1 = 12.50, Sosyal-2 = -5.00.
        assert_eq!(net(&["edebiyat", "tarih1", "cografya1"]), 12.5);
        assert_eq!(net(&["tarih2", "cografya2", "felsefe_grubu", "din"]), -5.0);
        assert_eq!(analysis["total_net"].as_f64().unwrap(), 15.25);
        assert!(
            unmapped(&analysis) <= 17,
            "unmapped: {}",
            unmapped(&analysis)
        );
        assert_eq!(
            topic_of(&analysis, "Edebî Sanatlar"),
            "ayt-edebiyat-edebi-sanatlar"
        );
        assert_eq!(
            topic_of(&analysis, "Divan Edebiyatında Nazım Şekilleri"),
            "ayt-edebiyat-divan-nazim"
        );
    }

    fn write_temp(name: &str, contents: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("yksresults-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        fs::write(&path, contents).unwrap();
        path
    }

    fn parse_str(name: &str, html: &str, exam: Exam) -> Result<Value, Box<dyn Error>> {
        let path = write_temp(name, html);
        parse_html_exam(
            &path,
            exam,
            &canonical_topics(exam),
            "2026-11-09",
            "results/x.html",
        )
    }

    #[test]
    fn rejects_exports_without_question_tables() {
        let html = "<table><tr><th>Ders</th></tr>\
                    <tr><td>TYT Türkçe</td><td>40</td><td>30</td><td>5</td><td>5</td><td>28,75</td></tr></table>";
        let error = parse_str("no-details.html", html, Exam::Tyt).unwrap_err();
        assert!(
            error.to_string().contains("no per-question tables"),
            "{error}"
        );
    }

    #[test]
    fn rejects_unreadable_numbers_instead_of_importing_zeros() {
        let html = "<table><tr><td>TYT Türkçe</td><td>40</td><td>abc</td><td>5</td><td>5</td><td>1</td></tr></table>";
        let error = parse_str("bad-number.html", html, Exam::Tyt).unwrap_err();
        assert!(error.to_string().contains("could not read"), "{error}");
    }

    #[test]
    fn rejects_unrecognised_layouts() {
        let html = "<table><tr><td>Something else</td><td>1</td><td>1</td><td>0</td><td>0</td><td>1</td></tr></table>";
        let error = parse_str("unknown.html", html, Exam::Ayt).unwrap_err();
        assert!(error.to_string().contains("not recognised"), "{error}");
    }

    #[test]
    fn rejects_misaligned_question_tables() {
        // Drop the last per-question table: 4 tables for 5 subject rows.
        let html = fs::read_to_string(fixture("ayt-sayisal.html")).unwrap();
        let last = html.rfind("<table>").unwrap();
        let end = html.rfind("</table>").unwrap() + "</table>".len();
        let broken = format!("{}{}", &html[..last], &html[end..]);
        let error = parse_str("misaligned.html", &broken, Exam::Ayt).unwrap_err();
        assert!(error.to_string().contains("per-question tables"), "{error}");
    }

    #[test]
    fn fold_handles_turkish_case_rules() {
        assert_eq!(fold("İSİM"), "isim");
        assert_eq!(fold("SAYILAR"), "sayılar");
        assert_eq!(fold("Işık"), "ışık");
        assert_eq!(fold("Edebî Sanatlar"), "edebi sanatlar");
        assert_eq!(fold("Osmanlı Devleti’nin"), "osmanlı devleti'nin");
        assert_eq!(fold("Kâr"), "kâr");
        assert_eq!(fold("i\u{307}"), "i".to_owned() + "");
    }

    #[test]
    fn tyt_rules_see_through_dotted_capital_i() {
        assert_eq!(
            map_legacy_outcome("turkce", "İsim tamlaması"),
            map_legacy_outcome("turkce", "isim tamlaması")
        );
        assert_eq!(
            map_legacy_outcome("turkce", "İsim tamlaması"),
            Some("turkce-sozcuk-turleri")
        );
    }

    #[test]
    fn word_class_outcomes_are_not_swallowed_by_word_meaning() {
        assert_eq!(
            map_legacy_outcome("turkce", "Sözcük türleri: isim, sıfat"),
            Some("turkce-sozcuk-turleri")
        );
        assert_eq!(
            map_legacy_outcome("turkce", "Sözcükte anlam"),
            Some("turkce-sozcuk-anlam")
        );
    }

    fn temp_tracker(name: &str, exam: Exam) -> Tracker {
        let directory =
            std::env::temp_dir().join(format!("yksresults-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("results")).unwrap();
        fs::create_dir_all(directory.join("analysis")).unwrap();
        let json = match exam {
            Exam::Tyt => include_str!("../topics/tyt.json"),
            Exam::Ayt => include_str!("../topics/ayt.json"),
        };
        let topics: Value = serde_json::from_str(json).unwrap();
        fs::write(
            directory.join("topics.json"),
            serde_json::to_vec(&topics).unwrap(),
        )
        .unwrap();
        Tracker {
            exam,
            directory,
            topics,
        }
    }

    fn total_questions(tracker: &Tracker) -> i64 {
        let topics = read_json(&tracker.directory.join("topics.json")).unwrap();
        topics["subjects"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|subject| subject["topics"].as_array().unwrap())
            .map(|topic| topic["question_total"].as_i64().unwrap())
            .sum()
    }

    fn dir_len(path: &Path) -> usize {
        fs::read_dir(path).unwrap().count()
    }

    #[test]
    fn import_is_idempotent() {
        let tracker = temp_tracker("idempotent", Exam::Tyt);
        let file = write_temp(
            "tyt-genel-deneme_03.11.2026.html",
            &fs::read_to_string(fixture("tyt-genel-deneme.html")).unwrap(),
        );
        assert_eq!(import(&tracker, None, Some(&file)).unwrap(), 1);
        assert_eq!(total_questions(&tracker), 120);
        // Importing the same file again, and rescanning, never double-counts.
        assert_eq!(import(&tracker, None, Some(&file)).unwrap(), 1);
        assert_eq!(import(&tracker, None, None).unwrap(), 1);
        assert_eq!(total_questions(&tracker), 120);
        assert_eq!(dir_len(&tracker.directory.join("results")), 1);
        assert_eq!(dir_len(&tracker.directory.join("analysis")), 1);
    }

    #[test]
    fn refused_import_leaves_results_directory_clean() {
        // Same exam name and date, different files: the second import must be refused
        // before its HTML is copied, otherwise every later rescan would fail too.
        let tracker = temp_tracker("collision", Exam::Ayt);
        let first = write_temp(
            "ayt-a_09.11.2026.html",
            &fs::read_to_string(fixture("ayt-sayisal.html")).unwrap(),
        );
        let second = write_temp(
            "ayt-b_09.11.2026.html",
            &fs::read_to_string(fixture("ayt-full.html")).unwrap(),
        );
        import(&tracker, None, Some(&first)).unwrap();
        let error = import(&tracker, None, Some(&second)).unwrap_err();
        assert!(error.to_string().contains("already exists"), "{error}");
        assert_eq!(dir_len(&tracker.directory.join("results")), 1);
        assert_eq!(dir_len(&tracker.directory.join("analysis")), 1);
        assert_eq!(import(&tracker, None, None).unwrap(), 1);
    }
}
