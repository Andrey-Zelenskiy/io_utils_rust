// Copyright Andrey Zelenskiy, 2024-2025

use std::{fmt, fs, io};

use io::Write;

use std::fs::{copy, create_dir_all, OpenOptions};

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Structure to setup a project directory
#[derive(Deserialize)]
pub struct ProjectManager {
    // Path to the project directory
    path: String,
    // Output file extension
    extension: String,
    // Type of behaviour if project files already exist
    overwrite_type: OverwriteType,
}

// Instructions for dealing with files that already exist
#[derive(Deserialize)]
pub enum OverwriteType {
    // Interrupts the program if duplicates are located
    Panic,
    // Copies duplicates to an archive folder
    Archive,
    // Overwrites all files
    Overwrite,
    // Ignore existing file during the writing
    Ignore,
}

impl ProjectManager {
    /// Returns the path to the project directory
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Modifies path of the project
    pub fn set_path(&mut self, path: String) {
        self.path = path;
    }

    /// Initializes output files
    pub fn initialize_output_files(
        &self,
        files: Vec<&mut FileManager>,
    ) -> Result<(), String> {
        files
            .into_iter()
            .map(|file| {
                file.set_project_path(&self.path)
                    .set_extension(&self.extension)
                    .set_path()
            })
            .try_for_each(|file| self.try_initialize_output(file))
    }

    /// Attempts to initialize output files depending on the overwrite
    /// conditions
    fn try_initialize_output(
        &self,
        file: &mut FileManager,
    ) -> Result<(), String> {
        match &self.overwrite_type {
            OverwriteType::Panic => {
                if file.path().exists() {
                    Err(String::from(
                        "Permission denied to overwrite existing output files.",
                    ))
                } else {
                    file.initialize_output();
                    Ok(())
                }
            }
            OverwriteType::Archive => {
                // Create an archive directory
                if file.path().exists() {
                    let archive_path = Path::new(&self.path).join("archive");

                    if !archive_path.exists() {
                        if let Err(reason) = create_dir_all(&archive_path) {
                            panic!(
                                "Unable to create archive directory {:?}: {:?}",
                                archive_path, reason
                            );
                        }
                    }
                    self.move_to_archive(file.path(), &archive_path);
                }

                file.initialize_output();
                Ok(())
            }
            OverwriteType::Overwrite => {
                file.initialize_output();
                Ok(())
            }
            OverwriteType::Ignore => {
                if file.path().exists() {
                    file.change_write_permission(false);
                } else {
                    file.initialize_output();
                }
                Ok(())
            }
        }
    }

    /// Moves a file to archive
    fn move_to_archive(&self, file_path: &Path, archive_path: &Path) {
        let filename = match file_path.file_name() {
            None => panic!(
                "Cannot move file to archive, no file name in path {:?}",
                file_path
            ),
            Some(name) => name,
        };

        let relative_path = match file_path.parent() {
            None => panic!(
                "Cannot move file to archive, no parent directory in path {:?}",
                file_path
            ),
            Some(full_path) => match full_path.file_name() {
                None => panic!(
                "Cannot move file to archive, no output directory in path {:?}",
                file_path
            ),
                Some(path) => path,
            },
        };

        if !archive_path.join(relative_path).exists() {
            if let Err(reason) =
                create_dir_all(archive_path.join(relative_path))
            {
                panic!(
                    "Cannot create {:?} directory: {:?}",
                    archive_path.join(relative_path),
                    reason
                );
            }
        }

        if let Err(reason) =
            copy(file_path, archive_path.join(relative_path).join(filename))
        {
            panic!(
                "Cannot move file {:?} to {:?}: {:?}",
                file_path,
                archive_path.join(relative_path).join(filename),
                reason
            );
        }
    }
}

impl fmt::Display for ProjectManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut summary = format!(
            "Project path is {}\nOutput files have extension .{}\n\
            Existing output files ",
            self.path, self.extension
        );

        match &self.overwrite_type {
            OverwriteType::Panic => summary.push_str(
                "will interrupt the program (overwrite_typ = Panic).\n",
            ),
            OverwriteType::Archive => summary
                .push_str("will be archived (overwrite_type = Archive).\n"),
            OverwriteType::Overwrite => summary.push_str(
                "will be overwritten (overwrite_type = Overwrite).\n",
            ),
            OverwriteType::Ignore => summary.push_str(
                "will not be collected again during this run \
                (overwirte_type = Ignore).\n",
            ),
        }

        write!(f, "{summary}")
    }
}

