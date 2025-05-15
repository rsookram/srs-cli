pub mod editor;
pub mod error;
pub mod prompt;
pub mod rand;
mod tmp;

use error::Result;
use rand::Rng;
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
    str,
};

/// The reduction factor applied to the next interval when the card was answered incorrectly.
const WRONG_ANSWER_PENALTY: f32 = 0.7;

const MAX_CARD_COUNT: usize = u16::MAX as usize;
// The file format can handle longer cards, but this should be more than enough.
const MAX_CARD_LEN: usize = 4 * 1024;

pub struct Srs {
    pub cards: Box<[Vec<u8>]>,
    pub schedule: Box<[CardSchedule]>,
    pub stats: Box<Stats>,
}

impl Default for Srs {
    fn default() -> Self {
        Self {
            cards: Box::new([]),
            schedule: Box::new([]),
            stats: Box::new([Stat::default(); STAT_ROW_COUNT]),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CardSchedule {
    pub most_recent_interval: u16,
    pub scheduled_for: u16,
}

const STAT_ROW_COUNT: usize = 365;

pub type Stats = [Stat; STAT_ROW_COUNT];

#[derive(Clone, Copy, Debug, Default)]
pub struct Stat {
    pub correct: u8,
    pub wrong: u8,
}

pub type CardIndex = u16;

#[derive(Debug)]
pub struct Card {
    pub front: Box<str>,
    pub back: Box<str>,
}

impl Card {
    const SEPARATOR: u8 = b'\0';
    const SEPARATOR_STR: &[u8; 1] = b"\0";
}

#[derive(Debug)]
pub struct Answer {
    pub card_index: CardIndex,
    pub is_correct: bool,
}

pub fn add_card(
    srs: Srs,
    path: &Path,
    now_in_epoch_days: u16,
    front: String,
    back: String,
) -> Result<()> {
    if srs.cards.len() >= MAX_CARD_COUNT {
        return Err("reached card count limit".into());
    }

    let card: Vec<u8> = front
        .as_bytes()
        .iter()
        .chain(Card::SEPARATOR_STR)
        .chain(back.as_bytes())
        .copied()
        .collect();

    if card.len() > MAX_CARD_LEN {
        return Err("this card is too long".into());
    }

    let mut cards = srs.cards.into_vec();
    cards.push(card);

    let mut schedule = srs.schedule.into_vec();
    schedule.push(CardSchedule {
        most_recent_interval: 1,
        scheduled_for: now_in_epoch_days + 1,
    });

    write(path, cards, &schedule, *srs.stats)?;

    Ok(())
}

pub fn edit_card(srs: Srs, path: &Path, idx: CardIndex, front: String, back: String) -> Result<()> {
    let card: Vec<u8> = front
        .as_bytes()
        .iter()
        .chain(Card::SEPARATOR_STR)
        .chain(back.as_bytes())
        .copied()
        .collect();

    if card.len() > MAX_CARD_LEN {
        return Err("this card is too long".into());
    }

    let mut cards = srs.cards.into_vec();
    cards[usize::from(idx)] = card;

    write(path, cards, &srs.schedule.into_vec(), *srs.stats)?;

    Ok(())
}

pub fn delete_card(srs: Srs, path: &Path, idx: CardIndex) -> Result<()> {
    let mut cards = srs.cards.into_vec();
    cards.remove(usize::from(idx));

    let mut schedule = srs.schedule.into_vec();
    schedule.remove(usize::from(idx));

    write(path, cards, &schedule, *srs.stats)?;

    Ok(())
}

pub fn card_front(bytes: &[u8]) -> Result<&str> {
    let separator_idx = bytes
        .iter()
        .position(|&b| b == Card::SEPARATOR)
        .ok_or_else(|| {
            format!(
                "no null separator in card string: len={}, str={:?}",
                bytes.len(),
                str::from_utf8(bytes),
            )
        })?;

    Ok(str::from_utf8(&bytes[..separator_idx])?)
}

pub fn cards_to_review(srs: &Srs, now_in_epoch_days: u16) -> Vec<CardIndex> {
    srs.schedule
        .iter()
        .enumerate()
        .filter(|&(_, sched)| sched.scheduled_for <= now_in_epoch_days)
        .map(|(i, _)| i as CardIndex)
        .collect()
}

pub fn card(srs: &Srs, i: CardIndex) -> Result<Card> {
    let card = srs
        .cards
        .get(usize::from(i))
        .ok_or_else(|| format!("card {i} doesn't exist"))?;

    let separator_idx = card
        .iter()
        .position(|&b| b == Card::SEPARATOR)
        .ok_or_else(|| {
            format!(
                "no null separator in card string: len={}, str={:?}",
                card.len(),
                str::from_utf8(card),
            )
        })?;

    let mut front = card.clone();
    let back = front.split_off(separator_idx);

    Ok(Card {
        front: String::from_utf8(front)?.into_boxed_str(),
        back: String::from_utf8(back)?.into_boxed_str(),
    })
}

pub fn apply_answers(
    srs: Srs,
    path: &Path,
    now_in_epoch_days: u16,
    answers: &mut [Answer],
) -> Result<()> {
    answers.sort_by_key(|k| k.card_index);

    let mut stats = srs.stats;
    let mut schedule = srs.schedule.into_vec();

    let mut rng = Rng::new();

    for answer in answers {
        let idx = answer.card_index;

        let sched = schedule
            .get_mut(usize::from(idx))
            .expect("card wasn't deleted during review");
        if answer.is_correct {
            let last_was_correct = sched.scheduled_for != 0;

            let mut new_interval = if last_was_correct {
                sched.most_recent_interval * 5
            } else {
                ((sched.most_recent_interval as f32) * WRONG_ANSWER_PENALTY).round() as u16
            };

            // Generate a number in -fuzz..=fuzz. This fuzz factor prevents cards from getting
            // grouped together based on when they were added.
            let max_fuzz = ((new_interval as f32) * 0.05).ceil() as u16;
            let fuzz = rng.u16(max_fuzz);

            if rng.bool() {
                new_interval += fuzz;
            } else {
                new_interval -= fuzz;
            }

            new_interval = new_interval.max(1);

            *sched = CardSchedule {
                most_recent_interval: new_interval,
                scheduled_for: now_in_epoch_days + new_interval,
            }
        } else {
            *sched = CardSchedule {
                scheduled_for: 0,
                ..*sched
            }
        }

        let stat = match stats.get_mut(usize::from(sched.most_recent_interval)) {
            Some(s) => s,
            // The last bucket in stats covers all the intervals from that day onward.
            None => stats.last_mut().unwrap(),
        };
        // Reset on wraparound
        if stat.correct == u8::MAX || stat.wrong == u8::MAX {
            *stat = Stat {
                correct: 0,
                wrong: 0,
            }
        }

        *stat = if answer.is_correct {
            Stat {
                correct: stat.correct + 1,
                ..*stat
            }
        } else {
            Stat {
                wrong: stat.wrong + 1,
                ..*stat
            }
        };
    }

    write(path, srs.cards.into_vec(), &schedule, *stats)?;

    Ok(())
}

pub fn open(p: &Path) -> Result<Srs> {
    const NUM_CARDS_BYTES: usize = 2;
    const CARD_LENGTH_BYTES: usize = 2;

    const SCHEDULE_ROW_BYTES: usize = 4;

    const STAT_ROW_BYTES: usize = 2;
    const STAT_BYTES: usize = STAT_ROW_BYTES * STAT_ROW_COUNT;

    const MIN_SIZE_BYTES: usize = NUM_CARDS_BYTES + STAT_BYTES;

    let bytes = std::fs::read(p)?;
    if bytes.len() < MIN_SIZE_BYTES {
        return Err(format!(
            "read {} < {} bytes from {}",
            bytes.len(),
            MIN_SIZE_BYTES,
            p.to_string_lossy(),
        )
        .into());
    }

    let num_cards = usize::from(u16::from_le_bytes([bytes[0], bytes[1]]));

    let mut stats = [Stat::default(); STAT_ROW_COUNT];
    bytes[NUM_CARDS_BYTES..][..STAT_BYTES]
        .chunks_exact(STAT_ROW_BYTES)
        .enumerate()
        .for_each(|(i, chunk)| {
            stats[i] = Stat {
                correct: chunk[0],
                wrong: chunk[1],
            }
        });

    let schedule_start_offset = MIN_SIZE_BYTES;

    let schedule = bytes[schedule_start_offset..][..num_cards * SCHEDULE_ROW_BYTES]
        .chunks_exact(SCHEDULE_ROW_BYTES)
        .map(|chunk| CardSchedule {
            most_recent_interval: u16::from_le_bytes([chunk[0], chunk[1]]),
            scheduled_for: u16::from_le_bytes([chunk[2], chunk[3]]),
        })
        .collect();

    let cards_start_offset = schedule_start_offset + (num_cards * SCHEDULE_ROW_BYTES);

    let mut cards = Vec::with_capacity(num_cards);
    let mut cards_bytes = &bytes[cards_start_offset..];
    while !cards_bytes.is_empty() {
        let length = usize::from(u16::from_le_bytes([cards_bytes[0], cards_bytes[1]]));
        cards_bytes = &cards_bytes[CARD_LENGTH_BYTES..];

        cards.push(cards_bytes[..length].to_vec());
        cards_bytes = &cards_bytes[length..];
    }

    Ok(Srs {
        cards: cards.into_boxed_slice(),
        schedule,
        stats: Box::new(stats),
    })
}

pub fn write(
    path: &Path,
    cards: Vec<Vec<u8>>,
    schedule: &[CardSchedule],
    stats: Stats,
) -> Result<()> {
    let num_cards: u16 = cards.len().try_into().unwrap();

    let tmp_path = tmp::path();
    {
        let new_file = File::create(&tmp_path)?;
        let mut new_buf = BufWriter::with_capacity(256 * 1024, new_file);

        // Fixed header
        new_buf.write_all(&num_cards.to_le_bytes())?;

        for stat in stats {
            new_buf.write_all(&[stat.correct, stat.wrong])?;
        }

        // Schedule
        for s in schedule {
            new_buf.write_all(&s.most_recent_interval.to_le_bytes())?;
            new_buf.write_all(&s.scheduled_for.to_le_bytes())?;
        }

        // Cards
        for card in cards {
            let length: u16 = card.len().try_into().unwrap();
            new_buf.write_all(&length.to_le_bytes())?;
            new_buf.write_all(&card)?;
        }
    }

    fs::rename(tmp_path, path)?;

    Ok(())
}
