// Copyright (C) 2017 Hove and/or its affiliates.
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License as published by the
// Free Software Foundation, version 3.

// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.

// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>

use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};
use tracing::warn;

// For the order, see
// https://github.com/hove-io/ntfs-specification/blob/v0.11.2/ntfs_fr.md#physical_modestxt-requis
// Note that 2 enum cannot have the same value so `Funicular` and `Cableway`
// have different values. Same for `Coach` and `Bus`.
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Clone, Copy)]
pub enum NetexMode {
    Air = 1,
    Water = 2,
    Rail = 3,
    Metro = 4,
    Tram = 5,
    Funicular = 6,
    Cableway = 7,
    Coach = 8,
    Bus = 9,
}

impl Display for NetexMode {
    fn fmt(&self, f: &mut Formatter) -> std::result::Result<(), fmt::Error> {
        use NetexMode::*;
        match self {
            Air => write!(f, "air"),
            Bus => write!(f, "bus"),
            Cableway => write!(f, "cableway"),
            Coach => write!(f, "coach"),
            Funicular => write!(f, "funicular"),
            Metro => write!(f, "metro"),
            Rail => write!(f, "rail"),
            Tram => write!(f, "tram"),
            Water => write!(f, "water"),
        }
    }
}

impl NetexMode {
    pub fn from_physical_mode_id(physical_mode_id: &str) -> Option<NetexMode> {
        use NetexMode::*;
        match physical_mode_id {
            "Air" => Some(Air),
            "Boat" => Some(Water),
            "Bus" => Some(Bus),
            "BusRapidTransit" => Some(Bus),
            "Coach" => Some(Coach),
            "Ferry" => Some(Water),
            "Funicular" => Some(Funicular),
            "LocalTrain" => Some(Rail),
            "LongDistanceTrain" => Some(Rail),
            "Metro" => Some(Metro),
            "RapidTransit" => Some(Rail),
            "RailShuttle" => Some(Rail),
            "Shuttle" => Some(Bus),
            "SuspendedCableCar" => Some(Cableway),
            "Train" => Some(Rail),
            "Tramway" => Some(Tram),
            mode => {
                warn!(
                    "Physical Mode '{}' is not supported for NeTEx France export.",
                    mode
                );
                None
            }
        }
    }
    pub fn calculate_highest_mode(netex_modes: &BTreeSet<NetexMode>) -> Option<NetexMode> {
        // Since `BTreeSet is ordered, the first one in the list is of highest priority
        netex_modes.iter().next().cloned()
    }
}