/// Type for output file manipulation
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct FileManager {
    // Column descriptions in the output file
    header: Option<String>,
    // Project path
    project_path: Option<String>,
    // Output path (relative to the project path)
    output_path: Option<String>,
    // File name
    name: Option<String>,
    // File extension
    extension: Option<String>,
    // Option for a series of data files with related name/structure,
    // stores number of files and current file index
    series: Option<(u32, usize)>,
    // Absolute path of the output file
    #[serde(skip)]
    path: Option<PathBuf>,
    // Permission for writing to the file
    #[serde(skip)]
    writable: bool,
}

impl FileManager {
    // Builder methods

    /// Return the initialization state
    pub fn initialized(&self) -> bool {
        self.path.is_some()
    }

    // Setters
    // Note: the setters only work when self.initialized() = false, and
    // the specified field has not been set yet.For all other cases,
    // one must use Modifiers

    /// Sets the header
    pub fn set_header(&mut self, header: &str) -> &mut Self {
        if !self.initialized() && self.header.is_none() {
            self.header = Some(header.to_string());
        }
        self
    }

    /// Adds a path to the root project directory
    pub fn set_project_path(&mut self, project_path: &str) -> &mut Self {
        if !self.initialized() && self.project_path.is_none() {
            self.project_path = Some(project_path.to_string());
        }
        self
    }

    /// Sets the output directory path (relative to the project directory)
    pub fn set_output_path(&mut self, output_path: &str) -> &mut Self {
        if !self.initialized() && self.output_path.is_none() {
            self.output_path = Some(output_path.to_string());
        }
        self
    }

    /// Sets the name of the output file
    pub fn set_file_name(&mut self, name: &str) -> &mut Self {
        if !self.initialized() && self.name.is_none() {
            self.name = Some(name.to_string());
        }
        self
    }

    /// Sets the extension of the ouput file
    pub fn set_extension(&mut self, extension: &str) -> &mut Self {
        if !self.initialized() && self.extension.is_none() {
            self.extension = Some(extension.to_string());
        }
        self
    }

    /// Sets the number of files in series
    pub fn set_series(&mut self, n_files: u32) -> &mut Self {
        if !self.initialized() && self.series.is_none() {
            self.series = Some((n_files, 0));
        }
        self
    }

    /// Attempts to set the path to the file
    fn set_path(&mut self) -> &mut Self {
        self.path = self.calculate_path();
        self
    }

    /// Attempts to calculate the path to the file
    fn calculate_path(&mut self) -> Option<PathBuf> {
        // To initialize the path, self.project_path, self.output_path,
        // self.name, and self.extension must be set
        match (
            &self.project_path,
            &self.output_path,
            &self.name,
            &self.extension,
        ) {
            (
                Some(project_path),
                Some(output_path),
                Some(name),
                Some(extension),
            ) => {
                let mut path = PathBuf::new();

                path.push(project_path);

                path.push(output_path);

                // If dealing with file series, add the index of the file
                let file_name = match &self.series {
                    Some((_, index)) => &format!("{name}_{index}"),
                    None => name,
                };

                path.push(file_name);

                path.set_extension(extension);

                // Attempt to canonicalize the path
                match path.canonicalize() {
                    Ok(absolute_path) => Some(absolute_path),
                    Err(_) => Some(path),
                }
            }
            _ => None,
        }
    }

    /// Call to build the path
    pub fn build(&mut self) -> Self {
        self.set_path().clone()
    }

    // Modifiers
    // Note: the setters only work when self.initialized() = true.
    // Otherwise, one must use Setters

    /// Changeis the path to the root project directory
    pub fn change_project_path(&mut self, project_path: &str) {
        if self.initialized() {
            self.project_path = Some(project_path.to_string());
            self.set_path();
        }
    }

    /// Changes the output directory path (relative to the project directory)
    pub fn change_output_path(&mut self, output_path: &str) {
        if self.initialized() {
            self.output_path = Some(output_path.to_string());
            self.set_path();
        }
    }

    /// Changes the name of the output file
    pub fn change_file_name(&mut self, name: &str) {
        if self.initialized() {
            self.name = Some(name.to_string());
            self.set_path();
        }
    }

