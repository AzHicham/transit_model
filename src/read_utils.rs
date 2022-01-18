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
//! Some utilities for input dataset to the library.

use crate::{
    objects::{self, Contributor},
    Result,
};
use anyhow::{anyhow, bail, Context};
use serde::Deserialize;
use skip_error::SkipError;
use std::path;
use std::path::{Path, PathBuf};
use std::{collections::BTreeMap, io::Read};
use std::{fs::File, io::Seek};
use tracing::info;
use typed_index_collection::{CollectionWithId, Id};

#[derive(Deserialize, Debug)]
struct ConfigDataset {
    dataset_id: String,
}

#[derive(Deserialize, Debug)]
struct Config {
    contributor: objects::Contributor,
    dataset: ConfigDataset,
    feed_infos: Option<BTreeMap<String, String>>,
}

/// Read a JSON configuration file to facilitate the creation of:
/// - a Contributor
/// - a Dataset
/// - a list of key/value which will be used in 'feed_infos.txt'
/// Below is an example of this file
/// ```text
/// {
///     "contributor": {
///         "contributor_id": "contributor_id",
///         "contributor_name": "Contributor Name",
///         "contributor_license": "AGPIT",
///         "contributor_website": "http://www.datasource-website.com"
///     },
///     "dataset": {
///         "dataset_id": "dataset-id"
///     },
///     "feed_infos": {
///         "feed_publisher_name": "The Great Data Publisher",
///         "feed_license": "AGPIT",
///         "feed_license_url": "http://www.datasource-website.com",
///         "tartare_platform": "dev",
///         "tartare_contributor_id": "contributor_id"
///     }
/// }
/// ```
pub fn read_config<P: AsRef<path::Path>>(
    config_path: Option<P>,
) -> Result<(
    objects::Contributor,
    objects::Dataset,
    BTreeMap<String, String>,
)> {
    let contributor;
    let dataset;
    let mut feed_infos = BTreeMap::default();

    if let Some(config_path) = config_path {
        let config_path = config_path.as_ref();
        info!("Reading dataset and contributor from {:?}", config_path);
        let json_config_file = File::open(config_path)?;
        let config: Config = serde_json::from_reader(json_config_file)?;

        contributor = config.contributor;
        dataset = objects::Dataset::new(config.dataset.dataset_id, contributor.id.clone());
        if let Some(config_feed_infos) = config.feed_infos {
            feed_infos = config_feed_infos;
        }
    } else {
        contributor = Contributor::default();
        dataset = objects::Dataset::default();
    }

    Ok((contributor, dataset, feed_infos))
}

/// Allows files in a directory or ZipArchive to be read either
pub trait FileHandler
where
    Self: std::marker::Sized,
{
    /// Reader
    type Reader: Read;

    /// Return a file if exist
    fn get_file_if_exists(self, name: &str) -> Result<(Option<Self::Reader>, PathBuf)>;

    /// Return a file or an error if not exist
    fn get_file(self, name: &str) -> Result<(Self::Reader, PathBuf)> {
        let (reader, path) = self.get_file_if_exists(name)?;
        Ok((
            reader.ok_or_else(|| anyhow!("file {:?} not found", path))?,
            path,
        ))
    }

    /// Allows to have nicer error messages
    fn source_name(&self) -> &str;
}

/// PathFileHandler is used to read files for a directory
pub struct PathFileHandler<P: AsRef<Path>> {
    base_path: P,
}

impl<P: AsRef<Path>> PathFileHandler<P> {
    /// Constructs a new PathFileHandler
    pub fn new(path: P) -> Self {
        PathFileHandler { base_path: path }
    }
}

impl<'a, P: AsRef<Path>> FileHandler for &'a mut PathFileHandler<P> {
    type Reader = File;
    fn get_file_if_exists(self, name: &str) -> Result<(Option<Self::Reader>, PathBuf)> {
        let f = self.base_path.as_ref().join(name);
        if f.exists() {
            Ok((
                Some(File::open(&f).with_context(|| format!("Error reading {:?}", &f))?),
                f,
            ))
        } else {
            Ok((None, f))
        }
    }
    fn source_name(&self) -> &str {
        self.base_path.as_ref().to_str().unwrap_or_else(|| {
            panic!(
                "the path '{:?}' should be valid UTF-8",
                self.base_path.as_ref()
            )
        })
    }
}

/// ZipHandler is a wrapper around a ZipArchive
/// It provides a way to access the archive's file by their names
///
/// Unlike ZipArchive, it gives access to a file by its name not regarding its path in the ZipArchive
/// It thus cannot be correct if there are 2 files with the same name in the archive,
/// but for transport data if will make it possible to handle a zip with a sub directory
pub struct ZipHandler<R: Seek + Read> {
    archive: zip::ZipArchive<R>,
    archive_path: PathBuf,
    index_by_name: BTreeMap<String, usize>,
}

impl<R> ZipHandler<R>
where
    R: Seek + Read,
{
    pub(crate) fn new<P: AsRef<Path>>(r: R, path: P) -> Result<Self> {
        let mut archive = zip::ZipArchive::new(r)?;
        Ok(ZipHandler {
            index_by_name: Self::files_by_name(&mut archive),
            archive,
            archive_path: path.as_ref().to_path_buf(),
        })
    }

    fn files_by_name(archive: &mut zip::ZipArchive<R>) -> BTreeMap<String, usize> {
        (0..archive.len())
            .filter_map(|i| {
                let file = archive.by_index(i).ok()?;
                // we get the name of the file, not regarding its path in the ZipArchive
                let real_name = Path::new(file.name()).file_name()?;
                let real_name: String = real_name.to_str()?.into();
                Some((real_name, i))
            })
            .collect()
    }
}

