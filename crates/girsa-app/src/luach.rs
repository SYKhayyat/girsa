//! What day it is, and what the world is learning today.
//!
//! # Why an application about seforim needs a calendar
//!
//! Girsa had no concept of *today*. Not the Hebrew date, not the daf — nothing
//! in 20,000 lines knew what day it was. And the single commonest reason a
//! person opens a Shas in the morning is the daf: Otzaria computes it from the
//! Hebrew date and opens the masechta, and that is the first thing a reader
//! coming from Otzaria looks for and does not find.
//!
//! It is also arithmetic. There is no corpus question here, no ambiguity to
//! surface, no reader decision — the daf on 16 August 2026 is one daf, and it
//! has been fixed since 1923.
//!
//! # Two calendars and one epoch
//!
//! The Hebrew date is the fixed calendar: the molad of Tishrei plus the four
//! dechiyos, exactly as in Reingold and Dershowitz and as in the Emacs
//! implementation of it that has been checked against printed luchos for thirty
//! years. Nothing here approximates. Both directions are exact integer
//! arithmetic on **RD** — the day number with RD 1 = 1 January 1 CE — so a
//! conversion round-trips or it is a bug, and [`tests`] asserts that it does
//! over a century.
//!
//! Daf Yomi does not need the Hebrew calendar at all, and that is worth saying
//! because Otzaria routes through one. The cycle is a fixed count of days from a
//! fixed morning, so it is RD arithmetic and nothing else:
//!
//! | | |
//! |---|---|
//! | first day of the first cycle | 11 September 1923 — Berachos ב' |
//! | cycles 1–7 | 2,702 dapim, Shekalim counted as 12 |
//! | first day of the eighth cycle | 24 June 1975 |
//! | cycles 8 onward | 2,711 dapim, Shekalim counted as 21 |
//!
//! Those two dates are 18,914 days apart, which is exactly seven cycles of
//! 2,702 — the arithmetic checks itself, and the test says so out loud. The
//! fourteenth cycle then falls on 5 January 2020, which is the date it actually
//! began.
//!
//! # The one thing this deliberately does not do
//!
//! The daf turns over at nightfall and this turns it over at midnight, because
//! nightfall is a function of where the reader is standing and Girsa does not
//! know and will not ask. Otzaria makes the same choice. What a reader gets
//! instead is [`Luach::tomorrow`], which is the daf they would be learning
//! tonight, said as such — an honest offer rather than a wrong answer that
//! looks right for four hours a day.

/// Days from 1 January 1 CE (Gregorian, proleptic), which is day 1.
///
/// The unit both calendars are converted through, so that neither one has to
/// know anything about the other.
pub type Rd = i64;

/// Days elapsed before RD 1, as the Hebrew calendar counts them.
///
/// The epoch itself — 1 Tishrei of year 1 — is RD −1,373,427, and this constant
/// is two less because [`elapsed_days`] returns a count that starts at 1.
const HEBREW_OFFSET: Rd = -1_373_429;

/// A day on the civil calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Civil {
    pub year: i32,
    /// 1–12.
    pub month: u32,
    pub day: u32,
}

/// A day on the Hebrew calendar.
///
/// Months are numbered from **Nisan**, which is how the Torah numbers them and
/// how every implementation of this algorithm numbers them: Nisan is 1, Tishrei
/// is 7, and in a leap year Adar I is 12 and Adar II is 13. A year therefore
/// begins in month 7 and the number goes down before it goes up, which looks
/// wrong and is right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Hebrew {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

/// Whether a Gregorian year is a leap year.
const fn gregorian_leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// The RD of a civil date.
#[must_use]
pub fn fixed_from_civil(date: Civil) -> Rd {
    let prior = Rd::from(date.year) - 1;
    let correction = if date.month <= 2 {
        0
    } else if gregorian_leap(date.year) {
        -1
    } else {
        -2
    };
    365 * prior + prior.div_euclid(4) - prior.div_euclid(100)
        + prior.div_euclid(400)
        + (367 * Rd::from(date.month) - 362).div_euclid(12)
        + correction
        + Rd::from(date.day)
}

