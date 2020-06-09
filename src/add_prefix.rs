// Copyright (C) 2017 Kisio Digital and/or its affiliates.
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

//! A trait for every structure that needs to be updated with a prefix

use crate::model::Collections;
use std::collections::HashMap;
use typed_index_collection::{Collection, CollectionWithId, Id};

/// Trait for object that can be prefixed
pub trait AddPrefix {
    /// Add the prefix to all elements of the object that needs to be prefixed.
    fn add_prefix(&mut self, prefix: &str);
    /// Add the prefix to all elements of the object that needs to be prefixed.
    /// A separator will be placed between the prefix and the identifier.
    fn add_prefix_with_sep(&mut self, prefix: &str, sep: &str) {
        let prefix = format!("{}{}", prefix, sep);
        self.add_prefix(&prefix);
    }
}

impl<T> AddPrefix for Collection<T>
where
    T: AddPrefix,
{
    fn add_prefix(&mut self, prefix: &str) {
        for obj in &mut self.values_mut() {
            obj.add_prefix(prefix);
        }
    }
}

impl<T> AddPrefix for CollectionWithId<T>
where
    T: Id<T> + AddPrefix,
{
    fn add_prefix(&mut self, prefix: &str) {
        let indexes: Vec<_> = self.iter().map(|(idx, _)| idx).collect();
        for index in indexes {
            self.index_mut(index).add_prefix(prefix);
        }
    }
}

fn add_prefix_on_vehicle_journey_ids(
    vehicle_journey_ids: &HashMap<(String, u32), String>,
    prefix: &str,
) -> HashMap<(String, u32), String> {
    vehicle_journey_ids
        .iter()
        .map(|((trip_id, sequence), value)| {
            (
                (format!("{}{}", prefix, trip_id), *sequence),
                value.to_string(),
            )
        })
        .collect()
}

fn add_prefix_on_vehicle_journey_ids_and_values(
    vehicle_journey_ids: &HashMap<(String, u32), String>,
    prefix: &str,
) -> HashMap<(String, u32), String> {
    vehicle_journey_ids
        .iter()
        .map(|((trip_id, sequence), value)| {
            (
                (format!("{}{}", prefix, trip_id), *sequence),
                format!("{}{}", prefix, value.to_string()),
            )
        })
        .collect()
}

impl AddPrefix for Collections {
    fn add_prefix(&mut self, prefix: &str) {
        self.contributors.add_prefix(&prefix);
        self.datasets.add_prefix(&prefix);
        self.networks.add_prefix(&prefix);
        self.lines.add_prefix(&prefix);
        self.routes.add_prefix(&prefix);
        self.vehicle_journeys.add_prefix(&prefix);
        self.frequencies.add_prefix(&prefix);
        self.stop_areas.add_prefix(&prefix);
        self.stop_points.add_prefix(&prefix);
        self.stop_locations.add_prefix(&prefix);
        self.calendars.add_prefix(&prefix);
        self.companies.add_prefix(&prefix);
        self.comments.add_prefix(&prefix);
        self.equipments.add_prefix(&prefix);
        self.transfers.add_prefix(&prefix);
        self.trip_properties.add_prefix(&prefix);
        self.geometries.add_prefix(&prefix);
        self.admin_stations.add_prefix(&prefix);
        self.prices_v1.add_prefix(&prefix);
        self.od_fares_v1.add_prefix(&prefix);
        self.fares_v1.add_prefix(&prefix);
        self.tickets.add_prefix(&prefix);
        self.ticket_prices.add_prefix(&prefix);
        self.ticket_uses.add_prefix(&prefix);
        self.ticket_use_perimeters.add_prefix(&prefix);
        self.ticket_use_restrictions.add_prefix(&prefix);
        self.pathways.add_prefix(&prefix);
        self.levels.add_prefix(&prefix);
        self.grid_calendars.add_prefix(&prefix);
        self.grid_exception_dates.add_prefix(&prefix);
        self.grid_periods.add_prefix(&prefix);
        self.grid_rel_calendar_line.add_prefix(&prefix);
        #[cfg(not(feature = "stop_time"))]
        {
            self.stop_time_headsigns =
                add_prefix_on_vehicle_journey_ids(&self.stop_time_headsigns, &prefix);
            self.stop_time_ids =
                add_prefix_on_vehicle_journey_ids_and_values(&self.stop_time_ids, &prefix);
            self.stop_time_comments =
                add_prefix_on_vehicle_journey_ids_and_values(&self.stop_time_comments, &prefix);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    struct Obj(String);
    impl Id<Obj> for Obj {
        fn id(&self) -> &str {
            self.0.as_str()
        }
        fn set_id(&mut self, _id: String) {
            unimplemented!()
        }
    }
    impl AddPrefix for Obj {
        fn add_prefix(&mut self, prefix: &str) {
            self.0 = format!("{}:{}", prefix, self.0);
        }
    }

    #[test]
    fn collection() {
        let obj1 = Obj(String::from("some_id"));
        let obj2 = Obj(String::from("other_id"));
        let mut collection = Collection::new(vec![obj1, obj2]);
        collection.add_prefix("pre");
        let mut values = collection.values();
        let element = values.next().unwrap();
        assert_eq!(String::from("pre:some_id"), element.0);
        let element = values.next().unwrap();
        assert_eq!(String::from("pre:other_id"), element.0);
    }

    #[test]
    fn collection_with_id() {
        let obj1 = Obj(String::from("some_id"));
        let obj2 = Obj(String::from("other_id"));
        let mut collection = CollectionWithId::new(vec![obj1, obj2]).unwrap();
        collection.add_prefix("pre");
        let mut values = collection.values();
        let element = values.next().unwrap();
        assert_eq!(String::from("pre:some_id"), element.0);
        let element = values.next().unwrap();
        assert_eq!(String::from("pre:other_id"), element.0);
    }
}
