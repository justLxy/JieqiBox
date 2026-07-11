use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveData {
    pub uci_move: String,
    pub priority: i32,
    pub wins: i32,
    pub draws: i32,
    pub losses: i32,
    pub allowed: bool,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddEntryRequest {
    pub fen: String,
    pub uci_move: String,
    pub priority: i32,
    pub wins: i32,
    pub draws: i32,
    pub losses: i32,
    pub allowed: bool,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningBookEntry {
    pub key: String,
    pub fen: String,
    pub moves: Vec<MoveData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningBookStats {
    pub total_positions: i64,
    pub total_moves: i64,
    pub allowed_moves: i64,
    pub disallowed_moves: i64,
}

pub struct JieqiOpeningBook {
    conn: Connection,
}

struct StoredOpeningBookEntry {
    key: Vec<u8>,
    move_int: i64,
    fen: String,
    priority: i32,
    wins: i32,
    draws: i32,
    losses: i32,
    allowed: bool,
    comment: String,
}

impl JieqiOpeningBook {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let book = JieqiOpeningBook { conn };
        book.initialize_database()?;
        Ok(book)
    }

    fn initialize_database(&self) -> Result<()> {
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS openings (
                key      BLOB NOT NULL,
                move     INTEGER NOT NULL,
                fen      TEXT NOT NULL DEFAULT '',
                priority INTEGER NOT NULL,
                wins     INTEGER NOT NULL,
                draws    INTEGER NOT NULL,
                losses   INTEGER NOT NULL,
                allowed  INTEGER NOT NULL,
                comment  TEXT,
                PRIMARY KEY (key, move)
            );
            "#,
            [],
        )?;

        let has_fen_column = self
            .conn
            .prepare("PRAGMA table_info(openings)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>>>()?
            .iter()
            .any(|column| column == "fen");

        if !has_fen_column {
            self.conn.execute(
                "ALTER TABLE openings ADD COLUMN fen TEXT NOT NULL DEFAULT ''",
                [],
            )?;
        }
        Ok(())
    }

    pub fn add_entry(&self, request: &AddEntryRequest) -> Result<bool> {
        self.upsert_stored_entries(&[stored_entry_from_request(request)])
    }

    /// Insert or update a group of entries in one SQLite transaction. This is
    /// used by marked-move edits and imports, where autocommit per move turns a
    /// small UI action into many fsyncs.
    pub fn upsert_all(&self, requests: &[AddEntryRequest]) -> Result<bool> {
        let entries: Vec<StoredOpeningBookEntry> =
            requests.iter().map(stored_entry_from_request).collect();
        self.upsert_stored_entries(&entries)
    }

    /// Import a legacy JSON export whose entries contain only the canonical key
    /// and canonical move coordinates. New exports carry FEN and use
    /// `upsert_all` instead.
    pub fn upsert_legacy_entries(&self, entries: &[(Vec<u8>, MoveData)]) -> Result<bool> {
        let stored_entries: Vec<StoredOpeningBookEntry> = entries
            .iter()
            .map(|(key, move_data)| StoredOpeningBookEntry {
                key: key.clone(),
                move_int: uci_to_int(&move_data.uci_move) as i64,
                fen: String::new(),
                priority: move_data.priority,
                wins: move_data.wins,
                draws: move_data.draws,
                losses: move_data.losses,
                allowed: move_data.allowed,
                comment: move_data.comment.clone(),
            })
            .collect();
        self.upsert_stored_entries(&stored_entries)
    }

    fn upsert_stored_entries(&self, entries: &[StoredOpeningBookEntry]) -> Result<bool> {
        if entries.is_empty() {
            return Ok(true);
        }

        let tx = self.conn.unchecked_transaction()?;
        {
            let mut statement = tx.prepare(
                r#"
                INSERT INTO openings (key, move, fen, priority, wins, draws, losses, allowed, comment)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(key, move) DO UPDATE SET
                    fen=CASE WHEN excluded.fen <> '' THEN excluded.fen ELSE openings.fen END,
                    priority=excluded.priority,
                    wins=excluded.wins,
                    draws=excluded.draws,
                    losses=excluded.losses,
                    allowed=excluded.allowed,
                    comment=excluded.comment;
                "#,
            )?;

            for entry in entries {
                statement.execute(rusqlite::params![
                    &entry.key,
                    entry.move_int,
                    &entry.fen,
                    entry.priority,
                    entry.wins,
                    entry.draws,
                    entry.losses,
                    if entry.allowed { 1 } else { 0 },
                    &entry.comment,
                ])?;
            }
        }
        tx.commit()?;
        Ok(true)
    }

    pub fn delete_entry(&self, fen: &str, uci_move: &str) -> Result<bool> {
        let (key_blob, transform_idx) = compute_key_and_transform(fen);
        let transformed_uci = transform_uci_move(uci_move, transform_idx);
        let move_int = uci_to_int(&transformed_uci) as i64;

        let affected_rows = self.conn.execute(
            "DELETE FROM openings WHERE key = ?1 AND move = ?2",
            rusqlite::params![&key_blob, move_int],
        )?;

        Ok(affected_rows > 0)
    }

    pub fn query_moves(&self, fen: &str) -> Result<Vec<MoveData>> {
        let (key_blob, transform_idx) = compute_key_and_transform(fen);

        let mut stmt = self.conn.prepare(
            "SELECT move, priority, wins, draws, losses, allowed, comment FROM openings WHERE key = ?1 ORDER BY priority DESC"
        )?;

        let move_iter = stmt.query_map(rusqlite::params![key_blob], |row| {
            Ok(MoveData {
                uci_move: int_to_uci(row.get::<_, i32>(0)? as u16),
                priority: row.get(1)?,
                wins: row.get(2)?,
                draws: row.get(3)?,
                losses: row.get(4)?,
                allowed: row.get::<_, i32>(5)? == 1,
                comment: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
            })
        })?;

        let mut moves = Vec::new();
        for move_result in move_iter {
            moves.push(move_result?);
        }

        // Restore moves from database (normalized coordinate system) to user's FEN coordinate system
        for m in moves.iter_mut() {
            m.uci_move = transform_uci_move(&m.uci_move, transform_idx);
        }

        Ok(moves)
    }

    pub fn get_stats(&self) -> Result<OpeningBookStats> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(DISTINCT key), COUNT(*), COALESCE(SUM(CASE WHEN allowed = 1 THEN 1 ELSE 0 END), 0), COALESCE(SUM(CASE WHEN allowed = 0 THEN 1 ELSE 0 END), 0) FROM openings"
        )?;

        let stats = stmt.query_row([], |row| {
            Ok(OpeningBookStats {
                total_positions: row.get(0)?,
                total_moves: row.get(1)?,
                allowed_moves: row.get(2)?,
                disallowed_moves: row.get(3)?,
            })
        })?;

        Ok(stats)
    }

    pub fn clear_all(&self) -> Result<()> {
        self.conn.execute("DELETE FROM openings", [])?;
        Ok(())
    }

    pub fn export_all(&self) -> Result<Vec<OpeningBookEntry>> {
        let mut entries: HashMap<String, OpeningBookEntry> = HashMap::new();

        let mut stmt = self.conn.prepare(
            "SELECT key, move, fen, priority, wins, draws, losses, allowed, comment FROM openings",
        )?;
        let entry_iter = stmt.query_map([], |row| {
            let key_blob: Vec<u8> = row.get(0)?;
            let key_hex = hex::encode(key_blob);
            let fen: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
            let move_data = MoveData {
                uci_move: int_to_uci(row.get::<_, i32>(1)? as u16),
                priority: row.get(3)?,
                wins: row.get(4)?,
                draws: row.get(5)?,
                losses: row.get(6)?,
                allowed: row.get::<_, i32>(7)? == 1,
                comment: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
            };
            Ok((key_hex, fen, move_data))
        })?;

        for entry_result in entry_iter {
            let (key, fen, move_data) = entry_result?;
            let entry = entries
                .entry(key.clone())
                .or_insert_with(|| OpeningBookEntry {
                    key: key.clone(),
                    fen: String::new(),
                    moves: Vec::new(),
                });
            if entry.fen.is_empty() && !fen.is_empty() {
                entry.fen = fen;
            }
            entry.moves.push(move_data);
        }

        // Restore canonical coordinates to the exported FEN. Legacy entries
        // without a FEN remain canonical and can be imported through their key.
        for entry in entries.values_mut() {
            if !entry.fen.is_empty() {
                let (_, transform_idx) = compute_key_and_transform(&entry.fen);
                for move_data in &mut entry.moves {
                    move_data.uci_move = transform_uci_move(&move_data.uci_move, transform_idx);
                }
            }
            entry.moves.sort_by(|a, b| b.priority.cmp(&a.priority));
        }

        Ok(entries.into_values().collect())
    }
}