/// The civil date of an RD.
#[must_use]
pub fn civil_from_fixed(rd: Rd) -> Civil {
    // The year, found by approximation and then corrected — the same shape as
    // the Hebrew conversion below, and for the same reason: there is no closed
    // form that survives the leap rules.
    #[allow(clippy::cast_possible_truncation)]
    let mut year = (rd.div_euclid(366) + 1) as i32;
    while fixed_from_civil(Civil {
        year: year + 1,
        month: 1,
        day: 1,
    }) <= rd
    {
        year += 1;
    }
    let mut month = 1;
    while fixed_from_civil(Civil {
        year,
        month: month + 1,
        day: 1,
    }) <= rd
        && month < 12
    {
        month += 1;
    }
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let day =
        (rd - fixed_from_civil(Civil {
            year,
            month,
            day: 1,
        }) + 1) as u32;
    Civil { year, month, day }
}

/// Whether a Hebrew year has thirteen months.
#[must_use]
pub const fn is_leap_year(year: i32) -> bool {
    (7 * year + 1).rem_euclid(19) < 7
}

/// The last month of a Hebrew year, counting from Nisan.
#[must_use]
pub const fn last_month_of_year(year: i32) -> u32 {
    if is_leap_year(year) {
        13
    } else {
        12
    }
}

/// Days elapsed from the epoch to 1 Tishrei of this year, plus one.
///
/// The molad of Tishrei and the four dechiyos. Every term here is doing
/// something a printed luach does:
///
/// * `19440` — molad zaken. A molad after noon pushes Rosh Hashanah a day.
/// * `9924` on a Tuesday of a common year — GaTaRaD, which would otherwise
///   force the year to 356 days, which no year may be.
/// * `16789` on a Monday after a leap year — BeTUTaKaPoT, the same problem at
///   the other end: a 382-day year.
/// * `{0, 3, 5}` — לא אד"ו ראש. Rosh Hashanah is never Sunday, Wednesday or
///   Friday.
fn elapsed_days(year: i32) -> Rd {
    let prior = Rd::from(year) - 1;
    let months = 235 * prior.div_euclid(19)
        + 12 * prior.rem_euclid(19)
        + (7 * prior.rem_euclid(19) + 1).div_euclid(19);
    let parts_elapsed = 204 + 793 * months.rem_euclid(1080);
    let hours_elapsed =
        5 + 12 * months + 793 * months.div_euclid(1080) + parts_elapsed.div_euclid(1080);
    let day = 1 + 29 * months + hours_elapsed.div_euclid(24);
    let parts = 1080 * hours_elapsed.rem_euclid(24) + parts_elapsed.rem_euclid(1080);
    let postponed = if parts >= 19440
        || (day.rem_euclid(7) == 2 && parts >= 9924 && !is_leap_year(year))
        || (day.rem_euclid(7) == 1 && parts >= 16789 && is_leap_year(year - 1))
    {
        day + 1
    } else {
        day
    };
    if matches!(postponed.rem_euclid(7), 0 | 3 | 5) {
        postponed + 1
    } else {
        postponed
    }
}

/// How many days a Hebrew year has — 353, 354, 355, 383, 384 or 385.
#[must_use]
pub fn days_in_year(year: i32) -> Rd {
    elapsed_days(year + 1) - elapsed_days(year)
}

/// The last day of a Hebrew month.
///
/// Cheshvan and Kislev are the two that move: a full year lengthens Cheshvan
/// and a deficient one shortens Kislev, which is what makes 353/354/355 three
/// year-lengths rather than one.
#[must_use]
pub fn last_day_of_month(month: u32, year: i32) -> u32 {
    match month {
        2 | 4 | 6 | 10 | 13 => 29,
        12 if !is_leap_year(year) => 29,
        8 if days_in_year(year).rem_euclid(10) != 5 => 29,
        9 if days_in_year(year).rem_euclid(10) == 3 => 29,
        _ => 30,
    }
}

/// The RD of a Hebrew date.
#[must_use]
pub fn fixed_from_hebrew(date: Hebrew) -> Rd {
    let months: Rd = if date.month < 7 {
        // A year runs Tishrei → Elul, so a month before Tishrei is at the far
        // end of it: every month from Tishrei to the end of the year, and then
        // from Nisan up to here.
        (7..=last_month_of_year(date.year))
            .chain(1..date.month)
            .map(|m| Rd::from(last_day_of_month(m, date.year)))
            .sum()
    } else {
        (7..date.month)
            .map(|m| Rd::from(last_day_of_month(m, date.year)))
            .sum()
    };
    Rd::from(date.day) + months + elapsed_days(date.year) + HEBREW_OFFSET
}

