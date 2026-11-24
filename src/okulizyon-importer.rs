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
    let text = outcome.to_lowercase();
    let result = match subject_id {
        "turkce" => {
            if includes_any(
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
                    "merkezî eğilim",
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
                    "millî mücadele",
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
    let text = outcome.to_lowercase();
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
    // Turkish uppercase İ lowercases to "i" plus a combining dot, which would
    // break substring matches such as "İntegral" -> "integral".
    let text = outcome.to_lowercase().replace('\u{307}', "");
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
                    "millî edebiyat",
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
                &["dünya gücü", "yükselme", "fatih", "kanuni", "yavuz"],
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
            if includes_any(&text, &["tarih metodolojisi", "felsefesi", "kaynak"]) {
                "ayt-tar2-tarih-metodolojisi"
            } else if includes_any(&text, &["eski çağ", "töre", "kut", "kurultay"]) {
                "ayt-tar2-eski-cag-ve-turk-devlet-teskilati"
            } else if includes_any(&text, &["orta çağ", "skolastik", "feodal"]) {
                "ayt-tar2-orta-cag-avrupa-ve-islam"
            } else if includes_any(&text, &["divan-ı hümayun", "tımar", "iltizam", "eyalet"]) {
                "ayt-tar2-osmanli-siyaset-ve-teskilat"
            } else if includes_any(&text, &["millet sistemi", "lonca", "ahilik", "vakıf"]) {
                "ayt-tar2-osmanli-toplum-hukuk-ekonomi"
            } else if includes_any(
                &text,
                &["rönesans", "reform", "aydınlanma", "sanayi devrimi"],
            ) {
                "ayt-tar2-avrupada-donusum-ve-aydinlanma"
            } else if includes_any(
                &text,
                &["sened-i ittifak", "tanzimat fermanı", "kanun-i esasi"],
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
            } else if includes_any(&text, &["ilk medeniyetler", "nüfus", "şehirleşme"]) {
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
            } else if includes_any(&text, &["çevre kirliliği", "atık", "erozyon", "çölleşme"])
            {
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
                    "hareketlilik",
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
                    "inanç", "iman", "allah", "melek", "kitap", "kader", "ahiret",
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

fn parse_number(value: &str) -> f64 {
    value.trim().replace(',', ".").parse().unwrap_or(0.0)
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

fn write_analysis(analysis_dir: &Path, analysis: &Value) -> Result<(), Box<dyn Error>> {
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
                 Two exams with the same date and name would overwrite each other; \
                 rename one of the HTML files or move the old analysis away.",
                existing
                    .get("source_file")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
            )
            .into());
        }
    }
    let mut contents = serde_json::to_vec_pretty(analysis)?;
    contents.push(b'\n');
    atomic_write(&target, &contents)?;
    println!("Created analysis file: {filename}");
    Ok(())
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
    let mut sections = Map::new();
    for subject in subject_names {
        sections.insert(
            (*subject).to_owned(),
            json!({"correct":0,"wrong":0,"blank":0,"net":0.0}),
        );
    }
    let section_map: &[(&str, &str)] = match exam {
        Exam::Tyt => &[
            ("TYT Türkçe", "turkce"),
            ("Tarih-1", "tarih"),
            ("Coğrafya-1", "cografya"),
            ("Felsefe", "felsefe"),
            ("Din Kül. ve Ahl. Bil.", "din"),
            ("Matematik-1", "mat_raw"),
            ("Geometri", "geo_raw"),
            ("Fizik", "fizik"),
            ("Kimya", "kimya"),
            ("Biyoloji", "biyoloji"),
        ],
        Exam::Ayt => &[
            ("Türk Dili ve Edebiyatı", "edebiyat"),
            ("Tarih-1", "tarih1"),
            ("Coğrafya-1", "cografya1"),
            ("Matematik-2", "mat_raw"),
            ("Geometri", "geo_raw"),
            ("Fizik", "fizik"),
            ("Kimya", "kimya"),
            ("Biyoloji", "biyoloji"),
            ("Tarih-2", "tarih2"),
            ("Coğrafya-2", "cografya2"),
            ("Felsefe Grubu", "felsefe_grubu"),
            ("Din Kül. ve Ahl. Bil.", "din"),
        ],
    };
    let mut math = Counts::default();
    let mut geometry = Counts::default();
    let mut total_net = 0.0;
    for row in first_table {
        if row.len() < 6 {
            continue;
        }
        let label = row[0].as_str();
        let correct = parse_number(&row[2]) as i64;
        let wrong = parse_number(&row[3]) as i64;
        let blank = parse_number(&row[4]) as i64;
        let net = parse_number(&row[5]);
        if let Some((_, target)) = section_map.iter().find(|(name, _)| *name == label) {
            match *target {
                "mat_raw" => {
                    math = Counts {
                        correct,
                        wrong,
                        blank,
                    }
                }
                "geo_raw" => {
                    geometry = Counts {
                        correct,
                        wrong,
                        blank,
                    }
                }
                subject => {
                    sections.insert(
                        subject.to_owned(),
                        json!({
                            "correct":correct,
                            "wrong":wrong,
                            "blank":blank,
                            "net":((correct as f64 - wrong as f64 / 4.0) * 100.0).round() / 100.0
                        }),
                    );
                }
            }
        } else if label.contains("Toplam:") {
            total_net = net;
        }
    }
    let math_correct = math.correct + geometry.correct;
    let math_wrong = math.wrong + geometry.wrong;
    sections.insert(
        "matematik".into(),
        json!({
            "correct": math_correct,
            "wrong": math_wrong,
            "blank": math.blank + geometry.blank,
            "net": ((math_correct as f64 - math_wrong as f64 / 4.0) * 100.0).round() / 100.0
        }),
    );

    if exam == Exam::Ayt {
        total_net = [
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
        ]
        .iter()
        .filter_map(|subject| sections.get(*subject))
        .filter_map(|section| section.get("net"))
        .filter_map(Value::as_f64)
        .sum::<f64>();
        total_net = (total_net * 100.0).round() / 100.0;
    }

    let table_subjects: &[(usize, &str, i64)] = match exam {
        Exam::Tyt => &[
            (1usize, "turkce", 1i64),
            (2, "tarih", 1),
            (3, "cografya", 6),
            (4, "felsefe", 11),
            (5, "din", 16),
            (7, "matematik", 1),
            (8, "matematik", 31),
            (9, "fizik", 1),
            (10, "kimya", 8),
            (11, "biyoloji", 15),
        ],
        Exam::Ayt => &[
            (1usize, "matematik", 1i64),
            (2, "matematik", 31),
            (3, "fizik", 1),
            (4, "kimya", 15),
            (5, "biyoloji", 28),
        ],
    };
    let mut question_map = Vec::new();
    for &(table_index, subject_id, question_offset) in table_subjects {
        let Some(rows) = detail_tables.get(table_index) else {
            continue;
        };
        let mut question_number = question_offset;
        for row in rows.iter().skip(1) {
            let cells: Vec<String> = row.iter().map(|cell| cell.trim().to_owned()).collect();
            if cells.len() < 4 {
                continue;
            }
            let (question, correct_answer, student_answer, outcome) = if cells.len() == 4 {
                (question_number, &cells[1], &cells[2], &cells[3])
            } else {
                let question = cells[0].parse::<i64>().unwrap_or(question_number);
                (question, &cells[2], &cells[3], &cells[4])
            };
            question_number += 1;
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
            let mapped = match exam {
                Exam::Tyt => map_outcome(subject_id, &outcome),
                Exam::Ayt => map_ayt_outcome(subject_id, &outcome).map(str::to_owned),
            };
            let topic_id = mapped.filter(|id| canonical_topics.contains(id));
            question_map.push(json!({
                "question":question,
                "subject_id":subject_id,
                "topic_id":topic_id,
                "kazanim":outcome,
                "correct_answer":correct_answer,
                "student_answer":student_answer,
                "result":result
            }));
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
            copy_to_results(path, &results_dir, &filename)?;
            write_analysis(&analysis_dir, &analysis)?;
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
}