    /// Change the extension of the ouput file - use cautiously!
    pub fn change_extension(&mut self, extension: &str) {
        if self.initialized() {
            self.extension = Some(extension.to_string());
            self.set_path();
        }
    }

    /// Changes the file index if dealing with file series
    pub fn change_file_index(&mut self, index: usize) {
        if self.initialized() {
            if let Some((n_files, _)) = self.series {
                self.series = Some((n_files, index));
                self.set_path();
            }
        }
    }

    /// Changes write permissions
    pub fn change_write_permission(&mut self, writable: bool) {
        self.writable = writable;
    }

    /// Returns the path to the output file
    pub fn path(&self) -> &PathBuf {
        match &self.path {
            Some(path) => path,
            None => panic!(
                "Attempting to access the path of uninitialized FileManager\n\
                Curently,\nproject_path: {:?},\noutput_path: {:?},\
                \nname: {:?},\nextension: {:?},\n",
                self.project_path, self.output_path, self.name, self.extension
            ),
        }
    }

    /// Returns the path to the output file as a string
    pub fn path_string(&self) -> String {
        match self.path().to_str() {
            Some(path_str) => path_str.to_string(),
            None => panic!(
                "Could not convert file path {:?} to string",
                self.path()
            ),
        }
    }

    // Initializer methods

    /// Creates the output file (or files if dealing with series)
    pub fn initialize_output(&mut self) {
        // Make sure that the path is initialized
        if !self.initialized() {
            self.set_path();
        }

        // Create output directory
        match self.path().parent() {
            None => panic!(
                "No parent directory found for {:?} FileManager",
                self.path()
            ),
            Some(path) => {
                if !path.exists() {
                    if let Err(reason) = create_dir_all(path) {
                        panic!(
                            "Cannot initialize output directory {:?}: {:?}",
                            path, reason,
                        );
                    }
                }
            }
        }

        // Initialize file(s)
        match &self.series {
            None => Self::initialize_file(self.path().as_path(), &self.header),
            Some((n_files, _)) => {
                for i in 0..*n_files as usize {
                    self.change_file_index(i);
                    self.set_path();

                    Self::initialize_file(self.path().as_path(), &self.header)
                }
            }
        }

        // Change the writing permissions
        self.writable = true;

        // Attempt to canonicalize the path
        match self.path().canonicalize() {
            Ok(absolute_path) => self.path = Some(absolute_path),
            Err(reason) => {
                println!(
                    "Could not canonicalize path {:?}, using relative \
                        form: {:?}",
                    self.path(),
                    reason.kind()
                );
            }
        }
    }

    /// Helper method for initializing a single (new) file
    fn initialize_file(path: &Path, header: &Option<String>) {
        match OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
        {
            // Write the header
            Ok(mut file) => match header {
                Some(header_str) => {
                    if let Err(reason) = writeln!(file, "{header_str}") {
                        panic!(
                            "Could not write to file {:?}: {:?}",
                            path, reason
                        );
                    }
                }
                None => (),
            },
            Err(reason) => {
                panic!("Could not open file {:?}: {:?}", path, reason)
            }
        }
    }

    // Write methods

    /// Returns write permission of the file manager
    pub fn writable(&self) -> bool {
        self.writable
    }

    /// Opens a file to append the data
    pub fn open_file(&self) -> fs::File {
        if self.writable() {
            match OpenOptions::new().append(true).open(self.path().as_path()) {
                Ok(file) => file,
                Err(reason) => panic!(
                    "Could not open file {:?}: {:?}",
                    self.path(),
                    reason
                ),
            }
        } else {
            panic!(
                "{}",
                io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!(
                        "File {:?} does not have write permissions",
                        self.path()
                    ),
                )
            )
        }
    }

    /// Opens a file in a buffer to append the data (for larger arrays)
    pub fn open_buffer(&self) -> io::BufWriter<fs::File> {
        io::BufWriter::new(self.open_file())
    }
}

#[cfg(test)]
mod tests {
    use std::fs::remove_dir_all;

    use super::*;