fn stored_entry_from_request(request: &AddEntryRequest) -> StoredOpeningBookEntry {
    let (key, transform_idx) = compute_key_and_transform(&request.fen);
    StoredOpeningBookEntry {
        key,
        move_int: uci_to_int(&transform_uci_move(&request.uci_move, transform_idx)) as i64,
        fen: request.fen.clone(),
        priority: request.priority,
        wins: request.wins,
        draws: request.draws,
        losses: request.losses,
        allowed: request.allowed,
        comment: request.comment.clone(),
    }
}

// FEN processing functions
fn parse_pool_string(pool_str: &str) -> (HashMap<char, i32>, HashMap<char, i32>) {
    let mut red_pool = HashMap::new();
    let mut black_pool = HashMap::new();

    let chars: Vec<char> = pool_str.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i].is_alphabetic() {
            let piece = chars[i];
            let mut count = 1;

            if i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                count = chars[i + 1].to_digit(10).unwrap_or(1) as i32;
                i += 1;
            }

            if piece.is_uppercase() {
                *red_pool.entry(piece).or_insert(0) += count;
            } else {
                *black_pool.entry(piece).or_insert(0) += count;
            }
        }
        i += 1;
    }

    (red_pool, black_pool)
}

fn calculate_gcd(pool: &HashMap<char, i32>) -> i32 {
    let values: Vec<i32> = pool.values().filter(|&&v| v > 0).copied().collect();
    if values.is_empty() {
        return 1;
    }

    let mut result = values[0];
    for &value in &values[1..] {
        result = gcd(result, value);
    }

    if result == 0 {
        1
    } else {
        result
    }
}

fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

fn format_pool(pool: &HashMap<char, i32>, order: &str) -> String {
    let mut result = String::new();
    for piece_type in order.chars() {
        if let Some(&count) = pool.get(&piece_type) {
            if count > 0 {
                result.push(piece_type);
                if count > 1 {
                    result.push_str(&count.to_string());
                }
            }
        }
    }
    result
}

fn normalize_fen(fen: &str) -> String {
    let parts: Vec<&str> = fen.split_whitespace().collect();
    if parts.len() < 4 {
        return fen.to_string();
    }

    let board = parts[0];
    let side_to_move = parts[1];
    let dark_pool_str = parts[2];

    // Check which side has dark pieces
    let red_has_dark_pieces = board.contains('X');
    let black_has_dark_pieces = board.contains('x');
    let red_dark_count_on_board = board.chars().filter(|&c| c == 'X').count();
    let black_dark_count_on_board = board.chars().filter(|&c| c == 'x').count();

    let (mut red_dark, mut black_dark) = if dark_pool_str != "-" {
        let (temp_red, temp_black) = parse_pool_string(dark_pool_str);
        (
            if red_has_dark_pieces {
                temp_red
            } else {
                HashMap::new()
            },
            if black_has_dark_pieces {
                temp_black
            } else {
                HashMap::new()
            },
        )
    } else {
        (HashMap::new(), HashMap::new())
    };

    // Normalize by GCD when and only when that side has exactly one dark piece
    let gcd_red_dark = if red_dark_count_on_board == 1 {
        calculate_gcd(&red_dark)
    } else {
        1
    };
    if gcd_red_dark > 1 {
        for value in red_dark.values_mut() {
            *value /= gcd_red_dark;
        }
    }

    let gcd_black_dark = if black_dark_count_on_board == 1 {
        calculate_gcd(&black_dark)
    } else {
        1
    };
    if gcd_black_dark > 1 {
        for value in black_dark.values_mut() {
            *value /= gcd_black_dark;
        }
    }

    // Format pools
    let red_order = "RNBACP";
    let black_order = "rnbacp";

    let mut final_dark_pool = format_pool(&red_dark, red_order);
    final_dark_pool.push_str(&format_pool(&black_dark, black_order));

    if final_dark_pool.is_empty() {
        final_dark_pool = "-".to_string();
    }

    format!("{} {} {} -", board, side_to_move, final_dark_pool)
}