impl<'a, R> FileHandler for &'a mut ZipHandler<R>
where
    R: Seek + Read,
{
    type Reader = zip::read::ZipFile<'a>;
    fn get_file_if_exists(self, name: &str) -> Result<(Option<Self::Reader>, PathBuf)> {
        let p = self.archive_path.join(name);
        match self.index_by_name.get(name) {
            None => Ok((None, p)),
            Some(i) => Ok((Some(self.archive.by_index(*i)?), p)),
        }
    }
    fn source_name(&self) -> &str {
        self.archive_path
            .to_str()
            .unwrap_or_else(|| panic!("the path '{:?}' should be valid UTF-8", self.archive_path))
    }
}

/// Read a vector of objects from a zip in a file_handler
pub fn read_objects<H, O>(
    file_handler: &mut H,
    file_name: &str,
    required_file: bool,
) -> Result<Vec<O>>
where
    for<'a> &'a mut H: FileHandler,
    O: for<'de> serde::Deserialize<'de>,
{
    let (reader, path) = file_handler.get_file_if_exists(file_name)?;
    let file_name = path.file_name();
    let basename = file_name.map_or(path.to_string_lossy(), |b| b.to_string_lossy());

    match (reader, required_file) {
        (None, false) => {
            info!("Skipping {}", basename);
            Ok(vec![])
        }
        (None, true) => {
            bail!("file {:?} not found", path)
        }
        (Some(reader), _) => {
            info!("Reading {}", basename);
            let mut rdr = csv::ReaderBuilder::new()
                .flexible(true)
                .trim(csv::Trim::All)
                .from_reader(reader);
            Ok(rdr
                .deserialize()
                .collect::<Result<_, _>>()
                .with_context(|| format!("Error reading {:?}", path))?)
        }
    }
}

/// Read a vector of objects from a zip in a file_handler ignoring error
pub(crate) fn read_objects_loose<H, O>(
    file_handler: &mut H,
    file_name: &str,
    required_file: bool,
) -> Result<Vec<O>>
where
    for<'a> &'a mut H: FileHandler,
    O: for<'de> serde::Deserialize<'de>,
{
    let (reader, path) = file_handler.get_file_if_exists(file_name)?;
    let file_name = path.file_name();
    let basename = file_name.map_or(path.to_string_lossy(), |b| b.to_string_lossy());

    match (reader, required_file) {
        (None, false) => {
            info!("Skipping {}", basename);
            Ok(vec![])
        }
        (None, true) => {
            bail!("file {:?} not found", path)
        }
        (Some(reader), _) => {
            info!("Reading {}", basename);
            let mut rdr = csv::ReaderBuilder::new()
                .flexible(true)
                .trim(csv::Trim::All)
                .from_reader(reader);
            let objects = rdr
                .deserialize()
                .map(|object| object.with_context(|| format!("Error reading {:?}", path)))
                .skip_error_and_warn()
                .collect();
            Ok(objects)
        }
    }
}

/// Read a CollectionId from a required file in a file_handler
pub fn read_collection<H, O>(file_handler: &mut H, file_name: &str) -> Result<CollectionWithId<O>>
where
    for<'a> &'a mut H: FileHandler,
    O: for<'de> serde::Deserialize<'de> + Id<O>,
{
    let vec = read_objects(file_handler, file_name, true)?;
    CollectionWithId::new(vec).map_err(|e| anyhow!("{}", e))
}

/// Read a CollectionId from a optional file in a file_handler
pub fn read_opt_collection<H, O>(
    file_handler: &mut H,
    file_name: &str,
) -> Result<CollectionWithId<O>>
where
    for<'a> &'a mut H: FileHandler,
    O: for<'de> serde::Deserialize<'de> + Id<O>,
{
    let vec = read_objects(file_handler, file_name, false)?;
    CollectionWithId::new(vec).map_err(|e| anyhow!("{}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::io::Read;

    #[test]
    fn path_file_handler() {
        let mut file_handler = PathFileHandler::new(PathBuf::from("tests/fixtures/file-handler"));

        let (mut hello, _) = file_handler.get_file("hello.txt").unwrap();
        let mut hello_str = String::new();
        hello.read_to_string(&mut hello_str).unwrap();
        assert_eq!("hello\n", hello_str);

        let (mut world, _) = file_handler.get_file("folder/world.txt").unwrap();
        let mut world_str = String::new();
        world.read_to_string(&mut world_str).unwrap();
        assert_eq!("world\n", world_str);
    }

    #[test]
    fn zip_file_handler() {
        let p = "tests/fixtures/file-handler.zip";
        let reader = File::open(p).unwrap();
        let mut file_handler = ZipHandler::new(reader, p).unwrap();

        {
            let (mut hello, _) = file_handler.get_file("hello.txt").unwrap();
            let mut hello_str = String::new();
            hello.read_to_string(&mut hello_str).unwrap();
            assert_eq!("hello\n", hello_str);
        }

        {
            let (mut world, _) = file_handler.get_file("world.txt").unwrap();
            let mut world_str = String::new();
            world.read_to_string(&mut world_str).unwrap();
            assert_eq!("world\n", world_str);
        }
    }
}