    #[test]
    fn overwrite_files() {
        // Setup test project directory tree
        let project_manager = ProjectManager {
            path: "test_overwrite".to_owned(),
            extension: "dat".to_owned(),
            overwrite_type: OverwriteType::Overwrite,
        };

        let mut test_file_1 = FileManager::default()
            .set_output_path("dir_1")
            .set_file_name("file_1")
            .set_extension("dat")
            .build();

        let mut test_file_2 = FileManager::default()
            .set_output_path("dir_1")
            .set_file_name("file_2")
            .set_extension("csv")
            .build();

        let mut test_file_3 = FileManager::default()
            .set_output_path("dir_2")
            .set_file_name("file_3")
            .set_extension("txt")
            .build();

        let mut test_file_4 = FileManager::default()
            .set_output_path("dir_3")
            .set_file_name("file_4")
            .build();

        let files = vec![
            &mut test_file_1,
            &mut test_file_2,
            &mut test_file_3,
            &mut test_file_4,
        ];

        if let Err(reason) = project_manager.initialize_output_files(files) {
            panic!(
                "Could not initialize output files for Overwrite test: \
                {reason}"
            )
        }

        let files = vec![
            &mut test_file_1,
            &mut test_file_2,
            &mut test_file_3,
            &mut test_file_4,
        ];

        if let Err(reason) = project_manager.initialize_output_files(files) {
            panic!(
                "Could not initialize output files for Overwrite test: \
                {reason}"
            )
        }

        // Verify that correct files were initialized
        assert!(
            Path::new("./test_overwrite/dir_1/file_1.dat").exists(),
            "file_1.dat was not created!"
        );

        assert!(
            Path::new("./test_overwrite/dir_1/file_2.csv").exists(),
            "file_2.csv was not created!"
        );

        assert!(
            Path::new("./test_overwrite/dir_2/file_3.txt").exists(),
            "file_3.txt was not created!"
        );

        assert!(
            Path::new("./test_overwrite/dir_3/file_4.dat").exists(),
            "file_4.dat was not created!"
        );

        // Verify final states of the FileManager
        assert_eq!(
            PathBuf::from("./test_overwrite/dir_1/file_1.dat")
                .canonicalize()
                .unwrap(),
            *test_file_1.path(),
            "The path of file_1.dat does not match the expected one."
        );

        assert_eq!(
            PathBuf::from("./test_overwrite/dir_1/file_2.csv")
                .canonicalize()
                .unwrap(),
            *test_file_2.path(),
            "The path of file_2.csv does not match the expected one."
        );

        assert_eq!(
            PathBuf::from("./test_overwrite/dir_2/file_3.txt")
                .canonicalize()
                .unwrap(),
            *test_file_3.path(),
            "The path of file_3.txt does not match the expected one."
        );

        assert_eq!(
            PathBuf::from("./test_overwrite/dir_3/file_4.dat")
                .canonicalize()
                .unwrap(),
            *test_file_4.path(),
            "The path of file_4.dat does not match the expected one."
        );

        // Delete test project directory tree
        if let Err(reason) = remove_dir_all("./test_overwrite/") {
            panic!(
                "Cannot remove project directory ./test_overwrite/: {:?}",
                reason
            );
        }
    }

    #[test]
    fn forbidden_overwrite() {
        // Setup test project directory tree
        let project_manager = ProjectManager {
            path: "test_panic".to_owned(),
            extension: "dat".to_owned(),
            overwrite_type: OverwriteType::Panic,
        };

        let mut test_file_1 = FileManager::default()
            .set_output_path("dir_1")
            .set_file_name("file_1")
            .set_extension("dat")
            .build();

        let mut test_file_2 = FileManager::default()
            .set_output_path("dir_1")
            .set_file_name("file_2")
            .set_extension("csv")
            .build();

        let mut test_file_3 = FileManager::default()
            .set_output_path("dir_2")
            .set_file_name("file_3")
            .set_extension("dat")
            .build();

        let mut test_file_4 = FileManager::default()
            .set_output_path("dir_3")
            .set_file_name("file_4")
            .set_extension("txt")
            .build();

        let files = vec![
            &mut test_file_1,
            &mut test_file_2,
            &mut test_file_3,
            &mut test_file_4,
        ];

        if let Err(reason) = project_manager.initialize_output_files(files) {
            panic!(
                "Could not initialize output files for Panic test: \
                {reason}"
            )
        }

        let mut test_file_1_copy = FileManager::default()
            .set_header("New file_1")
            .set_output_path("dir_1")
            .set_file_name("file_1")
            .set_extension("dat")
            .build();

        let files = vec![&mut test_file_1_copy];

        assert_eq!(
            Err(String::from(
                "Permission denied to overwrite existing output files."
            )),
            project_manager.initialize_output_files(files)
        );

        // Delete test project directory tree
        if let Err(reason) = remove_dir_all("./test_panic/") {
            panic!(
                "Cannot remove project directory ./test_panic/: {:?}",
                reason
            );
        }
    }