fn flip_board(fen_string: &str) -> String {
    let parts: Vec<&str> = fen_string.split_whitespace().collect();
    if parts.len() < 4 {
        return fen_string.to_string();
    }

    let board = parts[0];
    let side = parts[1];
    let dark_pool = parts[2];

    let rows: Vec<&str> = board.split('/').collect();
    let mut flipped_rows = Vec::new();

    for row in rows {
        let flipped_row: String = row.chars().rev().collect();
        flipped_rows.push(flipped_row);
    }

    let flipped_board = flipped_rows.join("/");
    format!("{} {} {} -", flipped_board, side, dark_pool)
}

fn swap_colors_fen(normalized_fen: &str) -> String {
    let parts: Vec<&str> = normalized_fen.split_whitespace().collect();
    if parts.len() < 4 {
        return normalized_fen.to_string();
    }

    let board = parts[0];
    let side_to_move = parts[1];
    let dark_pool_str = parts[2];

    // 1. Swap piece cases and flip board vertically
    let swapped_case_board: String = board
        .chars()
        .map(|c| {
            if c.is_lowercase() && c.is_alphabetic() {
                c.to_uppercase().next().unwrap_or(c)
            } else if c.is_uppercase() && c.is_alphabetic() {
                c.to_lowercase().next().unwrap_or(c)
            } else {
                c
            }
        })
        .collect();

    // Vertically flip the board (reverse row order)
    let rows: Vec<&str> = swapped_case_board.split('/').collect();
    let reversed_rows: Vec<&str> = rows.into_iter().rev().collect();
    let swapped_board = reversed_rows.join("/");

    // 2. Swap side to move
    let swapped_side = if side_to_move == "w" { "b" } else { "w" };

    // 3. Swap dark pool only
    let (red_dark_orig, black_dark_orig) = parse_pool_string(dark_pool_str);

    let mut new_red_dark = HashMap::new();
    for (&piece, &count) in &black_dark_orig {
        new_red_dark.insert(piece.to_uppercase().next().unwrap_or(piece), count);
    }

    let mut new_black_dark = HashMap::new();
    for (&piece, &count) in &red_dark_orig {
        new_black_dark.insert(piece.to_lowercase().next().unwrap_or(piece), count);
    }

    let red_order = "RNBACP";
    let black_order = "rnbacp";

    let mut swapped_dark_pool = format_pool(&new_red_dark, red_order);
    swapped_dark_pool.push_str(&format_pool(&new_black_dark, black_order));

    if swapped_dark_pool.is_empty() {
        swapped_dark_pool = "-".to_string();
    }

    // Always use "-" for captured pool to ignore it in key calculation
    format!("{} {} {} -", swapped_board, swapped_side, swapped_dark_pool)
}

// Compute key value, also return the transformation index used
// Transformation index definitions:
// 0 = original normalized FEN; 1 = horizontal mirror; 2 = color swap (with vertical flip); 3 = color swap then horizontal mirror
fn compute_key_and_transform(fen: &str) -> (Vec<u8>, usize) {
    let norm_fen = normalize_fen(fen);
    let swapped_fen = swap_colors_fen(&norm_fen);

    let fens = [
        norm_fen.clone(),
        flip_board(&norm_fen),
        swapped_fen.clone(),
        flip_board(&swapped_fen),
    ];

    let mut min_reversed_hash = String::new();
    let mut min_idx: usize = 0;

    for (i, fen_variant) in fens.iter().enumerate() {
        let mut hasher = Sha256::new();
        hasher.update(fen_variant.as_bytes());
        let hash_bytes = hasher.finalize();
        let hash_hex_str = hex::encode(hash_bytes);
        let reversed_hash: String = hash_hex_str.chars().rev().collect();

        if i == 0 || reversed_hash < min_reversed_hash {
            min_reversed_hash = reversed_hash;
            min_idx = i;
        }
    }

    // Take first 24 hex chars (96 bits) and convert to 12-byte blob
    let key_hex = &min_reversed_hash[..24];
    let mut key_blob: Vec<u8> = Vec::with_capacity(12);
    for i in (0..24).step_by(2) {
        let byte = u8::from_str_radix(&key_hex[i..i + 2], 16).unwrap_or(0);
        key_blob.push(byte);
    }
    (key_blob, min_idx)
}