/// The Hebrew date of an RD.
#[must_use]
pub fn hebrew_from_fixed(rd: Rd) -> Hebrew {
    // 365.2468 days is the mean Hebrew year; the approximation is always at or
    // below the answer, and the loop walks it up.
    #[allow(clippy::cast_possible_truncation)]
    let mut year = ((rd - HEBREW_OFFSET) / 366) as i32;
    while fixed_from_hebrew(Hebrew {
        year: year + 1,
        month: 7,
        day: 1,
    }) <= rd
    {
        year += 1;
    }
    // Before 1 Nisan the month number is still in the high half of the year.
    let mut month = if rd
        < fixed_from_hebrew(Hebrew {
            year,
            month: 1,
            day: 1,
        }) {
        7
    } else {
        1
    };
    while rd
        > fixed_from_hebrew(Hebrew {
            year,
            month,
            day: last_day_of_month(month, year),
        })
    {
        month += 1;
    }
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let day =
        (rd - fixed_from_hebrew(Hebrew {
            year,
            month,
            day: 1,
        }) + 1) as u32;
    Hebrew { year, month, day }
}

/// What a Hebrew month is called, given whether its year has two Adars.
#[must_use]
pub fn month_name(month: u32, leap: bool) -> &'static str {
    match month {
        1 => "ניסן",
        2 => "אייר",
        3 => "סיון",
        4 => "תמוז",
        5 => "אב",
        6 => "אלול",
        7 => "תשרי",
        8 => "חשון",
        9 => "כסלו",
        10 => "טבת",
        11 => "שבט",
        12 if leap => "אדר א'",
        12 => "אדר",
        _ => "אדר ב'",
    }
}

/// Which day of the week an RD is — 0 is Sunday.
#[must_use]
pub const fn day_of_week(rd: Rd) -> u32 {
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    {
        rd.rem_euclid(7) as u32
    }
}

/// What that day of the week is called.
#[must_use]
pub const fn weekday_name(day: u32) -> &'static str {
    match day {
        0 => "יום ראשון",
        1 => "יום שני",
        2 => "יום שלישי",
        3 => "יום רביעי",
        4 => "יום חמישי",
        5 => "ערב שבת",
        _ => "שבת קודש",
    }
}

impl Hebrew {
    /// The date as it is written — `כ״ז אב תשפ״ו`.
    ///
    /// The year without its thousands, because that is how a Jew writes a year
    /// and has since the fifth millennium: `תשפ"ו`, not `ה'תשפ"ו`.
    #[must_use]
    pub fn said(&self) -> String {
        #[allow(clippy::cast_sign_loss)]
        let year = (self.year % 1000) as u32;
        format!(
            "{} {} {}",
            girsa_ref::numerals::to_hebrew(self.day),
            month_name(self.month, is_leap_year(self.year)),
            girsa_ref::numerals::to_hebrew(year)
        )
    }
}

/// One masechta, as Daf Yomi counts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Masechta {
    /// Where it is on this shelf — `bavli/berakhot`.
    pub slug: &'static str,
    /// What it is called.
    pub said: &'static str,
    /// The daf the cycle starts it on. Two is the usual answer; the last four
    /// masechtos share a pagination and do not start at two.
    pub first: u32,
    /// How many days it takes.
    pub dapim: u32,
    /// Whether this shelf addresses it by daf.
    ///
    /// Three of the forty are not: Shekalim is the Yerushalmi, and Kinnim and
    /// Middos are Mishnayos with no gemara, so what the corpus holds is
    /// addressed by perek and mishnah with the Vilna daf marked inside the
    /// text. The luach still names the daf — the reader is learning it — and
    /// says that opening it lands on the sefer rather than on the page.
    pub by_daf: bool,
}