    #[test]
    fn archive_files() {
        // Setup test project directory tree
        let project_manager = ProjectManager {
            path: "test_archive".to_owned(),
            extension: "dat".to_owned(),
            overwrite_type: OverwriteType::Archive,
        };

        let mut test_file_1 = FileManager::default()
            .set_output_path("dir_1")
            .set_file_name("file_1")
            .set_extension("dat")
            .build();

        let mut test_file_2 = FileManager::default()
            .set_output_path("dir_1")
            .set_file_name("file_2")
            .set_extension("csv")
            .build();

        let mut test_file_3 = FileManager::default()
            .set_output_path("dir_2")
            .set_file_name("file_3")
            .set_extension("txt")
            .build();

        let mut test_file_4 = FileManager::default()
            .set_output_path("dir_3")
            .set_file_name("file_4")
            .build();

        let files = vec![
            &mut test_file_1,
            &mut test_file_2,
            &mut test_file_3,
            &mut test_file_4,
        ];

        if let Err(reason) = project_manager.initialize_output_files(files) {
            panic!(
                "Could not initialize output files for Archive test: \
                {reason}"
            )
        }

        let mut test_file_1_copy = FileManager::default()
            .set_header("New file_1")
            .set_output_path("dir_1")
            .set_file_name("file_1")
            .build();

        let files = vec![&mut test_file_1_copy];

        if let Err(reason) = project_manager.initialize_output_files(files) {
            panic!(
                "Could not initialize output files for Archive test: \
                {reason}"
            )
        }

        // Verify that correct files were initialized
        assert!(
            Path::new("./test_archive/dir_1/file_1.dat").exists(),
            "file_1.dat was not created!"
        );

        assert!(
            Path::new("./test_archive/dir_1/file_2.csv").exists(),
            "file_2.csv was not created!"
        );

        assert!(
            Path::new("./test_archive/dir_2/file_3.txt").exists(),
            "file_3.txt was not created!"
        );

        assert!(
            Path::new("./test_archive/dir_3/file_4.dat").exists(),
            "file_4.dat was not created!"
        );

        assert!(
            Path::new("./test_archive/archive/dir_1/file_1.dat").exists(),
            "file_1.dat was not created!"
        );

        // Verify final states of the FileManager
        assert_eq!(
            PathBuf::from("./test_archive/dir_1/file_1.dat")
                .canonicalize()
                .unwrap(),
            *test_file_1.path(),
            "The path of file_1.dat does not match the expected one."
        );

        assert_eq!(
            PathBuf::from("./test_archive/dir_1/file_2.csv")
                .canonicalize()
                .unwrap(),
            *test_file_2.path(),
            "The path of file_2.csv does not match the expected one."
        );

        assert_eq!(
            PathBuf::from("./test_archive/dir_2/file_3.txt")
                .canonicalize()
                .unwrap(),
            *test_file_3.path(),
            "The path of file_3.txt does not match the expected one."
        );

        assert_eq!(
            PathBuf::from("./test_archive/dir_3/file_4.dat")
                .canonicalize()
                .unwrap(),
            *test_file_4.path(),
            "The path of file_4.dat does not match the expected one."
        );

        assert_eq!(
            PathBuf::from("./test_archive/dir_1/file_1.dat")
                .canonicalize()
                .unwrap(),
            *test_file_1_copy.path(),
            "The path of file_1.dat does not match the expected one."
        );

        // Delete test project directory tree
        if let Err(reason) = remove_dir_all("./test_archive/") {
            panic!(
                "Cannot remove project directory ./test_archive/: {:?}",
                reason
            );
        }
    }