// Transform UCI move coordinates according to transformation index. This function is its own inverse (repeated calls with same index restore original).
fn transform_uci_move(uci: &str, transform_idx: usize) -> String {
    if uci.len() != 4 {
        return uci.to_string();
    }

    let chars: Vec<char> = uci.chars().collect();
    let mut from_x = (chars[0] as u8).wrapping_sub(b'a') as i32;
    let mut from_y = chars[1].to_digit(10).unwrap_or(0) as i32;
    let mut to_x = (chars[2] as u8).wrapping_sub(b'a') as i32;
    let mut to_y = chars[3].to_digit(10).unwrap_or(0) as i32;

    match transform_idx {
        0 => {}
        1 => {
            // Horizontal mirror: x -> 8 - x
            from_x = 8 - from_x;
            to_x = 8 - to_x;
        }
        2 => {
            // Color swap (with vertical flip): y -> 9 - y
            from_y = 9 - from_y;
            to_y = 9 - to_y;
        }
        3 => {
            // Combined: horizontal then vertical (order doesn't matter)
            from_x = 8 - from_x;
            to_x = 8 - to_x;
            from_y = 9 - from_y;
            to_y = 9 - to_y;
        }
        _ => {}
    }

    let fx = (b'a' + from_x as u8) as char;
    let fy = char::from_digit(from_y as u32, 10).unwrap_or('0');
    let tx = (b'a' + to_x as u8) as char;
    let ty = char::from_digit(to_y as u32, 10).unwrap_or('0');
    format!("{}{}{}{}", fx, fy, tx, ty)
}

fn uci_to_int(uci: &str) -> u16 {
    if uci.len() != 4 {
        return 0;
    }

    let chars: Vec<char> = uci.chars().collect();
    let from_x = (chars[0] as u8).wrapping_sub(b'a') as u16;
    let from_y = chars[1].to_digit(10).unwrap_or(0) as u16;
    let to_x = (chars[2] as u8).wrapping_sub(b'a') as u16;
    let to_y = chars[3].to_digit(10).unwrap_or(0) as u16;

    let from_coord = from_y * 9 + from_x;
    let to_coord = to_y * 9 + to_x;

    (from_coord << 8) | to_coord
}

fn int_to_uci(move_int: u16) -> String {
    let from_coord = (move_int >> 8) & 0xff;
    let to_coord = move_int & 0xff;

    let from_x = char::from(b'a' + (from_coord % 9) as u8);
    let from_y = char::from_digit((from_coord / 9) as u32, 10).unwrap_or('0');
    let to_x = char::from(b'a' + (to_coord % 9) as u8);
    let to_y = char::from_digit((to_coord / 9) as u32, 10).unwrap_or('0');

    format!("{}{}{}{}", from_x, from_y, to_x, to_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEN: &str = "rnbakabnr/9/1c5c1/p1p1p1p1p/9/9/P1P1P1P1P/1C5C1/9/RNBAKABNR w - - 0 1";

    fn request(move_name: &str, priority: i32) -> AddEntryRequest {
        AddEntryRequest {
            fen: FEN.to_string(),
            uci_move: move_name.to_string(),
            priority,
            wins: 0,
            draws: 0,
            losses: 0,
            allowed: true,
            comment: String::new(),
        }
    }

    #[test]
    fn batch_upsert_exports_and_reimports_position_fen() {
        let source = JieqiOpeningBook::new(":memory:").unwrap();
        source
            .upsert_all(&[request("a3a4", 80), request("b3b4", 120)])
            .unwrap();

        let exported = source.export_all().unwrap();
        assert_eq!(exported.len(), 1);
        assert_eq!(exported[0].fen, FEN);
        assert_eq!(exported[0].moves.len(), 2);

        let restored = JieqiOpeningBook::new(":memory:").unwrap();
        let requests: Vec<AddEntryRequest> = exported[0]
            .moves
            .iter()
            .map(|move_data| AddEntryRequest {
                fen: exported[0].fen.clone(),
                uci_move: move_data.uci_move.clone(),
                priority: move_data.priority,
                wins: move_data.wins,
                draws: move_data.draws,
                losses: move_data.losses,
                allowed: move_data.allowed,
                comment: move_data.comment.clone(),
            })
            .collect();
        restored.upsert_all(&requests).unwrap();

        let moves = restored.query_moves(FEN).unwrap();
        assert_eq!(moves.len(), 2);
        assert_eq!(moves[0].uci_move, "b3b4");
        assert_eq!(moves[1].uci_move, "a3a4");
    }
}