/// The masechtos in the order the cycle takes them, and how long each takes.
///
/// Shekalim's count is the one that moved: for the first seven cycles it was
/// the four dapim of the Bavli's stub, twelve days; from 24 June 1975 the cycle
/// learns the Yerushalmi, twenty-one. That is the whole difference between a
/// 2,702-day cycle and a 2,711-day one.
///
/// The tail is the part that looks wrong and is right. Meilah, Kinnim, Tamid
/// and Middos share one continuous pagination in the Vilna Shas — Meilah runs
/// to 22, Kinnim from 23, Tamid from 26, Middos from 34 — so their `first` is
/// not 2 and their totals are 21, 3, 8 and 4.
const SHAS: &[Masechta] = &[
    m("bavli/berakhot", "ברכות", 2, 63),
    m("bavli/shabbat", "שבת", 2, 156),
    m("bavli/eruvin", "עירובין", 2, 104),
    m("bavli/pesachim", "פסחים", 2, 120),
    Masechta {
        slug: "yerushalmi/jerusalem-talmud-shekalim",
        said: "שקלים",
        first: 2,
        dapim: 21,
        by_daf: false,
    },
    m("bavli/yoma", "יומא", 2, 87),
    m("bavli/sukkah", "סוכה", 2, 55),
    m("bavli/beitzah", "ביצה", 2, 39),
    m("bavli/rosh-hashanah", "ראש השנה", 2, 34),
    m("bavli/taanit", "תענית", 2, 30),
    m("bavli/megillah", "מגילה", 2, 31),
    m("bavli/moed-katan", "מועד קטן", 2, 28),
    m("bavli/chagigah", "חגיגה", 2, 26),
    m("bavli/yevamot", "יבמות", 2, 121),
    m("bavli/ketubot", "כתובות", 2, 111),
    m("bavli/nedarim", "נדרים", 2, 90),
    m("bavli/nazir", "נזיר", 2, 65),
    m("bavli/sotah", "סוטה", 2, 48),
    m("bavli/gittin", "גיטין", 2, 89),
    m("bavli/kiddushin", "קידושין", 2, 81),
    m("bavli/bava-kamma", "בבא קמא", 2, 118),
    m("bavli/bava-metzia", "בבא מציעא", 2, 118),
    m("bavli/bava-batra", "בבא בתרא", 2, 175),
    m("bavli/sanhedrin", "סנהדרין", 2, 112),
    m("bavli/makkot", "מכות", 2, 23),
    m("bavli/shevuot", "שבועות", 2, 48),
    m("bavli/avodah-zarah", "עבודה זרה", 2, 75),
    m("bavli/horayot", "הוריות", 2, 13),
    m("bavli/zevachim", "זבחים", 2, 119),
    m("bavli/menachot", "מנחות", 2, 109),
    m("bavli/chullin", "חולין", 2, 141),
    m("bavli/bekhorot", "בכורות", 2, 60),
    m("bavli/arakhin", "ערכין", 2, 33),
    m("bavli/temurah", "תמורה", 2, 33),
    m("bavli/keritot", "כריתות", 2, 27),
    m("bavli/meilah", "מעילה", 2, 21),
    Masechta {
        slug: "mishnah-kinnim",
        said: "קינים",
        first: 23,
        dapim: 3,
        by_daf: false,
    },
    m("bavli/tamid", "תמיד", 26, 8),
    Masechta {
        slug: "mishnah-middot",
        said: "מדות",
        first: 34,
        dapim: 4,
        by_daf: false,
    },
    m("bavli/niddah", "נדה", 2, 72),
];

const fn m(slug: &'static str, said: &'static str, first: u32, dapim: u32) -> Masechta {
    Masechta {
        slug,
        said,
        first,
        dapim,
        by_daf: true,
    }
}

/// Where Shekalim sits in [`SHAS`], for the cycles that counted it short.
const SHEKALIM: usize = 4;
/// Shekalim before 24 June 1975: the Bavli's four dapim, twelve days.
const SHEKALIM_WAS: u32 = 12;

/// The first day of the first cycle — Berachos ב', 11 September 1923.
const FIRST: Civil = Civil {
    year: 1923,
    month: 9,
    day: 11,
};
/// The first day of the eighth cycle, which is where Shekalim grew.
const EIGHTH: Civil = Civil {
    year: 1975,
    month: 6,
    day: 24,
};

/// A day's daf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Daf {
    pub masechta: &'static Masechta,
    /// The daf, as the Shas numbers it.
    pub daf: u32,
    /// Which cycle, counting the one that began in 1923 as the first.
    pub cycle: u32,
    /// Which day of that cycle, from 1.
    pub day: u32,
}

impl Daf {
    /// The address to open, when this masechta is addressed by daf.
    ///
    /// Amud א', because that is where a daf begins and what a person means by
    /// *the daf*.
    #[must_use]
    pub fn address(&self) -> Option<String> {
        self.masechta.by_daf.then(|| format!("{}a", self.daf))
    }

