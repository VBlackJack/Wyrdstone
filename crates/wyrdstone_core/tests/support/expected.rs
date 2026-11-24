/*
 * Copyright 2026 Julien Bombled
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! Pinned digests of scenarios S1-S11 captured from the shipped rules at c29ab4b.
//!
//! Each line is `support::digest::describe` of one observed event; the last line pins the RNG
//! stream position after the scenario.

/// Pinned digest lines of scenario `n` (1..=11).
pub fn lines(n: usize) -> &'static [&'static str] {
    match n {
        1 => &S1,
        2 => &S2,
        3 => &S3,
        4 => &S4,
        5 => &S5,
        6 => &S6,
        7 => &S7,
        8 => &S8,
        9 => &S9,
        10 => &S10,
        11 => &S11,
        _ => panic!("unknown scenario {n}"),
    }
}

const S1: [&str; 4] = [
    "a0: pending combo=1 consumes_turn=true direct=false cells=[4,0 5,0 6,0] spawned=[] converted=[] fired=[] | score=0 fury=0 moves=23 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 0] boss(none) out=InProgress stats=(0,0) curses=[] board=63254f674209c73a",
    "a0: cleared combo=1 raw=150 runes=3 cleared=[4,0 5,0 6,0] fall=3:e860870a61af6643 boss_hit=false phase2=None | score=150 fury=15 moves=23 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 0] boss(none) out=InProgress stats=(3,1) curses=[] board=3a3857172a42b40e",
    "a0: ended frost=None curse_hits=[] riposte=[]None cursed_remain=false shuffled=false outcome=InProgress boosters_left=0 | score=150 fury=15 moves=23 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 0] boss(none) out=InProgress stats=(3,1) curses=[] board=3a3857172a42b40e",
    "rng_next=14689791305943627509",
];

const S2: [&str; 6] = [
    "a0: pending combo=1 consumes_turn=true direct=false cells=[3,1 3,2 3,3] spawned=[] converted=[] fired=[] | score=0 fury=0 moves=23 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 0] boss(none) out=InProgress stats=(0,0) curses=[] board=bc9a4b05bce48dfb",
    "a0: cleared combo=1 raw=370 runes=2 cleared=[3,1 3,2 3,3] fall=3:b0dad5d74f9d63e7 boss_hit=false phase2=None | score=370 fury=20 moves=23 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 2] boss(none) out=InProgress stats=(2,1) curses=[] board=34a9d1c87c6f88a3",
    "a0: pending combo=2 consumes_turn=true direct=false cells=[1,1 2,1 3,1] spawned=[] converted=[] fired=[] | score=370 fury=20 moves=23 last_combo=2 shuffled=false ice=0 frost=None obj=[0, 2] boss(none) out=InProgress stats=(2,1) curses=[] board=34a9d1c87c6f88a3",
    "a0: cleared combo=2 raw=225 runes=3 cleared=[1,1 2,1 3,1] fall=6:f9a430591986432c boss_hit=false phase2=None | score=595 fury=43 moves=23 last_combo=2 shuffled=false ice=0 frost=None obj=[0, 2] boss(none) out=InProgress stats=(5,2) curses=[] board=972eaa06292d89ea",
    "a0: ended frost=None curse_hits=[] riposte=[]None cursed_remain=false shuffled=false outcome=InProgress boosters_left=0 | score=595 fury=43 moves=23 last_combo=2 shuffled=false ice=0 frost=None obj=[0, 2] boss(none) out=InProgress stats=(5,2) curses=[] board=972eaa06292d89ea",
    "rng_next=10452571977219184393",
];

const S3: [&str; 5] = [
    "a0: pending combo=1 consumes_turn=true direct=false cells=[1,7 2,7 3,7] spawned=[] converted=[] fired=[] | score=0 fury=0 moves=23 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 0] boss(none) out=InProgress stats=(0,0) curses=[] board=366e1e3bdc61f2ff",
    "a0: cleared combo=1 raw=150 runes=3 cleared=[1,7 2,7 3,7] fall=23:70dfd487e38252ae boss_hit=false phase2=None | score=150 fury=15 moves=23 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 0] boss(none) out=InProgress stats=(3,1) curses=[] board=cbf4406e74a58359",
    "a0: relics positions=[3,7] fall=7:a843fafb3d91a69c | score=500 fury=15 moves=23 last_combo=1 shuffled=false ice=0 frost=None obj=[1, 0] boss(none) out=InProgress stats=(3,1) curses=[] board=0927d40d9fb861d1",
    "a0: ended frost=None curse_hits=[] riposte=[]None cursed_remain=false shuffled=false outcome=InProgress boosters_left=0 | score=500 fury=15 moves=23 last_combo=1 shuffled=false ice=0 frost=None obj=[1, 0] boss(none) out=InProgress stats=(3,1) curses=[] board=0927d40d9fb861d1",
    "rng_next=17172080263765897934",
];

const S4: [&str; 10] = [
    "a0: pending combo=1 consumes_turn=true direct=false cells=[2,0 3,0 4,0] spawned=[] converted=[] fired=[] | score=0 fury=0 moves=25 last_combo=1 shuffled=false ice=0 frost=None obj=[0] boss(hp=2567 riposte_in=3 last_dmg=0 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(0,0) curses=[] board=93940dd327f5392f",
    "a0: cleared combo=1 raw=150 runes=3 cleared=[2,0 3,0 4,0] fall=3:2ae1a426ac785bc1 boss_hit=true phase2=None | score=150 fury=15 moves=25 last_combo=1 shuffled=false ice=0 frost=None obj=[2068] boss(hp=2432 riposte_in=3 last_dmg=135 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(3,1) curses=[] board=209d91fb383aabb8",
    "a0: pending combo=2 consumes_turn=true direct=false cells=[4,0 4,1 4,2] spawned=[] converted=[] fired=[] | score=150 fury=15 moves=25 last_combo=2 shuffled=false ice=0 frost=None obj=[2068] boss(hp=2432 riposte_in=3 last_dmg=135 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(3,1) curses=[] board=209d91fb383aabb8",
    "a0: cleared combo=2 raw=225 runes=3 cleared=[4,0 4,1 4,2] fall=3:a36874344d86a24c boss_hit=true phase2=Some(\"boss.fenrir.name\") | score=375 fury=38 moves=25 last_combo=2 shuffled=false ice=0 frost=None obj=[2433] boss(hp=2067 riposte_in=3 last_dmg=365 p2=true p2_now=false riposte_cells=[]) out=InProgress stats=(6,2) curses=[] board=c104791428261584",
    "a0: pending combo=3 consumes_turn=true direct=false cells=[4,1 2,2 3,2 4,2 4,3] spawned=[(CellPos { col: 4, row: 1 }, Wood, Bomb)] converted=[] fired=[] | score=375 fury=38 moves=25 last_combo=3 shuffled=false ice=0 frost=None obj=[2433] boss(hp=2067 riposte_in=3 last_dmg=365 p2=true p2_now=false riposte_cells=[]) out=InProgress stats=(6,2) curses=[] board=c104791428261584",
    "a0: cleared combo=3 raw=940 runes=4 cleared=[4,1 2,2 3,2 4,2 4,3] fall=9:6c7b767d05f8ccbb boss_hit=true phase2=None | score=1315 fury=94 moves=25 last_combo=3 shuffled=false ice=1 frost=None obj=[3028] boss(hp=1472 riposte_in=3 last_dmg=595 p2=true p2_now=false riposte_cells=[]) out=InProgress stats=(10,3) curses=[] board=7279b9c6ba3bd24f",
    "a0: pending combo=4 consumes_turn=true direct=false cells=[2,0 3,0 4,0 2,1 2,2] spawned=[(CellPos { col: 2, row: 0 }, Gold, Bomb)] converted=[] fired=[] | score=1315 fury=94 moves=25 last_combo=4 shuffled=false ice=1 frost=None obj=[3028] boss(hp=1472 riposte_in=3 last_dmg=595 p2=true p2_now=false riposte_cells=[]) out=InProgress stats=(10,3) curses=[] board=7279b9c6ba3bd24f",
    "a0: cleared combo=4 raw=625 runes=5 cleared=[2,0 3,0 4,0 2,1 2,2] fall=5:9da4b0821a0d2cb1 boss_hit=true phase2=None | score=1940 fury=100 moves=25 last_combo=4 shuffled=false ice=1 frost=None obj=[3489] boss(hp=1011 riposte_in=3 last_dmg=461 p2=true p2_now=false riposte_cells=[]) out=InProgress stats=(15,4) curses=[] board=c7657d2533aada2a",
    "a0: ended frost=None curse_hits=[] riposte=[]None cursed_remain=false shuffled=false outcome=InProgress boosters_left=2 | score=1940 fury=100 moves=25 last_combo=4 shuffled=false ice=1 frost=None obj=[3489] boss(hp=1011 riposte_in=2 last_dmg=461 p2=true p2_now=false riposte_cells=[]) out=InProgress stats=(15,4) curses=[] board=c7657d2533aada2a",
    "rng_next=12022541424913748353",
];

const S5: [&str; 4] = [
    "a0: pending combo=1 consumes_turn=true direct=false cells=[0,2 1,2 2,2] spawned=[] converted=[] fired=[] | score=0 fury=0 moves=25 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 0] boss(none) out=InProgress stats=(0,0) curses=[] board=be4462ed895e36ca",
    "a0: cleared combo=1 raw=150 runes=3 cleared=[0,2 1,2 2,2] fall=9:fc860b1931c97095 boss_hit=false phase2=None | score=150 fury=15 moves=25 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 0] boss(none) out=InProgress stats=(3,1) curses=[] board=4548aa34cce01cde",
    "a0: ended frost=Some((0, 3)) curse_hits=[] riposte=[]None cursed_remain=false shuffled=true outcome=InProgress boosters_left=0 | score=150 fury=15 moves=25 last_combo=1 shuffled=true ice=0 frost=Some((0, 3)) obj=[0, 0] boss(none) out=InProgress stats=(3,1) curses=[] board=94935737e325241f",
    "rng_next=5818779186642849583",
];

const S6: [&str; 7] = [
    "a0: pending combo=1 consumes_turn=false direct=true cells=[4,1] spawned=[] converted=[] fired=[] | score=0 fury=0 moves=30 last_combo=0 shuffled=false ice=0 frost=None obj=[0, 0, 0] boss(none) out=InProgress stats=(0,0) curses=[2,4:5 5,4:6] board=efc353127af0e025",
    "a0: cleared combo=1 raw=50 runes=1 cleared=[4,1] fall=2:4ffb5ba638518994 boss_hit=false phase2=None | score=50 fury=5 moves=30 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 0, 50] boss(none) out=InProgress stats=(1,1) curses=[2,4:5 5,4:6] board=7b646cf2a05672c9",
    "a0: ended frost=None curse_hits=[] riposte=[]None cursed_remain=true shuffled=false outcome=InProgress boosters_left=0 | score=50 fury=5 moves=30 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 0, 50] boss(none) out=InProgress stats=(1,1) curses=[2,4:5 5,4:6] board=7b646cf2a05672c9",
    "a1: pending combo=1 consumes_turn=true direct=false cells=[5,0 5,1 5,2] spawned=[] converted=[] fired=[] | score=50 fury=5 moves=29 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 0, 50] boss(none) out=InProgress stats=(1,1) curses=[2,4:5 5,4:6] board=c074cdb2d880b487",
    "a1: cleared combo=1 raw=270 runes=3 cleared=[5,0 5,1 5,2] fall=3:2d96313603449857 boss_hit=false phase2=None | score=320 fury=20 moves=29 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 0, 320] boss(none) out=InProgress stats=(4,1) curses=[2,4:5 5,4:6] board=00c7683aec4b9995",
    "a1: ended frost=Some((4, 3)) curse_hits=[] riposte=[]None cursed_remain=true shuffled=false outcome=InProgress boosters_left=0 | score=320 fury=20 moves=29 last_combo=1 shuffled=false ice=0 frost=Some((4, 3)) obj=[0, 0, 320] boss(none) out=InProgress stats=(4,1) curses=[2,4:4 5,4:5] board=79f40498655c7c2e",
    "rng_next=11017302231760453023",
];

const S7: [&str; 8] = [
    "a0: pending combo=1 consumes_turn=false direct=true cells=[3,3] spawned=[] converted=[] fired=[] | score=0 fury=0 moves=24 last_combo=0 shuffled=false ice=0 frost=None obj=[0, 0] boss(none) out=InProgress stats=(0,0) curses=[] board=a36835b4eaf96b70",
    "a0: cleared combo=1 raw=200 runes=1 cleared=[3,3] fall=4:fa4c3ed09387740a boss_hit=false phase2=None | score=200 fury=15 moves=24 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 0] boss(none) out=InProgress stats=(1,1) curses=[] board=fd363c4f2138d30f",
    "a0: ended frost=None curse_hits=[] riposte=[]None cursed_remain=false shuffled=false outcome=InProgress boosters_left=0 | score=200 fury=15 moves=24 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 0] boss(none) out=InProgress stats=(1,1) curses=[] board=fd363c4f2138d30f",
    "a1: loki valid=true shuffled=true | score=200 fury=15 moves=24 last_combo=0 shuffled=true ice=0 frost=None obj=[0, 0] boss(none) out=InProgress stats=(1,1) curses=[] board=64948e0c184914b5",
    "a3: pending combo=1 consumes_turn=false direct=true cells=[0,0] spawned=[] converted=[] fired=[] | score=200 fury=15 moves=0 last_combo=0 shuffled=true ice=0 frost=None obj=[0, 0] boss(none) out=InProgress stats=(1,1) curses=[] board=64948e0c184914b5",
    "a3: cleared combo=1 raw=50 runes=1 cleared=[0,0] fall=1:7f8aa49e1acb2d8a boss_hit=false phase2=None | score=250 fury=20 moves=0 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 0] boss(none) out=InProgress stats=(2,1) curses=[] board=56fe48ca57e739fb",
    "a3: ended frost=None curse_hits=[] riposte=[]None cursed_remain=false shuffled=false outcome=Defeat boosters_left=0 | score=250 fury=20 moves=0 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 0] boss(none) out=Defeat stats=(2,1) curses=[] board=56fe48ca57e739fb",
    "rng_next=11593007456998153698",
];

const S8: [&str; 17] = [
    "a1: fury patron=Thor started_turn=true targets=[2,2 4,3 3,4 4,4] | score=0 fury=0 moves=26 last_combo=0 shuffled=false ice=0 frost=None obj=[0] boss(hp=4500 riposte_in=1 last_dmg=0 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(0,0) curses=[] board=2b534650a5b6e970",
    "a1: pending combo=1 consumes_turn=false direct=true cells=[2,2 4,3 3,4 4,4] spawned=[] converted=[] fired=[] | score=0 fury=0 moves=26 last_combo=0 shuffled=false ice=0 frost=None obj=[0] boss(hp=4500 riposte_in=1 last_dmg=0 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(0,0) curses=[] board=2b534650a5b6e970",
    "a1: cleared combo=1 raw=770 runes=4 cleared=[2,2 4,3 3,4 4,4] fall=12:e4ea52f040d795cf boss_hit=true phase2=None | score=770 fury=60 moves=26 last_combo=1 shuffled=false ice=1 frost=None obj=[565] boss(hp=3935 riposte_in=1 last_dmg=565 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(4,1) curses=[] board=6fd5c37d802e01a2",
    "a1: ended frost=None curse_hits=[] riposte=[]None cursed_remain=false shuffled=false outcome=InProgress boosters_left=0 | score=770 fury=60 moves=26 last_combo=1 shuffled=false ice=1 frost=None obj=[565] boss(hp=3935 riposte_in=1 last_dmg=565 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(4,1) curses=[] board=6fd5c37d802e01a2",
    "a4: fury patron=Freya started_turn=true targets=[5,0 1,1 3,1 7,1 2,2 6,2 3,3 7,3 4,5 3,6 4,6 6,6 2,7 6,7] | score=770 fury=0 moves=26 last_combo=1 shuffled=false ice=1 frost=None obj=[565] boss(hp=3935 riposte_in=1 last_dmg=565 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(4,1) curses=[] board=6fd5c37d802e01a2",
    "a4: pending combo=1 consumes_turn=false direct=true cells=[5,0 1,1 3,1 7,1 2,2 6,2 3,3 7,3 4,5 3,6 4,6 6,6 2,7 6,7] spawned=[] converted=[] fired=[] | score=770 fury=0 moves=26 last_combo=1 shuffled=false ice=1 frost=None obj=[565] boss(hp=3935 riposte_in=1 last_dmg=565 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(4,1) curses=[] board=6fd5c37d802e01a2",
    "a4: cleared combo=1 raw=850 runes=14 cleared=[5,0 1,1 3,1 7,1 2,2 6,2 3,3 7,3 4,5 3,6 4,6 6,6 2,7 6,7] fall=36:bd0186e00b43f339 boss_hit=true phase2=None | score=1620 fury=80 moves=26 last_combo=1 shuffled=false ice=1 frost=None obj=[1280] boss(hp=3220 riposte_in=1 last_dmg=715 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(18,1) curses=[] board=c4d5b6656c90bf06",
    "a4: pending combo=2 consumes_turn=false direct=false cells=[0,2 1,2 2,2 3,3 4,3 5,3 1,6 2,6 3,6 4,6 5,6] spawned=[(CellPos { col: 1, row: 6 }, Wood, BifrostShard)] converted=[] fired=[] | score=1620 fury=80 moves=26 last_combo=2 shuffled=false ice=1 frost=None obj=[1280] boss(hp=3220 riposte_in=1 last_dmg=715 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(18,1) curses=[] board=c4d5b6656c90bf06",
    "a4: cleared combo=2 raw=825 runes=11 cleared=[0,2 1,2 2,2 3,3 4,3 5,3 1,6 2,6 3,6 4,6 5,6] fall=33:edc069d2d50c3834 boss_hit=true phase2=None | score=2445 fury=100 moves=26 last_combo=2 shuffled=false ice=1 frost=None obj=[2131] boss(hp=2369 riposte_in=1 last_dmg=851 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(29,2) curses=[] board=ed0fea8fa571a9cc",
    "a4: pending combo=3 consumes_turn=false direct=false cells=[2,3 5,3 6,3 7,3 2,4 5,4 0,5 1,5 2,5 4,5 5,5 6,5] spawned=[(CellPos { col: 5, row: 3 }, Stone, Bomb), (CellPos { col: 2, row: 3 }, Gold, Bomb)] converted=[] fired=[] | score=2445 fury=100 moves=26 last_combo=3 shuffled=false ice=1 frost=None obj=[2131] boss(hp=2369 riposte_in=1 last_dmg=851 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(29,2) curses=[] board=ed0fea8fa571a9cc",
    "a4: cleared combo=3 raw=1680 runes=12 cleared=[2,3 5,3 6,3 7,3 2,4 5,4 0,5 1,5 2,5 4,5 5,5 6,5] fall=38:fefdeaeecfb8ad07 boss_hit=true phase2=Some(\"boss.fenrir.name\") | score=4125 fury=100 moves=26 last_combo=3 shuffled=false ice=3 frost=None obj=[3338] boss(hp=1162 riposte_in=1 last_dmg=1207 p2=true p2_now=false riposte_cells=[]) out=InProgress stats=(41,3) curses=[] board=9e4f394cbf0100c2",
    "a4: pending combo=4 consumes_turn=false direct=false cells=[5,0 5,1 5,2] spawned=[] converted=[] fired=[] | score=4125 fury=100 moves=26 last_combo=4 shuffled=false ice=3 frost=None obj=[3338] boss(hp=1162 riposte_in=1 last_dmg=1207 p2=true p2_now=false riposte_cells=[]) out=InProgress stats=(41,3) curses=[] board=9e4f394cbf0100c2",
    "a4: cleared combo=4 raw=675 runes=3 cleared=[5,0 5,1 5,2] fall=3:2d96313603449857 boss_hit=true phase2=None | score=4800 fury=100 moves=26 last_combo=4 shuffled=false ice=4 frost=None obj=[3789] boss(hp=711 riposte_in=1 last_dmg=451 p2=true p2_now=false riposte_cells=[]) out=InProgress stats=(44,4) curses=[] board=a1a7a8fb535d3ee4",
    "a4: ended frost=None curse_hits=[] riposte=[]None cursed_remain=false shuffled=false outcome=InProgress boosters_left=3 | score=4800 fury=100 moves=26 last_combo=4 shuffled=false ice=4 frost=None obj=[3789] boss(hp=711 riposte_in=1 last_dmg=451 p2=true p2_now=false riposte_cells=[]) out=InProgress stats=(44,4) curses=[] board=a1a7a8fb535d3ee4",
    "a7: fury patron=Loki started_turn=false targets=[3,6 5,6] | score=4800 fury=0 moves=26 last_combo=4 shuffled=false ice=4 frost=None obj=[3789] boss(hp=711 riposte_in=1 last_dmg=451 p2=true p2_now=false riposte_cells=[]) out=InProgress stats=(44,4) curses=[] board=7afbd63903add7da",
    "a10: fury patron=Odin started_turn=false targets=[3,4] | score=4800 fury=0 moves=28 last_combo=4 shuffled=false ice=4 frost=None obj=[3789] boss(hp=711 riposte_in=1 last_dmg=451 p2=true p2_now=false riposte_cells=[]) out=InProgress stats=(44,4) curses=[] board=ba0cef584d617d0c",
    "rng_next=2151995562853640017",
];

const S9: [&str; 7] = [
    "a0: pending combo=1 consumes_turn=true direct=false cells=[8,1 9,1 10,1] spawned=[] converted=[] fired=[(CellPos { col: 10, row: 1 }, Bomb)] | score=0 fury=0 moves=23 last_combo=1 shuffled=false ice=0 frost=None obj=[0] boss(none) out=InProgress stats=(0,0) curses=[] board=eed33025de37c5dc",
    "a0: cleared combo=1 raw=750 runes=10 cleared=[9,0 10,0 11,0 8,1 9,1 10,1 11,1 9,2 10,2 11,2] fall=11:8436f367f44e92d3 boss_hit=false phase2=None | score=750 fury=50 moves=23 last_combo=1 shuffled=false ice=0 frost=None obj=[1] boss(none) out=InProgress stats=(10,1) curses=[] board=af5bda44d08ecd6a",
    "a0: ended frost=None curse_hits=[] riposte=[]None cursed_remain=false shuffled=false outcome=Victory boosters_left=24 | score=5350 fury=50 moves=23 last_combo=1 shuffled=false ice=0 frost=None obj=[1] boss(none) out=Victory stats=(10,1) curses=[] board=af5bda44d08ecd6a",
    "a1: burst at=1,1 booster=Bomb raw=1050 cleared=[0,0 1,0 2,0 0,1 1,1 2,1 0,2 1,2 2,2] | score=6400 fury=100 moves=23 last_combo=2 shuffled=false ice=0 frost=None obj=[1] boss(none) out=Victory stats=(10,1) curses=[] board=092a4d56d1ae0e12",
    "a2: burst at=4,1 booster=Bomb raw=1050 cleared=[3,0 4,0 5,0 3,1 4,1 5,1 3,2 4,2 5,2] | score=7450 fury=100 moves=23 last_combo=2 shuffled=false ice=0 frost=None obj=[1] boss(none) out=Victory stats=(10,1) curses=[] board=3e09ec1d7641ef7d",
    "a3: skip detonations=16 boosters_left=6 | score=24250 fury=100 moves=23 last_combo=2 shuffled=false ice=0 frost=None obj=[1] boss(none) out=Victory stats=(10,1) curses=[] board=a31103432f9b1f40",
    "rng_next=17827638624650655137",
];

const S10: [&str; 6] = [
    "a0: pending combo=1 consumes_turn=true direct=true cells=[0,0 1,0 6,2 0,3 6,4 7,4 5,6 5,7 7,7] spawned=[] converted=[(CellPos { col: 0, row: 0 }, LineHorizontal), (CellPos { col: 6, row: 2 }, LineVertical), (CellPos { col: 0, row: 3 }, LineHorizontal), (CellPos { col: 6, row: 4 }, LineVertical), (CellPos { col: 7, row: 4 }, LineHorizontal), (CellPos { col: 5, row: 6 }, LineVertical), (CellPos { col: 5, row: 7 }, LineHorizontal), (CellPos { col: 7, row: 7 }, LineVertical)] fired=[(CellPos { col: 0, row: 0 }, LineHorizontal), (CellPos { col: 1, row: 0 }, BifrostShard)] | score=0 fury=0 moves=23 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 0] boss(none) out=InProgress stats=(0,0) curses=[] board=d24af4f71f3a8e82",
    "a0: cleared combo=1 raw=4400 runes=43 cleared=[0,0 2,0 3,0 4,0 5,0 6,0 7,0 5,1 6,1 7,1 5,2 6,2 7,2 0,3 1,3 2,3 3,3 4,3 5,3 6,3 7,3 0,4 1,4 2,4 3,4 4,4 5,4 6,4 7,4 5,5 6,5 7,5 5,6 6,6 7,6 0,7 1,7 2,7 3,7 4,7 5,7 6,7 7,7] fall=64:5e485d307ad87fd9 boss_hit=false phase2=None | score=4400 fury=100 moves=23 last_combo=1 shuffled=false ice=0 frost=None obj=[0, 7] boss(none) out=InProgress stats=(43,1) curses=[] board=bf4b12b478adfa5c",
    "a0: pending combo=2 consumes_turn=true direct=false cells=[6,5 6,6 6,7] spawned=[] converted=[] fired=[] | score=4400 fury=100 moves=23 last_combo=2 shuffled=false ice=0 frost=None obj=[0, 7] boss(none) out=InProgress stats=(43,1) curses=[] board=bf4b12b478adfa5c",
    "a0: cleared combo=2 raw=225 runes=3 cleared=[6,5 6,6 6,7] fall=8:596027404b72cdac boss_hit=false phase2=None | score=4625 fury=100 moves=23 last_combo=2 shuffled=false ice=0 frost=None obj=[0, 7] boss(none) out=InProgress stats=(46,2) curses=[] board=66f252020e156dd0",
    "a0: ended frost=None curse_hits=[] riposte=[]None cursed_remain=false shuffled=false outcome=InProgress boosters_left=0 | score=4625 fury=100 moves=23 last_combo=2 shuffled=false ice=0 frost=None obj=[0, 7] boss(none) out=InProgress stats=(46,2) curses=[] board=66f252020e156dd0",
    "rng_next=13022825323231516737",
];

const S11: [&str; 19] = [
    "a0: pending combo=1 consumes_turn=true direct=false cells=[1,1 2,1 3,1] spawned=[] converted=[] fired=[] | score=0 fury=0 moves=25 last_combo=1 shuffled=false ice=0 frost=None obj=[0] boss(hp=4500 riposte_in=3 last_dmg=0 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(0,0) curses=[] board=843ff1ae7df0326c",
    "a0: cleared combo=1 raw=150 runes=3 cleared=[1,1 2,1 3,1] fall=6:f9a430591986432c boss_hit=true phase2=None | score=150 fury=15 moves=25 last_combo=1 shuffled=false ice=0 frost=None obj=[135] boss(hp=4365 riposte_in=3 last_dmg=135 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(3,1) curses=[] board=ba89bfce84b9de09",
    "a0: ended frost=None curse_hits=[] riposte=[]None cursed_remain=false shuffled=false outcome=InProgress boosters_left=0 | score=150 fury=15 moves=25 last_combo=1 shuffled=false ice=0 frost=None obj=[135] boss(hp=4365 riposte_in=2 last_dmg=135 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(3,1) curses=[] board=ba89bfce84b9de09",
    "a1: pending combo=1 consumes_turn=false direct=true cells=[3,3] spawned=[] converted=[] fired=[] | score=150 fury=15 moves=25 last_combo=1 shuffled=false ice=0 frost=None obj=[135] boss(hp=4365 riposte_in=2 last_dmg=135 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(3,1) curses=[] board=ba89bfce84b9de09",
    "a1: cleared combo=1 raw=200 runes=1 cleared=[3,3] fall=4:fa4c3ed09387740a boss_hit=true phase2=None | score=350 fury=30 moves=25 last_combo=1 shuffled=false ice=0 frost=None obj=[265] boss(hp=4235 riposte_in=2 last_dmg=130 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(4,1) curses=[] board=aec388ae9d9fcafc",
    "a1: ended frost=None curse_hits=[] riposte=[]None cursed_remain=false shuffled=false outcome=InProgress boosters_left=0 | score=350 fury=30 moves=25 last_combo=1 shuffled=false ice=0 frost=None obj=[265] boss(hp=4235 riposte_in=2 last_dmg=130 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(4,1) curses=[] board=aec388ae9d9fcafc",
    "a3: fury patron=Thor started_turn=true targets=[5,2 3,4 4,4 5,5] | score=350 fury=0 moves=25 last_combo=1 shuffled=false ice=0 frost=None obj=[265] boss(hp=4235 riposte_in=2 last_dmg=130 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(4,1) curses=[] board=aec388ae9d9fcafc",
    "a3: pending combo=1 consumes_turn=false direct=true cells=[5,2 3,4 4,4 5,5] spawned=[] converted=[] fired=[] | score=350 fury=0 moves=25 last_combo=1 shuffled=false ice=0 frost=None obj=[265] boss(hp=4235 riposte_in=2 last_dmg=130 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(4,1) curses=[] board=aec388ae9d9fcafc",
    "a3: cleared combo=1 raw=740 runes=4 cleared=[5,2 3,4 4,4 5,5] fall=15:d276926b8b0aae5b boss_hit=true phase2=None | score=1090 fury=60 moves=25 last_combo=1 shuffled=false ice=2 frost=None obj=[830] boss(hp=3670 riposte_in=2 last_dmg=565 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(8,1) curses=[] board=cf0553cc99fdb57d",
    "a3: ended frost=None curse_hits=[] riposte=[]None cursed_remain=false shuffled=false outcome=InProgress boosters_left=0 | score=1090 fury=60 moves=25 last_combo=1 shuffled=false ice=2 frost=None obj=[830] boss(hp=3670 riposte_in=2 last_dmg=565 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(8,1) curses=[] board=cf0553cc99fdb57d",
    "a4: pending combo=1 consumes_turn=true direct=false cells=[5,2 6,2 7,2] spawned=[] converted=[] fired=[] | score=1090 fury=60 moves=24 last_combo=1 shuffled=false ice=0 frost=None obj=[830] boss(hp=3670 riposte_in=2 last_dmg=565 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(8,1) curses=[] board=1518ea1ce3fd394d",
    "a4: cleared combo=1 raw=150 runes=3 cleared=[5,2 6,2 7,2] fall=8:d0999fa80822d282 boss_hit=true phase2=None | score=1240 fury=75 moves=24 last_combo=1 shuffled=false ice=0 frost=None obj=[965] boss(hp=3535 riposte_in=2 last_dmg=135 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(11,1) curses=[] board=9273722807bf10a3",
    "a4: ended frost=None curse_hits=[] riposte=[]None cursed_remain=false shuffled=false outcome=InProgress boosters_left=0 | score=1240 fury=75 moves=24 last_combo=1 shuffled=false ice=0 frost=None obj=[965] boss(hp=3535 riposte_in=1 last_dmg=135 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(11,1) curses=[] board=9273722807bf10a3",
    "a5: pending combo=1 consumes_turn=true direct=false cells=[2,0 2,1 2,2] spawned=[] converted=[] fired=[] | score=1240 fury=75 moves=23 last_combo=1 shuffled=false ice=0 frost=None obj=[965] boss(hp=3535 riposte_in=1 last_dmg=135 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(11,1) curses=[] board=f7a65b67b748b7e3",
    "a5: cleared combo=1 raw=270 runes=3 cleared=[2,0 2,1 2,2] fall=3:fa47169af64a7276 boss_hit=true phase2=None | score=1510 fury=100 moves=23 last_combo=1 shuffled=false ice=1 frost=None obj=[1185] boss(hp=3315 riposte_in=1 last_dmg=220 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(14,1) curses=[] board=8caebbac13b3de01",
    "a5: pending combo=2 consumes_turn=true direct=false cells=[0,2 1,2 2,2] spawned=[] converted=[] fired=[] | score=1510 fury=100 moves=23 last_combo=2 shuffled=false ice=1 frost=None obj=[1185] boss(hp=3315 riposte_in=1 last_dmg=220 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(14,1) curses=[] board=8caebbac13b3de01",
    "a5: cleared combo=2 raw=225 runes=3 cleared=[0,2 1,2 2,2] fall=8:e40cda60cc7b0904 boss_hit=true phase2=None | score=1735 fury=100 moves=23 last_combo=2 shuffled=false ice=1 frost=None obj=[1367] boss(hp=3133 riposte_in=1 last_dmg=182 p2=false p2_now=false riposte_cells=[]) out=InProgress stats=(17,2) curses=[] board=d75c875b6f0d17b6",
    "a5: ended frost=None curse_hits=[] riposte=[4,1 2,2]Some(\"boss.fenrir.name\") cursed_remain=false shuffled=false outcome=InProgress boosters_left=0 | score=1735 fury=100 moves=23 last_combo=2 shuffled=false ice=1 frost=None obj=[1367] boss(hp=3133 riposte_in=3 last_dmg=182 p2=false p2_now=false riposte_cells=[4,1 2,2]) out=InProgress stats=(17,2) curses=[] board=c50155f854d4d370",
    "rng_next=8982417080430441189",
];