    #[test]
    fn ignore_files() {
        // Setup test project directory tree
        let project_manager = ProjectManager {
            path: "test_ignore".to_owned(),
            extension: "dat".to_owned(),
            overwrite_type: OverwriteType::Ignore,
        };

        let mut test_file_1 = FileManager::default()
            .set_output_path("dir_1")
            .set_file_name("file_1")
            .set_extension("dat")
            .build();

        let mut test_file_2 = FileManager::default()
            .set_output_path("dir_1")
            .set_file_name("file_2")
            .set_extension("csv")
            .build();

        let mut test_file_3 = FileManager::default()
            .set_output_path("dir_2")
            .set_file_name("file_3")
            .set_extension("txt")
            .build();

        let mut test_file_4 = FileManager::default()
            .set_output_path("dir_3")
            .set_file_name("file_4")
            .build();

        let files = vec![
            &mut test_file_1,
            &mut test_file_2,
            &mut test_file_3,
            &mut test_file_4,
        ];

        if let Err(reason) = project_manager.initialize_output_files(files) {
            panic!(
                "Could not initialize output files for Ignore test: \
                {reason}"
            )
        }

        let mut test_file_1_copy = FileManager::default()
            .set_header("New file_1")
            .set_output_path("dir_1")
            .set_file_name("file_1")
            .set_extension("dat")
            .build();

        let files = vec![&mut test_file_1_copy];

        if let Err(reason) = project_manager.initialize_output_files(files) {
            panic!(
                "Could not initialize output files for Ignore test: \
                {reason}"
            )
        }

        // Verify that correct files were initialized
        assert!(
            Path::new("./test_ignore/dir_1/file_1.dat").exists(),
            "file_1.dat was not created!"
        );

        assert!(
            Path::new("./test_ignore/dir_1/file_2.csv").exists(),
            "file_2.csv was not created!"
        );

        assert!(
            Path::new("./test_ignore/dir_2/file_3.txt").exists(),
            "file_3.txt was not created!"
        );

        assert!(
            Path::new("./test_ignore/dir_3/file_4.dat").exists(),
            "file_4.dat was not created!"
        );

        // Verify final states of the FileManager
        assert_eq!(
            PathBuf::from("./test_ignore/dir_1/file_1.dat")
                .canonicalize()
                .unwrap(),
            *test_file_1.path(),
            "The path of file_1.dat does not match the expected one."
        );

        assert!(!test_file_1_copy.writable(), "{:?}", test_file_1_copy);

        assert_eq!(
            PathBuf::from("./test_ignore/dir_1/file_2.csv")
                .canonicalize()
                .unwrap(),
            *test_file_2.path(),
            "The path of file_2.csv does not match the expected one."
        );

        assert_eq!(
            PathBuf::from("./test_ignore/dir_2/file_3.txt")
                .canonicalize()
                .unwrap(),
            *test_file_3.path(),
            "The path of file_3.txt does not match the expected one."
        );

        assert_eq!(
            PathBuf::from("./test_ignore/dir_3/file_4.dat")
                .canonicalize()
                .unwrap(),
            *test_file_4.path(),
            "The path of file_4.dat does not match the expected one."
        );

        // Delete test project directory tree
        if let Err(reason) = remove_dir_all("./test_ignore/") {
            panic!(
                "Cannot remove project directory ./test_ignore/: {:?}",
                reason
            );
        }
    }

    #[test]
    fn define_builder() {
        let file = FileManager::default()
            .set_header("Some header")
            .set_project_path(".")
            .set_output_path("test")
            .set_file_name("test")
            .set_extension("dat")
            .build();

        assert_eq!(
            FileManager {
                header: Some(String::from("Some header")),
                project_path: Some(String::from(".")),
                output_path: Some(String::from("test")),
                name: Some(String::from("test")),
                extension: Some(String::from("dat")),
                series: None,
                path: Some(PathBuf::from("./test/test.dat")),
                writable: false
            },
            file
        );
    }

    #[test]
    fn file_series() {
        // Setup test project directory tree
        let project_manager = ProjectManager {
            path: "test_series".to_owned(),
            extension: "dat".to_owned(),
            overwrite_type: OverwriteType::Overwrite,
        };

        let mut test_file = FileManager::default()
            .set_output_path("dir")
            .set_file_name("file")
            .set_extension("dat")
            .set_series(10)
            .build();

        if let Err(reason) =
            project_manager.initialize_output_files(vec![&mut test_file])
        {
            panic!(
                "Could not initialize output files for file series test: \
                {reason}"
            )
        }

        for i in 0..10 {
            // Verify that correct files were initialized
            assert!(
                Path::new(&format!("./test_series/dir/file_{i}.dat")).exists(),
                "file_{i}.dat was not created!"
            );
        }

        // Delete test project directory tree
        if let Err(reason) = remove_dir_all("./test_series/") {
            panic!(
                "Cannot remove project directory ./test_series/: {:?}",
                reason
            );
        }
    }
}