    /// The mareh makom, said — `ברכות ב׳`.
    #[must_use]
    pub fn said(&self) -> String {
        format!(
            "{} {}",
            self.masechta.said,
            girsa_ref::numerals::to_hebrew(self.daf)
        )
    }
}

/// The Daf Yomi of a day, or `None` before the cycle began.
#[must_use]
pub fn daf_yomi(rd: Rd) -> Option<Daf> {
    let first = fixed_from_civil(FIRST);
    let eighth = fixed_from_civil(EIGHTH);
    if rd < first {
        return None;
    }
    let (cycle, index, shekalim) = if rd < eighth {
        let since = rd - first;
        let length = length_with(SHEKALIM_WAS);
        (since / length + 1, since % length, SHEKALIM_WAS)
    } else {
        let since = rd - eighth;
        let length = length_with(SHAS[SHEKALIM].dapim);
        (since / length + 8, since % length, SHAS[SHEKALIM].dapim)
    };

    let mut left = index;
    for masechta in SHAS {
        let dapim = Rd::from(if std::ptr::eq(masechta, &SHAS[SHEKALIM]) {
            shekalim
        } else {
            masechta.dapim
        });
        if left < dapim {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            return Some(Daf {
                masechta,
                daf: masechta.first + left as u32,
                cycle: cycle as u32,
                day: index as u32 + 1,
            });
        }
        left -= dapim;
    }
    None
}

/// How long a cycle is, with Shekalim counted at `shekalim` dapim.
fn length_with(shekalim: u32) -> Rd {
    SHAS.iter()
        .enumerate()
        .map(|(at, masechta)| {
            Rd::from(if at == SHEKALIM {
                shekalim
            } else {
                masechta.dapim
            })
        })
        .sum()
}

/// Everything the window says about a day.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Limud {
    /// `daf-yomi`, and room for the others.
    pub key: &'static str,
    /// What the limud is called.
    pub said: &'static str,
    /// The place, said — `ברכות ב׳`.
    pub place: String,
    /// The sefer to open.
    pub slug: String,
    /// The address inside it, when there is one. Absent means *this shelf holds
    /// the sefer and does not address it by daf* — Shekalim, Kinnim, Middos.
    pub address: Option<String>,
    /// The ref to open, so the window can take the same road a citation takes
    /// rather than a second one of its own — `girsa:bavli/berakhot/2a`.
    ///
    /// A masechta the shelf does not address by daf gets the whole work, which
    /// opens it at the beginning: the daf is named beside it and the reader can
    /// see it is not being sent to a page that does not exist.
    pub reference: String,
    /// Which cycle and which day of it, for the line under the place.
    pub cycle: u32,
    pub day: u32,
    pub of: u32,
    /// Whether the sefer is on this shelf.
    ///
    /// Set by the shell, which is the only thing that knows. A daf whose
    /// masechta was never imported is still today's daf and is still said —
    /// what changes is that the window offers to open it or does not, rather
    /// than opening nothing and looking broken.
    pub here: bool,
}

/// What a day is, and what is being learned on it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Luach {
    /// The civil date this was computed for.
    pub civil: Civil,
    /// The Hebrew date, said — `כ״ז אב תשפ״ו`.
    pub hebrew: String,
    /// The day of the week, said — `יום שלישי`.
    pub weekday: &'static str,
    /// Today's limudim.
    pub today: Vec<Limud>,
    /// Tomorrow's, because the daf turns over at nightfall and this does not.
    pub tomorrow: Vec<Limud>,
}

/// The luach for a civil date.
#[must_use]
pub fn of(date: Civil) -> Luach {
    let rd = fixed_from_civil(date);
    Luach {
        civil: date,
        hebrew: hebrew_from_fixed(rd).said(),
        weekday: weekday_name(day_of_week(rd)),
        today: limudim(rd),
        tomorrow: limudim(rd + 1),
    }
}

/// Every limud of one day.
///
/// A `Vec` with one element in it today. Mishnah Yomis and Rambam Yomi are the
/// obvious next two and they are schedules of a different shape — a count of
/// mishnayos and a count of perakim rather than a count of dapim — so the shape
/// that carries them is here from the start rather than retrofitted around the
/// one that exists.
fn limudim(rd: Rd) -> Vec<Limud> {
    daf_yomi(rd)
        .map(|daf| Limud {
            key: "daf-yomi",
            said: "דף היומי",
            place: daf.said(),
            slug: daf.masechta.slug.to_string(),
            reference: match daf.address() {
                Some(address) => format!("girsa:{}/{address}", daf.masechta.slug),
                None => format!("girsa:{}/", daf.masechta.slug),
            },
            address: daf.address(),
            cycle: daf.cycle,
            day: daf.day,
            of: 2711,
            here: false,
        })
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn civil(year: i32, month: u32, day: u32) -> Civil {
        Civil { year, month, day }
    }

    #[test]
    fn the_civil_calendar_round_trips_and_lands_where_it_is_known_to() {
        // RD 1 is 1 January of year 1, by definition.
        assert_eq!(fixed_from_civil(civil(1, 1, 1)), 1);
        // And two dates whose RD is published: the first of January 2020, and
        // the day the twenty-first century began.
        assert_eq!(fixed_from_civil(civil(2020, 1, 1)), 737_425);
        assert_eq!(fixed_from_civil(civil(2001, 1, 1)), 730_486);
        for rd in 730_000..735_000 {
            assert_eq!(fixed_from_civil(civil_from_fixed(rd)), rd, "rd {rd}");
        }
    }

    #[test]
    fn the_hebrew_calendar_round_trips_over_a_century() {
        // Both directions, every day from 1950 to 2050. A conversion that is
        // wrong about one day in that span is wrong about somebody's yahrtzeit.
        let from = fixed_from_civil(civil(1950, 1, 1));
        let to = fixed_from_civil(civil(2050, 1, 1));
        for rd in from..to {
            let hebrew = hebrew_from_fixed(rd);
            assert_eq!(fixed_from_hebrew(hebrew), rd, "rd {rd} → {hebrew:?}");
        }
    }

    #[test]
    fn rosh_hashanah_falls_where_the_luach_says_it_does() {
        // Four Rosh Hashanahs, each of them a date anybody can check against a
        // printed calendar. If the dechiyos were wrong these would drift by a
        // day, and nothing else in this module would notice.
        let rosh = |year: i32| {
            civil_from_fixed(fixed_from_hebrew(Hebrew {
                year,
                month: 7,
                day: 1,
            }))
        };
        assert_eq!(rosh(5784), civil(2023, 9, 16));
        assert_eq!(rosh(5785), civil(2024, 10, 3));
        assert_eq!(rosh(5786), civil(2025, 9, 23));
        assert_eq!(rosh(5787), civil(2026, 9, 12));
        // …and לא אד"ו ראש holds for every year in a century.
        for year in 5700..5800 {
            let day = day_of_week(fixed_from_hebrew(Hebrew {
                year,
                month: 7,
                day: 1,
            }));
            assert!(!matches!(day, 0 | 3 | 5), "{year} began on day {day}");
        }
    }

    #[test]
    fn a_year_is_one_of_the_six_lengths_it_is_allowed_to_be() {
        for year in 5600..5900 {
            let days = days_in_year(year);
            assert!(
                matches!(days, 353 | 354 | 355 | 383 | 384 | 385),
                "{year} has {days} days"
            );
        }
    }

    #[test]
    fn the_two_daf_yomi_epochs_are_seven_cycles_apart() {
        // The arithmetic checking itself. 11 September 1923 to 24 June 1975 is
        // 18,914 days, and 18,914 is exactly seven times 2,702 — which is the
        // whole Shas with Shekalim counted at twelve. If either date or either
        // count were wrong this would not divide.
        let span = fixed_from_civil(EIGHTH) - fixed_from_civil(FIRST);
        assert_eq!(span, 18_914);
        assert_eq!(length_with(SHEKALIM_WAS), 2_702);
        assert_eq!(span % length_with(SHEKALIM_WAS), 0);
        assert_eq!(span / length_with(SHEKALIM_WAS), 7);
        // And the Shas the cycle learns today is the 2,711 everybody counts.
        assert_eq!(length_with(SHAS[SHEKALIM].dapim), 2_711);
    }

    #[test]
    fn every_cycle_since_1923_begins_on_berachos_two() {
        for cycle in 1..=20u32 {
            let rd = if cycle <= 7 {
                fixed_from_civil(FIRST) + Rd::from(cycle - 1) * 2_702
            } else {
                fixed_from_civil(EIGHTH) + Rd::from(cycle - 8) * 2_711
            };
            let daf = daf_yomi(rd).expect("a daf");
            assert_eq!(daf.masechta.said, "ברכות", "cycle {cycle}");
            assert_eq!(daf.daf, 2, "cycle {cycle}");
            assert_eq!(daf.cycle, cycle);
            assert_eq!(daf.day, 1);
        }
    }

    #[test]
    fn the_two_cycle_starts_everybody_remembers() {
        // Independent of the epoch arithmetic: these are the dates the world
        // held the siyum on, and they only both come out right if the epoch,
        // the Shekalim change and the cycle length are all correct.
        let start = |date: Civil| {
            let daf = daf_yomi(fixed_from_civil(date)).expect("a daf");
            (daf.masechta.said, daf.daf, daf.cycle)
        };
        assert_eq!(start(civil(2012, 8, 3)), ("ברכות", 2, 13));
        assert_eq!(start(civil(2020, 1, 5)), ("ברכות", 2, 14));
        // The day before is the last daf of the Shas, which is what the siyum
        // was for.
        let last = daf_yomi(fixed_from_civil(civil(2020, 1, 4))).expect("a daf");
        assert_eq!((last.masechta.said, last.daf), ("נדה", 73));
        assert_eq!(last.day, 2_711);
    }

    #[test]
    fn a_cycle_covers_every_daf_of_every_masechta_exactly_once() {
        let start = fixed_from_civil(civil(2020, 1, 5));
        let mut seen: Vec<(&str, u32)> = Vec::new();
        for offset in 0..2_711 {
            let daf = daf_yomi(start + offset).expect("a daf");
            seen.push((daf.masechta.said, daf.daf));
        }
        assert_eq!(seen.len(), 2_711);
        let mut sorted = seen.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 2_711, "a daf was learned twice");
        // The four that share a pagination run on continuously, which is the
        // part of the table that looks like a mistake.
        let where_is = |name: &str| {
            seen.iter()
                .filter(|(m, _)| *m == name)
                .map(|(_, d)| *d)
                .collect::<Vec<_>>()
        };
        assert_eq!(where_is("מעילה").last(), Some(&22));
        assert_eq!(where_is("קינים"), vec![23, 24, 25]);
        assert_eq!(where_is("תמיד").first(), Some(&26));
        assert_eq!(where_is("מדות"), vec![34, 35, 36, 37]);
    }

    #[test]
    fn nothing_is_said_about_a_day_before_the_cycle_began() {
        assert!(daf_yomi(fixed_from_civil(civil(1923, 9, 10))).is_none());
        assert!(daf_yomi(fixed_from_civil(civil(1923, 9, 11))).is_some());
    }

    #[test]
    fn a_luach_says_the_date_and_the_daf_the_way_a_person_writes_them() {
        let luach = of(civil(2020, 1, 5));
        assert_eq!(luach.weekday, "יום ראשון");
        assert_eq!(luach.hebrew, "ח' טבת תש\"פ");
        assert_eq!(luach.today.len(), 1);
        assert_eq!(luach.today[0].place, "ברכות ב'");
        assert_eq!(luach.today[0].slug, "bavli/berakhot");
        assert_eq!(luach.today[0].address.as_deref(), Some("2a"));
        assert_eq!(luach.today[0].reference, "girsa:bavli/berakhot/2a");
        // Tomorrow is offered because the daf turns over at nightfall and this
        // turns it over at midnight.
        assert_eq!(luach.tomorrow[0].place, "ברכות ג'");
    }

    #[test]
    fn the_three_masechtos_this_shelf_does_not_address_by_daf_say_so() {
        // Shekalim is the Yerushalmi and Kinnim and Middos are Mishnayos: the
        // corpus holds all three and addresses none of them by daf. A luach
        // that handed the window `girsa:mishnah-kinnim/23a` would send it to a
        // place that does not exist.
        let not_by_daf: Vec<&str> = SHAS.iter().filter(|m| !m.by_daf).map(|m| m.said).collect();
        assert_eq!(not_by_daf, ["שקלים", "קינים", "מדות"]);
        for masechta in SHAS.iter().filter(|m| !m.by_daf) {
            let daf = Daf {
                masechta,
                daf: masechta.first,
                cycle: 14,
                day: 1,
            };
            assert_eq!(daf.address(), None);
        }
    }
}
